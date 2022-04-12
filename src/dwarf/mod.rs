use crate::dyld::SectionInfo;
#[cfg(target_arch = "aarch64")]
use crate::registers::UNW_ARM64_RA_SIGN_STATE;
use crate::registers::{Registers, UNW_REG_IP, UNW_REG_SP};
use crate::utils::{address_is_readable, load};
use cfi::{CommonInformationEntry, FrameDescriptionEntry};
use header::EhFrameHeader;
use instruction::{get_saved_float_register, get_saved_register, get_saved_vector_register, RegisterSavedWhere};

mod cfi;
mod consts;
mod encoding;
mod expression;
mod header;
mod instruction;

#[derive(thiserror::Error, Debug, Copy, Clone)]
pub enum DwarfError {
    #[error("invalid .eh_frame_hdr version: {0}")]
    InvalidHeaderVersion(u8),

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

    #[error("invalid cfa register number: {0}")]
    InvalidCfaRegisterNumber(usize),

    #[error("invalid return address register number: {0}")]
    InvalidReturnAddressRegisterNumber(usize),

    #[error("invalid instruction: {0}")]
    InvalidInstruction(u8),

    #[error("invalid expression: {0}")]
    InvalidExpression(u8),

    #[error("invalid expression deref size: {0}")]
    InvalidExpressionDerefSize(u8),

    #[error("invalid expression register number: {0}")]
    InvalidExpressionRegisterNumber(u32),

    #[error("invalid pointer encoding offset: {0}")]
    InvalidPointerEncodingOffset(u8),

    #[error("invalid pointer encoding value: {0}")]
    InvalidPointerEncodingValue(u8),

    #[error("invalid pointer encoding size: {0}")]
    InvalidPointerEncodingSize(u8),

    #[error("invalid datarel_base")]
    InvalidDataRelBase,

    #[error("invalid register location")]
    InvalidRegisterLocation,

    #[error("no remember state")]
    NoRememberState,

    #[error("unreadable address: {0:#x}")]
    UnreadableAddress(u64),

    #[error("unimplemented ra sign state")]
    UnimplementedRaSignState,

    #[error("malformed uleb128 expression at: {0:#x}")]
    MalformedUleb128Expression(u64),

    #[error("truncated uleb128 expression at: {0:#x}")]
    TruncatedUleb128Expression(u64),

    #[error("truncated uleb128 expression at: {0:#x}")]
    TruncatedSleb128Expression(u64),

    #[error("no way to calculate cfa")]
    NoWayToCalculateCfa,
}

pub fn step(pc: u64, section: &SectionInfo, registers: &mut Registers) -> Result<(), DwarfError> {
    // Search FDE & CIE for target PC.
    let (fde, cie) = search_fde(pc, section)?;

    // Run instructions to calculate PrologInfo from FDE.
    let info = instruction::run(pc, &fde, &cie)?;

    // Get pointer to cfa (architecture specific).
    let cfa = info.cfa(registers)?;

    // Restore registers that DWARF says were saved.
    let mut new_registers = *registers;

    // Typically, the CFA is the stack pointer at the call site in
    // the previous frame. However, there are scenarios in which this is not
    // true. For example, if we switched to a new stack. In that case, the
    // value of the previous SP might be indicated by a CFI directive.
    //
    // We set the SP here to the CFA, allowing for it to be overridden
    // by a CFI directive later on.
    new_registers[UNW_REG_SP] = cfa;

    let mut return_address = 0;
    for n in 0..=Registers::max_register_num() {
        if info.saved_registers[n].location != RegisterSavedWhere::Unused {
            if Registers::valid_float_register(n) {
                new_registers.set_float_register(n, get_saved_float_register(registers, info.saved_registers[n], cfa)?);
            } else if Registers::valid_vector_register(n) {
                new_registers
                    .set_vector_register(n, get_saved_vector_register(registers, info.saved_registers[n], cfa)?);
            } else if n == cie.return_address_register as usize {
                return_address = get_saved_register(registers, info.saved_registers[n], cfa)?;
            } else if Registers::valid_register(n) {
                new_registers[n] = get_saved_register(registers, info.saved_registers[n], cfa)?;
            } else {
                return Err(DwarfError::InvalidRegisterNumber(n));
            }
        } else if n == cie.return_address_register as usize {
            // Leaf function keeps the return address in register and there is no
            // explicit instructions how to restore it.
            if !Registers::valid_register(cie.return_address_register as usize) {
                return Err(DwarfError::InvalidReturnAddressRegisterNumber(
                    cie.return_address_register as usize,
                ));
            }
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
            // TODO: implement
            return Err(DwarfError::UnimplementedRaSignState);
        }
    }

    // Return address is address after call site instruction, so setting IP to
    // that does simulates a return.
    new_registers[UNW_REG_IP] = return_address;

    // Simulate the step by replacing the register set with the new ones.
    *registers = new_registers;
    Ok(())
}

fn search_fde(pc: u64, s: &SectionInfo) -> Result<(FrameDescriptionEntry, CommonInformationEntry), DwarfError> {
    let end = s.eh_frame_hdr + s.eh_frame_hdr_len;
    let header = EhFrameHeader::decode(s.eh_frame_hdr, end)?;
    match header.search(pc) {
        Ok(v) => Ok(v),
        Err(DwarfError::FDENotFound) => cfi::scan(header.eh_frame, u64::MAX, pc),
        Err(err) => Err(err),
    }
}

#[inline]
fn load_with_protect<T: Copy>(address: u64) -> Result<T, DwarfError> {
    if address_is_readable(address) {
        Ok(load(address))
    } else {
        Err(DwarfError::UnreadableAddress(address))
    }
}
