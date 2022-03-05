#![allow(dead_code)]

use crate::macos::compact::*;
use crate::Registers;
use gimli::{Reader, UnwindContext};

pub fn step<R: Reader>(
    registers: &mut Registers,
    info: UnwindFuncInfo,
    sections: DyldUnwindSections,
    ctx: &mut UnwindContext<R>,
) -> bool {
    match info.encoding & UNWIND_ARM64_MODE_MASK {
        UNWIND_ARM64_MODE_FRAME => step_frame(registers, info.encoding),
        UNWIND_ARM64_MODE_FRAMELESS => step_frameless(registers, info.encoding),
        UNWIND_ARM64_MODE_DWARF => return step_dwarf(registers, info.encoding, sections, ctx),
        _ => unreachable!(),
    }
    true
}

fn step_frame(registers: &mut Registers, encoding: Encoding) {
    restore_registers(registers, encoding, registers.fp - 8);
    let fp = registers.fp;
    unsafe {
        // fp points to old fp
        registers.fp = *(fp as *const u64);
        // old sp is fp less saved fp and lr
        registers.sp = fp + 16;
        // pop return address into pc
        registers.pc = *((fp + 8) as *const u64);
    }
}

fn step_frameless(registers: &mut Registers, encoding: Encoding) {
    let stack_size = 16 * ((encoding >> 12) & 0xFFF) as u64;
    let loc = restore_registers(registers, encoding, registers.sp + stack_size);
    // subtract stack size off of sp
    registers.sp = loc;
    // set pc to be value in lr
    registers.pc = registers.lr;
}

fn step_dwarf<R: Reader>(
    _registers: &mut Registers,
    _encoding: Encoding,
    _sections: DyldUnwindSections,
    _ctx: &mut UnwindContext<R>,
) -> bool {
    false
}

fn restore_registers(registers: &mut Registers, encoding: Encoding, mut loc: u64) -> u64 {
    unsafe {
        if encoding & UNWIND_ARM64_FRAME_X19_X20_PAIR != 0 {
            registers.x[19] = *(loc as *const u64);
            loc -= 8;
            registers.x[20] = *(loc as *const u64);
            loc -= 8;
        }
        if encoding & UNWIND_ARM64_FRAME_X21_X22_PAIR != 0 {
            registers.x[21] = *(loc as *const u64);
            loc -= 8;
            registers.x[22] = *(loc as *const u64);
            loc -= 8;
        }
        if encoding & UNWIND_ARM64_FRAME_X23_X24_PAIR != 0 {
            registers.x[23] = *(loc as *const u64);
            loc -= 8;
            registers.x[24] = *(loc as *const u64);
            loc -= 8;
        }
        if encoding & UNWIND_ARM64_FRAME_X25_X26_PAIR != 0 {
            registers.x[25] = *(loc as *const u64);
            loc -= 8;
            registers.x[26] = *(loc as *const u64);
            loc -= 8;
        }
        if encoding & UNWIND_ARM64_FRAME_X27_X28_PAIR != 0 {
            registers.x[27] = *(loc as *const u64);
            loc -= 8;
            registers.x[28] = *(loc as *const u64);
            loc -= 8;
        }
        if encoding & UNWIND_ARM64_FRAME_D8_D9_PAIR != 0 {
            registers.d[8] = *(loc as *const f64);
            loc -= 8;
            registers.d[9] = *(loc as *const f64);
            loc -= 8;
        }
        if encoding & UNWIND_ARM64_FRAME_D10_D11_PAIR != 0 {
            registers.d[10] = *(loc as *const f64);
            loc -= 8;
            registers.d[11] = *(loc as *const f64);
            loc -= 8;
        }
        if encoding & UNWIND_ARM64_FRAME_D12_D13_PAIR != 0 {
            registers.d[12] = *(loc as *const f64);
            loc -= 8;
            registers.d[13] = *(loc as *const f64);
            loc -= 8;
        }
        if encoding & UNWIND_ARM64_FRAME_D14_D15_PAIR != 0 {
            registers.d[14] = *(loc as *const f64);
            loc -= 8;
            registers.d[15] = *(loc as *const f64);
            loc -= 8;
        }
        loc
    }
}
