use crate::macos::compact::*;
use crate::Registers;
use gimli::{
    BaseAddresses, EhFrame, EhFrameOffset, EndianSlice, NativeEndian, UnwindContext, UnwindSection, UnwindTable, X86_64,
};
use std::slice;

pub fn step(
    registers: &mut Registers,
    info: UnwindFuncInfo,
    sections: DyldUnwindSections,
    ctx: &mut UnwindContext<EndianSlice<'static, NativeEndian>>,
) -> bool {
    match info.encoding & UNWIND_X86_64_MODE_MASK {
        UNWIND_X86_64_MODE_RBP_FRAME => step_frame(registers, info.encoding),
        UNWIND_X86_64_MODE_STACK_IMMD => step_frameless(registers, info, true),
        UNWIND_X86_64_MODE_STACK_IND => step_frameless(registers, info, false),
        UNWIND_X86_64_MODE_DWARF => return step_dwarf(registers, info.encoding, sections, ctx),
        _ => unreachable!(),
    }
    true
}

fn step_frame(registers: &mut Registers, encoding: Encoding) {
    // UNWIND_X86_64_RBP_FRAME_OFFSET = 0000_0000_1111_1111_0000_0000_0000_0000 (0x00FF0000)
    //                                            |-------|
    let saved_registers_offset = (encoding >> 16) & 0b1111_1111;
    // UNWIND_X86_64_RBP_FRAME_REGISTERS = 0000_0000_0000_0000_0111_1111_1111_1111 (0x00007FFF)
    //                                                          |----------------|
    let mut saved_registers_locations = encoding & 0b111_1111_1111_1111;

    // restore registers
    let mut saved_registers = registers[X86_64::RBP] - (8 * saved_registers_offset) as u64;
    for _ in 0..5 {
        unsafe {
            match saved_registers_locations & 0x7 {
                UNWIND_X86_64_REG_NONE => {} // no register saved in this slot
                UNWIND_X86_64_REG_RBX => registers[X86_64::RBX] = *(saved_registers as *const u64),
                UNWIND_X86_64_REG_R12 => registers[X86_64::R12] = *(saved_registers as *const u64),
                UNWIND_X86_64_REG_R13 => registers[X86_64::R13] = *(saved_registers as *const u64),
                UNWIND_X86_64_REG_R14 => registers[X86_64::R14] = *(saved_registers as *const u64),
                UNWIND_X86_64_REG_R15 => registers[X86_64::R15] = *(saved_registers as *const u64),
                UNWIND_X86_64_REG_RBP | _ => unreachable!(),
            }
        }
        saved_registers += 8;
        saved_registers_locations = saved_registers_locations >> 3;
    }

    // frame unwind
    unsafe {
        let rbp = registers[X86_64::RBP];
        registers[X86_64::RBP] = *(rbp as *const u64);
        registers[X86_64::RSP] = rbp + 16;
        registers[X86_64::RA] = *((rbp + 8) as *const u64)
    }
}

