use crate::dwarf::cfi::{CommonInformationEntry, FrameDescriptionEntry};
use crate::dwarf::encoding::{decode_pointer, decode_sleb128, decode_uleb128};
use crate::dwarf::expression::evaluate;
use crate::dwarf::DwarfError;
use crate::registers::Registers;
use crate::registers::UNW_ARM64_RA_SIGN_STATE;
use crate::utils::load;

// These DW_* constants were taken from version 3 of the DWARF standard,
// which is Copyright (c) 2005 Free Standards Group, and
// Copyright (c) 1992, 1993 UNIX International, Inc.
//
// DWARF unwind instructions.
const DW_CFA_NOP: u8 = 0x0;
const DW_CFA_SET_LOC: u8 = 0x1;
const DW_CFA_ADVANCE_LOC1: u8 = 0x2;
const DW_CFA_ADVANCE_LOC2: u8 = 0x3;
const DW_CFA_ADVANCE_LOC4: u8 = 0x4;
const DW_CFA_OFFSET_EXTENDED: u8 = 0x5;
const DW_CFA_RESTORE_EXTENDED: u8 = 0x6;
const DW_CFA_UNDEFINED: u8 = 0x7;
const DW_CFA_SAME_VALUE: u8 = 0x8;
const DW_CFA_REGISTER: u8 = 0x9;
const DW_CFA_REMEMBER_STATE: u8 = 0xA;
const DW_CFA_RESTORE_STATE: u8 = 0xB;
const DW_CFA_DEF_CFA: u8 = 0xC;
const DW_CFA_DEF_CFA_REGISTER: u8 = 0xD;
const DW_CFA_DEF_CFA_OFFSET: u8 = 0xE;
const DW_CFA_DEF_CFA_EXPRESSION: u8 = 0xF;
const DW_CFA_EXPRESSION: u8 = 0x10;
const DW_CFA_OFFSET_EXTENDED_SF: u8 = 0x11;
const DW_CFA_DEF_CFA_SF: u8 = 0x12;
const DW_CFA_DEF_CFA_OFFSET_SF: u8 = 0x13;
const DW_CFA_VAL_OFFSET: u8 = 0x14;
const DW_CFA_VAL_OFFSET_SF: u8 = 0x15;
const DW_CFA_VAL_EXPRESSION: u8 = 0x16;
const DW_CFA_ADVANCE_LOC: u8 = 0x40; // high 2 bits are 0x1, lower 6 bits are delta
const DW_CFA_OFFSET: u8 = 0x80; // high 2 bits are 0x2, lower 6 bits are register
const DW_CFA_RESTORE: u8 = 0xC0; // high 2 bits are 0x3, lower 6 bits are register
const _DW_CFA_GNU_WINDOW_SAVE: u8 = 0x2D; // GNU extensions
const DW_CFA_GNU_ARGS_SIZE: u8 = 0x2E;
const DW_CFA_GNU_NEGATIVE_OFFSET_EXTENDED: u8 = 0x2F;
const DW_CFA_AARCH64_NEGATE_RA_STATE: u8 = 0x2D; // AARCH64 extensions

const MAX_REGISTER_NUM: usize = 287;

/// "Run" the DWARF instructions and create the abstract [PrologInfo].
pub fn run(pc: u64, fde: &FrameDescriptionEntry, cie: &CommonInformationEntry) -> Result<PrologInfo, DwarfError> {
    let mut result = PrologInfo::default();
    run_(
        &mut result,
        cie,
        cie.cie_instructions,
        cie.cie_start + cie.cie_length,
        u64::MAX,
    )?;
    run_(
        &mut result,
        cie,
        fde.fde_instructions,
        fde.fde_start + fde.fde_length,
        pc - fde.pc_start,
    )?;
    Ok(result)
}

/// Information about a frame layout and registers saved determined
/// by "running" the DWARF FDE "instructions".
#[derive(Debug, Copy, Clone)]
pub struct PrologInfo {
    pub cfa_register: u32,
    pub cfa_register_offset: i32, // CFA = (cfa_register) + cfa_register_offset
    pub cfa_expression: i64,      // CFA = expression
    pub sp_extra_arg_size: u32,
    pub saved_registers: [RegisterLocation; MAX_REGISTER_NUM],
}

