#![allow(dead_code)]

use crate::compact::*;
use crate::utils::load;
use crate::Registers;

/// Restore `Registers` based on the rules in the `UnwindFuncInfo.encoding`.
pub fn step(registers: &mut Registers, info: UnwindFuncInfo, sections: DyldUnwindSections) -> bool {
    match info.encoding & UNWIND_ARM64_MODE_MASK {
        UNWIND_ARM64_MODE_FRAME => step_frame(registers, info.encoding),
        UNWIND_ARM64_MODE_FRAMELESS => step_frameless(registers, info.encoding),
        UNWIND_ARM64_MODE_DWARF => return step_dwarf(registers, info.encoding, sections),
        _ => unreachable!(),
    }
    true
}

fn step_frame(registers: &mut Registers, encoding: Encoding) {
    restore_registers(registers, encoding, registers.fp - 8);
    let fp = registers.fp;
    // fp points to old fp
    registers.fp = load::<u64>(fp);
    // old sp is fp less saved fp and lr
    registers.sp = fp + 16;
    // pop return address into pc
    registers.pc = load::<u64>(fp + 8);
}

fn step_frameless(registers: &mut Registers, encoding: Encoding) {
    let stack_size = 16 * ((encoding >> 12) & 0xFFF) as u64;
    let loc = restore_registers(registers, encoding, registers.sp + stack_size);
    // subtract stack size off of sp
    registers.sp = loc;
    // set pc to be value in lr
    registers.pc = registers.lr;
}

fn step_dwarf(_registers: &mut Registers, _encoding: Encoding, _sections: DyldUnwindSections) -> bool {
    false
}

fn restore_registers(registers: &mut Registers, encoding: Encoding, mut loc: u64) -> u64 {
    if encoding & UNWIND_ARM64_FRAME_X19_X20_PAIR != 0 {
        registers.x[19] = load::<u64>(loc);
        loc -= 8;
        registers.x[20] = load::<u64>(loc);
        loc -= 8;
    }
    if encoding & UNWIND_ARM64_FRAME_X21_X22_PAIR != 0 {
        registers.x[21] = load::<u64>(loc);
        loc -= 8;
        registers.x[22] = load::<u64>(loc);
        loc -= 8;
    }
    if encoding & UNWIND_ARM64_FRAME_X23_X24_PAIR != 0 {
        registers.x[23] = load::<u64>(loc);
        loc -= 8;
        registers.x[24] = load::<u64>(loc);
        loc -= 8;
    }
    if encoding & UNWIND_ARM64_FRAME_X25_X26_PAIR != 0 {
        registers.x[25] = load::<u64>(loc);
        loc -= 8;
        registers.x[26] = load::<u64>(loc);
        loc -= 8;
    }
    if encoding & UNWIND_ARM64_FRAME_X27_X28_PAIR != 0 {
        registers.x[27] = load::<u64>(loc);
        loc -= 8;
        registers.x[28] = load::<u64>(loc);
        loc -= 8;
    }
    if encoding & UNWIND_ARM64_FRAME_D8_D9_PAIR != 0 {
        registers.d[8] = load::<f64>(loc);
        loc -= 8;
        registers.d[9] = load::<f64>(loc);
        loc -= 8;
    }
    if encoding & UNWIND_ARM64_FRAME_D10_D11_PAIR != 0 {
        registers.d[10] = load::<f64>(loc);
        loc -= 8;
        registers.d[11] = load::<f64>(loc);
        loc -= 8;
    }
    if encoding & UNWIND_ARM64_FRAME_D12_D13_PAIR != 0 {
        registers.d[12] = load::<f64>(loc);
        loc -= 8;
        registers.d[13] = load::<f64>(loc);
        loc -= 8;
    }
    if encoding & UNWIND_ARM64_FRAME_D14_D15_PAIR != 0 {
        registers.d[14] = load::<f64>(loc);
        loc -= 8;
        registers.d[15] = load::<f64>(loc);
        loc -= 8;
    }
    loc
}