fn step_frameless(registers: &mut Registers, info: UnwindFuncInfo, imm: bool) {
    // UNWIND_X86_64_FRAMELESS_STACK_SIZE = 0000_0000_1111_1111_0000_0000_0000_0000 (0x00FF0000)
    //                                                |-------|
    let stack_size_encoded = (info.encoding >> 16) & 0b1111_1111;
    // UNWIND_X86_64_FRAMELESS_STACK_ADJUST = 0000_0000_0000_0000_1110_0000_0000_0000 (0x0000E000)
    //                                                            |-|
    let stack_adjust = (info.encoding >> 13) & 0b111;
    // UNWIND_X86_64_FRAMELESS_STACK_REG_COUNT = 0000_0000_0000_0000_0001_1100_0000_0000 (0x00001C00)
    //                                                                  |--|
    let reg_count = (info.encoding >> 10) & 0b111;
    // UNWIND_X86_64_FRAMELESS_STACK_REG_PERMUTATION = 0000_0000_0000_0000_0000_0011_1111_1111 (0x000003FF)
    //                                                                            |----------|
    let mut permutation = info.encoding & 0b11_1111_1111;

    // calculate stack size
    let stack_size = if imm {
        stack_size_encoded * 8
    } else {
        // stack size is encoded in subl $xxx,%esp instruction
        let subl = unsafe { *((info.start + stack_size_encoded as usize) as *const u32) };
        subl + 8 * stack_adjust
    };

    // decompress permutation
    let mut regs = [0u32; 6];
    match reg_count {
        6 => {
            regs[0] = permutation / 120;
            permutation -= regs[0] * 120;
            regs[1] = permutation / 24;
            permutation -= regs[1] * 24;
            regs[2] = permutation / 6;
            permutation -= regs[2] * 6;
            regs[3] = permutation / 2;
            permutation -= regs[3] * 2;
            regs[4] = permutation;
            regs[5] = 0;
        }
        5 => {
            regs[0] = permutation / 120;
            permutation -= regs[0] * 120;
            regs[1] = permutation / 24;
            permutation -= regs[1] * 24;
            regs[2] = permutation / 6;
            permutation -= regs[2] * 6;
            regs[3] = permutation / 2;
            permutation -= regs[3] * 2;
            regs[4] = permutation;
        }
        4 => {
            regs[0] = permutation / 60;
            permutation -= regs[0] * 60;
            regs[1] = permutation / 12;
            permutation -= regs[1] * 12;
            regs[2] = permutation / 3;
            permutation -= regs[2] * 3;
            regs[3] = permutation;
        }
        3 => {
            regs[0] = permutation / 20;
            permutation -= regs[0] * 20;
            regs[1] = permutation / 4;
            permutation -= regs[1] * 4;
            regs[2] = permutation;
        }
        2 => {
            regs[0] = permutation / 5;
            permutation -= regs[0] * 5;
            regs[1] = permutation;
        }
        1 => {
            regs[0] = permutation;
        }
        _ => {}
    }

    // re-number registers back to standard numbers
    let mut register_saved = [0; 6];
    let mut used = [false; 7];
    for n in 0..(reg_count as usize) {
        let mut reg_num = 0;
        for u in 1..7 {
            if !used[u] {
                if reg_num == regs[n] {
                    register_saved[n] = u as u32;
                    used[u] = true;
                    break;
                }
                reg_num += 1;
            }
        }
    }

    // restore registers
    let mut saved_registers = registers[X86_64::RSP] + stack_size as u64 - 8 - 8 * reg_count as u64;
    for n in 0..(reg_count as usize) {
        unsafe {
            match register_saved[n] {
                UNWIND_X86_64_REG_RBX => registers[X86_64::RBX] = *(saved_registers as *const u64),
                UNWIND_X86_64_REG_R12 => registers[X86_64::R12] = *(saved_registers as *const u64),
                UNWIND_X86_64_REG_R13 => registers[X86_64::R13] = *(saved_registers as *const u64),
                UNWIND_X86_64_REG_R14 => registers[X86_64::R14] = *(saved_registers as *const u64),
                UNWIND_X86_64_REG_R15 => registers[X86_64::R15] = *(saved_registers as *const u64),
                UNWIND_X86_64_REG_RBP => registers[X86_64::RBP] = *(saved_registers as *const u64),
                _ => unreachable!(),
            }
        }
        saved_registers += 8;
    }

    // frameless unwind
    unsafe {
        // return address is on stack after last saved register
        registers[X86_64::RA] = *(saved_registers as *const u64);
        // old rsp is before return address
        registers[X86_64::RSP] = *((saved_registers + 8) as *const u64);
    }
}

fn step_dwarf(
    registers: &mut Registers,
    encoding: Encoding,
    sections: DyldUnwindSections,
    ctx: &mut UnwindContext<EndianSlice<'static, NativeEndian>>,
) -> bool {
    if sections.dwarf_section == 0 || sections.dwarf_section_length == 0 {
        return false;
    }
    let section_data =
        unsafe { slice::from_raw_parts(sections.dwarf_section as *const u8, sections.dwarf_section_length as _) };
    let eh_frame = EhFrame::new(section_data, NativeEndian);
    let offset = sections.dwarf_section as usize + (encoding & UNWIND_X86_64_DWARF_SECTION_OFFSET) as usize;
    let base_address = BaseAddresses::default().set_eh_frame(sections.dwarf_section);
    let fde = match eh_frame.fde_from_offset(&base_address, EhFrameOffset::from(offset), EhFrame::cie_from_offset) {
        Ok(fde) => fde,
        Err(_) => return false,
    };
    let mut unwind_tab = match UnwindTable::new(&eh_frame, &base_address, ctx, &fde) {
        Ok(tab) => tab,
        Err(_) => return false,
    };
    while let Some(row) = match unwind_tab.next_row() {
        Ok(v) => v,
        Err(_) => return false,
    } {
        if row.contains(registers[X86_64::RA]) {
            return crate::dwarf::step(registers, row);
        }
    }
    false
}
