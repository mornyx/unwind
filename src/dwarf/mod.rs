use crate::registers::{Registers, UNW_ARM64_MAX_REG_NUM, UNW_ARM64_RA_SIGN_STATE};
use instruction::{get_saved_float_register, get_saved_register, get_saved_vector_register, RegisterSavedWhere};

pub use cfi::*;
pub use header::EhFrameHeader;

mod cfi;
mod encoding;
mod expression;
mod header;
mod instruction;

#[derive(thiserror::Error, Debug, Copy, Clone)]
pub enum DwarfError {
    #[error("invalid .eh_frame_hdr version: {0}")]
    HeaderInvalidVersion(u8),

    #[error("cie zero length")]
    CIEZeroLength,

    #[error("cie id is not zero")]
    CIEIdIsNotZero,

    #[error("invalid cie version: {0}")]
    CIEInvalidVersion(u8),

    #[error("fde not found")]
    FDENotFound,

    #[error("zero fde length")]
    FDEZeroLength,

    #[error("fde is really cie")]
    FDEIsReallyCIE,

    #[error("invalid register number: {0}")]
    InvalidRegisterNumber(usize),

    #[error("invalid opcode: {0}")]
    InvalidOpcode(u8),

    #[error("no remember state")]
    NoRememberState,
}

pub fn step(
    pc: u64,
    fde: &FrameDescriptionEntry,
    cie: &CommonInformationEntry,
    registers: &mut Registers,
) -> Result<(), DwarfError> {
    // Run instructions to calculate PrologInfo from FDE.
    let info = instruction::run(pc, fde, cie)?;

    // Get pointer to cfa (architecture specific).
    let cfa = info.cfa(registers);

    // Restore registers that DWARF says were saved.
    let mut new_registers = *registers;

    // Typically, the CFA is the stack pointer at the call site in
    // the previous frame. However, there are scenarios in which this is not
    // true. For example, if we switched to a new stack. In that case, the
    // value of the previous SP might be indicated by a CFI directive.
    //
    // We set the SP here to the CFA, allowing for it to be overridden
    // by a CFI directive later on.
    new_registers.set_sp(cfa);

    let mut return_address = 0;
    for n in 0..=UNW_ARM64_MAX_REG_NUM {
        if info.saved_registers[n].location != RegisterSavedWhere::Unused {
            if registers.valid_float_register(n) {
                new_registers.set_float_register(n, get_saved_float_register(registers, info.saved_registers[n], cfa));
            } else if registers.valid_vector_register(n) {
                new_registers
                    .set_vector_register(n, get_saved_vector_register(registers, info.saved_registers[n], cfa));
            } else if n == cie.return_address_register as usize {
                return_address = get_saved_register(registers, info.saved_registers[n], cfa);
            } else if registers.valid_register(n) {
                new_registers[n] = get_saved_register(registers, info.saved_registers[n], cfa);
            } else {
                return Err(DwarfError::InvalidRegisterNumber(n));
            }
        } else if n == cie.return_address_register as usize {
            // Leaf function keeps the return address in register and there is no
            // explicit instructions how to restore it.
            return_address = registers[cie.return_address_register as usize];
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        // If the target is aarch64 then the return address may have been signed
        // using the v8.3 pointer authentication extensions. The original
        // return address needs to be authenticated before the return address is
        // restored. autia1716 is used instead of autia as autia1716 assembles
        // to a NOP on pre-v8.3a architectures.
        if info.saved_registers[UNW_ARM64_RA_SIGN_STATE].value != 0 && return_address != 0 {
            unimplemented!(); // TODO: implement
        }
    }

    // Return address is address after call site instruction, so setting IP to
    // that does simulates a return.
    new_registers.set_pc(return_address);

    // Simulate the step by replacing the register set with the new ones.
    *registers = new_registers;
    Ok(())
}