impl Default for PrologInfo {
    fn default() -> Self {
        Self {
            cfa_register: 0,
            cfa_register_offset: 0,
            cfa_expression: 0,
            sp_extra_arg_size: 0,
            saved_registers: [RegisterLocation {
                location: RegisterSavedWhere::Unused,
                initial_state_saved: false,
                value: 0,
            }; MAX_REGISTER_NUM],
        }
    }
}

impl PrologInfo {
    pub fn cfa(&self, registers: &Registers) -> u64 {
        if self.cfa_register != 0 {
            (registers[self.cfa_register as usize] as i64 + self.cfa_register_offset as i64) as u64
        } else if self.cfa_expression != 0 {
            evaluate(self.cfa_expression as u64, registers, 0)
        } else {
            unreachable!()
        }
    }

    pub fn set_register(&mut self, r: usize, new_loc: RegisterSavedWhere, new_v: i64, initial_state: &mut PrologInfo) {
        self.check_save_register(r, initial_state);
        self.saved_registers[r].location = new_loc;
        self.saved_registers[r].value = new_v;
    }

    pub fn set_register_location(&mut self, r: usize, new_loc: RegisterSavedWhere, initial_state: &mut PrologInfo) {
        self.check_save_register(r, initial_state);
        self.saved_registers[r].location = new_loc;
    }

    pub fn set_register_value(&mut self, r: usize, new_v: i64, initial_state: &mut PrologInfo) {
        self.check_save_register(r, initial_state);
        self.saved_registers[r].value = new_v;
    }

    pub fn restore_register_to_initial_state(&mut self, r: usize, initial_state: &PrologInfo) {
        if self.saved_registers[r].initial_state_saved {
            self.saved_registers[r] = initial_state.saved_registers[r];
        }
        // Else the register still holds its initial state.
    }

