use crate::compact::*;
use crate::registers::{UNW_ARM64_FP, UNW_ARM64_LR, UNW_REG_IP, UNW_REG_SP};
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
    restore_registers(registers, encoding, registers[UNW_ARM64_FP] - 8);
    let fp = registers[UNW_ARM64_FP];
    // fp points to old fp
    registers[UNW_ARM64_FP] = load::<u64>(fp);
    // old sp is fp less saved fp and lr
    registers[UNW_REG_SP] = fp + 16;
    // pop return address into pc
    registers[UNW_REG_IP] = load::<u64>(fp + 8);
}

fn step_frameless(registers: &mut Registers, encoding: Encoding) {
    let stack_size = 16 * ((encoding >> 12) & 0xFFF) as u64;
    let loc = restore_registers(registers, encoding, registers[UNW_REG_SP] + stack_size);
    // subtract stack size off of sp
    registers[UNW_REG_SP] = loc;
    // set pc to be value in lr
    registers[UNW_REG_IP] = registers[UNW_ARM64_LR];
}

fn step_dwarf(_registers: &mut Registers, _encoding: Encoding, _sections: DyldUnwindSections) -> bool {
    false
}

fn restore_registers(registers: &mut Registers, encoding: Encoding, mut loc: u64) -> u64 {
    if encoding & UNWIND_ARM64_FRAME_X19_X20_PAIR != 0 {
        registers[19] = load::<u64>(loc);
        loc -= 8;
        registers[20] = load::<u64>(loc);
        loc -= 8;
    }
    if encoding & UNWIND_ARM64_FRAME_X21_X22_PAIR != 0 {
        registers[21] = load::<u64>(loc);
        loc -= 8;
        registers[22] = load::<u64>(loc);
        loc -= 8;
    }
    if encoding & UNWIND_ARM64_FRAME_X23_X24_PAIR != 0 {
        registers[23] = load::<u64>(loc);
        loc -= 8;
        registers[24] = load::<u64>(loc);
        loc -= 8;
    }
    if encoding & UNWIND_ARM64_FRAME_X25_X26_PAIR != 0 {
        registers[25] = load::<u64>(loc);
        loc -= 8;
        registers[26] = load::<u64>(loc);
        loc -= 8;
    }
    if encoding & UNWIND_ARM64_FRAME_X27_X28_PAIR != 0 {
        registers[27] = load::<u64>(loc);
        loc -= 8;
        registers[28] = load::<u64>(loc);
        loc -= 8;
    }
    if encoding & UNWIND_ARM64_FRAME_D8_D9_PAIR != 0 {
        registers.set_float_register(8, load::<f64>(loc));
        loc -= 8;
        registers.set_float_register(9, load::<f64>(loc));
        loc -= 8;
    }
    if encoding & UNWIND_ARM64_FRAME_D10_D11_PAIR != 0 {
        registers.set_float_register(10, load::<f64>(loc));
        loc -= 8;
        registers.set_float_register(11, load::<f64>(loc));
        loc -= 8;
    }
    if encoding & UNWIND_ARM64_FRAME_D12_D13_PAIR != 0 {
        registers.set_float_register(12, load::<f64>(loc));
        loc -= 8;
        registers.set_float_register(13, load::<f64>(loc));
        loc -= 8;
    }
    if encoding & UNWIND_ARM64_FRAME_D14_D15_PAIR != 0 {
        registers.set_float_register(14, load::<f64>(loc));
        loc -= 8;
        registers.set_float_register(15, load::<f64>(loc));
        loc -= 8;
    }
    loc
}
