#![cfg_attr(all(target_os = "macos", target_arch = "aarch64"), allow(dead_code))]

use crate::Registers;
use gimli::{CfaRule, Reader, RegisterRule, UnwindTableRow};

pub fn step<R: Reader>(registers: &mut Registers, row: &UnwindTableRow<R>) -> bool {
    let cfa = match *row.cfa() {
        CfaRule::RegisterAndOffset { register, offset } => registers[register].wrapping_add(offset as _),
        CfaRule::Expression(_) => return false,
    };
    let mut new_registers = registers.clone();
    new_registers.set_pc(0);
    for &(register, ref rule) in row.registers() {
        new_registers[register] = match *rule {
            RegisterRule::Undefined => return false,
            RegisterRule::SameValue => registers[register],
            RegisterRule::Offset(n) => unsafe { *(cfa.wrapping_add(n as _) as *const u64) },
            RegisterRule::ValOffset(n) => cfa.wrapping_add(n as _),
            RegisterRule::Register(r) => registers[r],
            RegisterRule::Expression(_) => return false,
            RegisterRule::ValExpression(_) => return false,
            RegisterRule::Architectural => return false,
        };
    }
    new_registers.set_sp(cfa);
    *registers = new_registers;
    true
}