    fn check_save_register(&mut self, r: usize, initial_state: &mut PrologInfo) {
        if !self.saved_registers[r].initial_state_saved {
            initial_state.saved_registers[r] = self.saved_registers[r];
            self.saved_registers[r].initial_state_saved = true;
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RegisterLocation {
    pub location: RegisterSavedWhere,
    pub initial_state_saved: bool,
    pub value: i64,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum RegisterSavedWhere {
    Unused,
    Undefined,
    InCFA,
    OffsetFromCFA,
    InRegister,
    AtExpression,
    IsExpression,
}

pub fn get_saved_register(registers: &Registers, loc: RegisterLocation, cfa: u64) -> u64 {
    match loc.location {
        RegisterSavedWhere::InCFA => load::<u64>((cfa as i64 + loc.value) as u64),
        RegisterSavedWhere::AtExpression => load::<u64>(evaluate(loc.value as u64, registers, cfa)),
        RegisterSavedWhere::IsExpression => evaluate(loc.value as u64, registers, cfa),
        RegisterSavedWhere::InRegister => load::<u64>(loc.value as u64),
        RegisterSavedWhere::Undefined => 0,
        _ => unreachable!(),
    }
}

pub fn get_saved_float_register(registers: &Registers, loc: RegisterLocation, cfa: u64) -> f64 {
    match loc.location {
        RegisterSavedWhere::InCFA => load::<f64>((cfa as i64 + loc.value) as u64),
        RegisterSavedWhere::AtExpression => load::<f64>(evaluate(loc.value as u64, registers, cfa)),
        _ => unreachable!(),
    }
}

pub fn get_saved_vector_register(registers: &Registers, loc: RegisterLocation, cfa: u64) -> u128 {
    match loc.location {
        RegisterSavedWhere::InCFA => load::<u128>((cfa as i64 + loc.value) as u64),
        RegisterSavedWhere::AtExpression => load::<u128>(evaluate(loc.value as u64, registers, cfa)),
        _ => unreachable!(),
    }
}

struct RememberStack {
    info: PrologInfo,
    next: *const RememberStack,
}

fn run_(
    result: &mut PrologInfo,
    cie: &CommonInformationEntry,
    start: u64,
    end: u64,
    pc_offset: u64,
) -> Result<(), DwarfError> {
    let mut loc = start;
    let mut code_offset = 0;
    let mut initial_state = PrologInfo::default();
    let mut remember_stack: *const RememberStack = std::ptr::null();

    // See DWARF Spec, section 6.4.2 for details on unwind opcodes.
    while loc < end && code_offset < pc_offset {
        let opcode = load::<u8>(loc);
        loc += 1;

        match opcode {
            DW_CFA_NOP => {}
            DW_CFA_SET_LOC => {
                code_offset = decode_pointer(&mut loc, end, cie.pointer_encoding, 0);
            }
            DW_CFA_ADVANCE_LOC1 => {
                code_offset += load::<u8>(loc) as u64 * cie.code_align_factor as u64;
                loc += 1;
            }
            DW_CFA_ADVANCE_LOC2 => {
                code_offset += load::<u16>(loc) as u64 * cie.code_align_factor as u64;
                loc += 2;
            }
            DW_CFA_ADVANCE_LOC4 => {
                code_offset += load::<u32>(loc) as u64 * cie.code_align_factor as u64;
                loc += 4;
            }
            DW_CFA_OFFSET_EXTENDED => {
                let r = decode_uleb128(&mut loc, end) as usize;
                if r > MAX_REGISTER_NUM {
                    return Err(DwarfError::InvalidRegisterNumber(r));
                }
                let offset = decode_uleb128(&mut loc, end) as i64 * cie.data_align_factor as i64;
                result.set_register(r, RegisterSavedWhere::InCFA, offset, &mut initial_state);
            }
            DW_CFA_RESTORE_EXTENDED => {
                let r = decode_uleb128(&mut loc, end) as usize;
                if r > MAX_REGISTER_NUM {
                    return Err(DwarfError::InvalidRegisterNumber(r));
                }
                result.restore_register_to_initial_state(r, &mut initial_state);
            }
            DW_CFA_UNDEFINED => {
                let r = decode_uleb128(&mut loc, end) as usize;
                if r > MAX_REGISTER_NUM {
                    return Err(DwarfError::InvalidRegisterNumber(r));
                }
                result.set_register_location(r, RegisterSavedWhere::Undefined, &mut initial_state);
            }
            DW_CFA_SAME_VALUE => {
                let r = decode_uleb128(&mut loc, end) as usize;
                if r > MAX_REGISTER_NUM {
                    return Err(DwarfError::InvalidRegisterNumber(r));
                }
                // "same value" means register was stored in frame, but its current
                // value has not changed, so no need to restore from frame.
                // We model this as if the register was never saved.
                result.set_register_location(r, RegisterSavedWhere::Unused, &mut initial_state);
            }
            DW_CFA_REGISTER => {
                let r1 = decode_uleb128(&mut loc, end) as usize;
                if r1 > MAX_REGISTER_NUM {
                    return Err(DwarfError::InvalidRegisterNumber(r1));
                }
                let r2 = decode_uleb128(&mut loc, end) as usize;
                if r2 > MAX_REGISTER_NUM {
                    return Err(DwarfError::InvalidRegisterNumber(r2));
                }
                result.set_register(r1, RegisterSavedWhere::InRegister, r2 as i64, &mut initial_state);
            }
            DW_CFA_REMEMBER_STATE => {
                // Avoid malloc because it needs heap allocation.
                remember_stack = &RememberStack {
                    info: *result,
                    next: remember_stack,
                } as _;
            }
            DW_CFA_RESTORE_STATE => {
                if remember_stack == std::ptr::null() {
                    return Err(DwarfError::NoRememberState);
                }
                unsafe {
                    *result = (*remember_stack).info;
                    remember_stack = (*remember_stack).next;
                }
            }
            DW_CFA_DEF_CFA => {
                let r = decode_uleb128(&mut loc, end) as usize;
                if r > MAX_REGISTER_NUM {
                    return Err(DwarfError::InvalidRegisterNumber(r));
                }
                result.cfa_register = r as u32;
                result.cfa_register_offset = decode_uleb128(&mut loc, end) as i32;
            }
            DW_CFA_DEF_CFA_REGISTER => {
                let r = decode_uleb128(&mut loc, end) as usize;
                if r > MAX_REGISTER_NUM {
                    return Err(DwarfError::InvalidRegisterNumber(r));
                }
                result.cfa_register = r as u32;
            }
            DW_CFA_DEF_CFA_OFFSET => {
                result.cfa_register_offset = decode_uleb128(&mut loc, end) as i32;
            }
            DW_CFA_DEF_CFA_EXPRESSION => {
                result.cfa_register = 0;
                result.cfa_expression = loc as i64;
                loc += decode_uleb128(&mut loc, end);
            }
            DW_CFA_EXPRESSION => {
                let r = decode_uleb128(&mut loc, end) as usize;
                if r > MAX_REGISTER_NUM {
                    return Err(DwarfError::InvalidRegisterNumber(r));
                }
                result.set_register(r, RegisterSavedWhere::AtExpression, loc as i64, &mut initial_state);
                loc += decode_uleb128(&mut loc, end);
            }
            DW_CFA_OFFSET_EXTENDED_SF => {
                let r = decode_uleb128(&mut loc, end) as usize;
                if r > MAX_REGISTER_NUM {
                    return Err(DwarfError::InvalidRegisterNumber(r));
                }
                let offset = decode_sleb128(&mut loc, end) * cie.data_align_factor as i64;
                result.set_register(r, RegisterSavedWhere::InCFA, offset, &mut initial_state);
            }
            DW_CFA_DEF_CFA_SF => {
                let r = decode_uleb128(&mut loc, end) as usize;
                if r > MAX_REGISTER_NUM {
                    return Err(DwarfError::InvalidRegisterNumber(r));
                }
                result.cfa_register = r as u32;
                result.cfa_register_offset = (decode_sleb128(&mut loc, end) * cie.data_align_factor as i64) as i32;
            }
            DW_CFA_DEF_CFA_OFFSET_SF => {
                result.cfa_register_offset = (decode_sleb128(&mut loc, end) * cie.data_align_factor as i64) as i32;
            }
            DW_CFA_VAL_OFFSET => {
                let r = decode_uleb128(&mut loc, end) as usize;
                if r > MAX_REGISTER_NUM {
                    return Err(DwarfError::InvalidRegisterNumber(r));
                }
                let offset = decode_uleb128(&mut loc, end) as i64 * cie.data_align_factor as i64;
                result.set_register(r, RegisterSavedWhere::OffsetFromCFA, offset, &mut initial_state);
            }
            DW_CFA_VAL_OFFSET_SF => {
                let r = decode_uleb128(&mut loc, end) as usize;
                if r > MAX_REGISTER_NUM {
                    return Err(DwarfError::InvalidRegisterNumber(r));
                }
                let offset = decode_sleb128(&mut loc, end) * cie.data_align_factor as i64;
                result.set_register(r, RegisterSavedWhere::OffsetFromCFA, offset, &mut initial_state);
            }
            DW_CFA_VAL_EXPRESSION => {
                let r = decode_uleb128(&mut loc, end) as usize;
                if r > MAX_REGISTER_NUM {
                    return Err(DwarfError::InvalidRegisterNumber(r));
                }
                result.set_register(r, RegisterSavedWhere::IsExpression, loc as i64, &mut initial_state);
                loc += decode_uleb128(&mut loc, end);
            }
            DW_CFA_GNU_ARGS_SIZE => {
                result.sp_extra_arg_size = decode_uleb128(&mut loc, end) as u32;
            }
            DW_CFA_GNU_NEGATIVE_OFFSET_EXTENDED => {
                let r = decode_uleb128(&mut loc, end) as usize;
                if r > MAX_REGISTER_NUM {
                    return Err(DwarfError::InvalidRegisterNumber(r));
                }
                let offset = decode_uleb128(&mut loc, end) as i64 * cie.data_align_factor as i64;
                result.set_register(r, RegisterSavedWhere::InCFA, -offset, &mut initial_state);
            }
            #[cfg(target_arch = "aarch64")]
            DW_CFA_AARCH64_NEGATE_RA_STATE => {
                let value = result.saved_registers[UNW_ARM64_RA_SIGN_STATE].value ^ 0x1;
                result.set_register_value(UNW_ARM64_RA_SIGN_STATE, value, &mut initial_state);
            }
            _ => {
                let operand = opcode & 0b111111;
                match opcode & 0b11000000 {
                    DW_CFA_OFFSET => {
                        let r = operand as usize;
                        if r > MAX_REGISTER_NUM {
                            return Err(DwarfError::InvalidRegisterNumber(r));
                        }
                        let offset = decode_uleb128(&mut loc, end) as i64 * cie.data_align_factor as i64;
                        result.set_register(r, RegisterSavedWhere::InCFA, offset, &mut initial_state);
                    }
                    DW_CFA_ADVANCE_LOC => {
                        code_offset += operand as u64 * cie.code_align_factor as u64;
                    }
                    DW_CFA_RESTORE => {
                        let r = operand as usize;
                        if r > MAX_REGISTER_NUM {
                            return Err(DwarfError::InvalidRegisterNumber(r));
                        }
                        result.restore_register_to_initial_state(r, &initial_state);
                    }
                    _ => return Err(DwarfError::InvalidOpcode(opcode)),
                }
            }
        }
    }
    Ok(())
}
