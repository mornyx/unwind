use crate::dyld::DyldUnwindSections;
use crate::utils::load;
use std::{mem, slice};

#[cfg(target_arch = "x86_64")]
mod x64;
#[cfg(target_arch = "x86_64")]
pub use x64::*;

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "aarch64")]
pub use aarch64::*;

#[derive(Debug, Default)]
pub struct UnwindFuncInfo {
    pub start: u64,         // start address of function
    pub end: u64,           // address after end of function
    pub encoding: Encoding, // compact unwind encoding, or zero if none
}

impl UnwindFuncInfo {
    pub fn find(pc: u64, section_address: u64, base_address: u64) -> Option<Self> {
        let header = unsafe { mem::transmute::<_, &UnwindInfoSectionHeader>(section_address) };
        if header.version != UNWIND_SECTION_VERSION {
            // Err: invalid version of section header
            return None;
        }

        let indexes: &[UnwindInfoSectionHeaderIndexEntry] = unsafe {
            slice::from_raw_parts(
                (section_address + header.index_section_offset as u64) as _,
                header.index_count as _,
            )
        };

        // Do a binary search of top level index to find page with unwind info.
        let target_function_offset = (pc - base_address) as u32;
        let mut low = 0;
        let mut high = header.index_count as usize;
        let last = high - 1;
        while low < high {
            let mid = (low + high) / 2;
            if indexes[mid].function_offset <= target_function_offset {
                if mid == last || indexes[mid + 1].function_offset > target_function_offset {
                    low = mid;
                    break;
                } else {
                    low = mid + 1;
                }
            } else {
                high = mid;
            }
        }

        // TODO: Find corresponding logic in llvm/libunwind.
        //       Or describe it.
        if low == last {
            // Err: index not found
            return None;
        }

        let l1_function_offset = indexes[low].function_offset;
        let l1_next_page_function_offset = indexes[low + 1].function_offset as u64;
        let l2_address = section_address + indexes[low].second_level_pages_section_offset as u64;
        let l2_kind = load::<u32>(l2_address);
        if l2_kind == UNWIND_SECOND_LEVEL_REGULAR {
            let l2_header = unsafe { mem::transmute::<_, &UnwindInfoRegularSecondLevelPageHeader>(l2_address) };
            let l2_indexes: &[UnwindInfoRegularSecondLevelEntry] = unsafe {
                slice::from_raw_parts(
                    (l2_address + l2_header.entry_page_offset as u64) as _,
                    l2_header.entry_count as _,
                )
            };
            // Binary search looks for entry with e where index[e].offset <= pc < index[e+1].offset
            let mut func_end = 0;
            let mut low = 0;
            let mut high = l2_header.entry_count as usize;
            while low < high {
                let mid = (low + high) / 2;
                if l2_indexes[mid].function_offset <= target_function_offset {
                    if mid == l2_header.entry_count as usize - 1 {
                        // at end of table
                        low = mid;
                        func_end = base_address + l1_next_page_function_offset;
                        break;
                    } else if l2_indexes[mid + 1].function_offset > target_function_offset {
                        // next is too big, so we found it
                        low = mid;
                        func_end = base_address + l2_indexes[low + 1].function_offset as u64;
                        break;
                    } else {
                        low = mid + 1;
                    }
                } else {
                    high = mid;
                }
            }
            let encoding = l2_indexes[low].encoding;
            let func_start = base_address + l2_indexes[low].function_offset as u64;
            if pc < func_start || pc > func_end {
                // Err: invalid func range
                return None;
            }
            Some(UnwindFuncInfo {
                start: func_start,
                end: func_end,
                encoding,
            })
        } else if l2_kind == UNWIND_SECOND_LEVEL_COMPRESSED {
            let l2_header = unsafe { mem::transmute::<_, &UnwindInfoCompressedSecondLevelPageHeader>(l2_address) };
            let l2_indexes: &[u32] = unsafe {
                slice::from_raw_parts(
                    (l2_address + l2_header.entry_page_offset as u64) as _,
                    l2_header.entry_count as _,
                )
            };
            let target_function_page_offset = target_function_offset - l1_function_offset;
            let mut low = 0;
            let mut high = l2_header.entry_count as usize;
            let last = high - 1;
            while low < high {
                let mid = (low + high) / 2;
                if unwind_info_compressed_entry_func_offset(l2_indexes[mid]) <= target_function_page_offset {
                    if mid == last
                        || unwind_info_compressed_entry_func_offset(l2_indexes[mid + 1]) > target_function_page_offset
                    {
                        low = mid;
                        break;
                    } else {
                        low = mid + 1;
                    }
                } else {
                    high = mid;
                }
            }
            let func_start = base_address
                + l1_function_offset as u64
                + unwind_info_compressed_entry_func_offset(l2_indexes[low]) as u64;
            let func_end = if low < last {
                base_address
                    + l1_function_offset as u64
                    + unwind_info_compressed_entry_func_offset(l2_indexes[low + 1]) as u64
            } else {
                base_address + l1_next_page_function_offset
            };
            if pc < func_start || pc > func_end {
                // Err: invalid func range
                return None;
            }
            let encoding_index = unwind_info_compressed_entry_encoding_index(l2_indexes[low]) as u32;
            let encoding = if encoding_index < header.common_encodings_array_count {
                // encoding is in common table in section header
                unsafe {
                    let encodings_address = section_address + header.common_encodings_array_section_offset as u64;
                    let encodings_ptr = encodings_address as *const Encoding;
                    let encodings_len = header.common_encodings_array_count as usize;
                    let encodings = slice::from_raw_parts(encodings_ptr, encodings_len);
                    encodings[encoding_index as usize]
                }
            } else {
                // encoding is in page specific table
                unsafe {
                    let encodings_address = l2_address + l2_header.encodings_page_offset as u64;
                    let encodings_ptr = encodings_address as *const Encoding;
                    let encodings_len = l2_header.encodings_count as usize;
                    let encodings = slice::from_raw_parts(encodings_ptr, encodings_len);
                    encodings[(encoding_index - header.common_encodings_array_count) as usize]
                }
            };
            Some(UnwindFuncInfo {
                start: func_start,
                end: func_end,
                encoding,
            })
        } else {
            // Err: invalid second level header kind
            None
        }
    }
}

//===----------------------------------------------------------------------===//
//
// Part of the LLVM Project, under the Apache License v2.0 with LLVM Exceptions.
// See https://llvm.org/LICENSE.txt for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
//
//
// Darwin's alternative to DWARF based unwind encodings.
//
//===----------------------------------------------------------------------===//

// Compilers can emit standard DWARF FDEs in the __TEXT,__eh_frame section
// of object files. Or compilers can emit compact unwind information in
// the __LD,__compact_unwind section.
//
// When the linker creates a final linked image, it will create a
// __TEXT,__unwind_info section.  This section is a small and fast way for the
// runtime to access unwind info for any given function.  If the compiler
// emitted compact unwind info for the function, that compact unwind info will
// be encoded in the __TEXT,__unwind_info section. If the compiler emitted
// DWARF unwind info, the __TEXT,__unwind_info section will contain the offset
// of the FDE in the __TEXT,__eh_frame section in the final linked image.
//
// Note: Previously, the linker would transform some DWARF unwind infos into
//       compact unwind info.  But that is fragile and no longer done.

// x86_64
//
// 1-bit: start
// 1-bit: has lsda
// 2-bit: personality index
//
// 4-bits: 0=old, 1=rbp based, 2=stack-imm, 3=stack-ind, 4=DWARF
//  rbp based:
//        15-bits (5*3-bits per reg) register permutation
//        8-bits for stack offset
//  frameless:
//        8-bits stack size
//        3-bits stack adjust
//        3-bits register count
//        10-bits register permutation
//
// For x86_64 there are four modes for the compact unwind encoding:
// UNWIND_X86_64_MODE_RBP_FRAME:
//    RBP based frame where RBP is push on stack immediately after return address,
//    then RSP is moved to RBP. Thus, to unwind RSP is restored with the current
//    EPB value, then RBP is restored by popping off the stack, and the return
//    is done by popping the stack once more into the pc.
//    All non-volatile registers that need to be restored must have been saved
//    in a small range in the stack that starts RBP-8 to RBP-2040.  The offset/8
//    is encoded in the UNWIND_X86_64_RBP_FRAME_OFFSET bits.  The registers saved
//    are encoded in the UNWIND_X86_64_RBP_FRAME_REGISTERS bits as five 3-bit entries.
//    Each entry contains which register to restore.
// UNWIND_X86_64_MODE_STACK_IMMD:
//    A "frameless" (RBP not used as frame pointer) function with a small
//    constant stack size.  To return, a constant (encoded in the compact
//    unwind encoding) is added to the RSP. Then the return is done by
//    popping the stack into the pc.
//    All non-volatile registers that need to be restored must have been saved
//    on the stack immediately after the return address.  The stack_size/8 is
//    encoded in the UNWIND_X86_64_FRAMELESS_STACK_SIZE (max stack size is 2048).
//    The number of registers saved is encoded in UNWIND_X86_64_FRAMELESS_STACK_REG_COUNT.
//    UNWIND_X86_64_FRAMELESS_STACK_REG_PERMUTATION constains which registers were
//    saved and their order.
// UNWIND_X86_64_MODE_STACK_IND:
//    A "frameless" (RBP not used as frame pointer) function large constant
//    stack size.  This case is like the previous, except the stack size is too
//    large to encode in the compact unwind encoding.  Instead it requires that
//    the function contains "subq $nnnnnnnn,RSP" in its prolog.  The compact
//    encoding contains the offset to the nnnnnnnn value in the function in
//    UNWIND_X86_64_FRAMELESS_STACK_SIZE.
// UNWIND_X86_64_MODE_DWARF:
//    No compact unwind encoding is available.  Instead the low 24-bits of the
//    compact encoding is the offset of the DWARF FDE in the __eh_frame section.
//    This mode is never used in object files.  It is only generated by the
//    linker in final linked images which have only DWARF unwind info for a
//    function.
pub const UNWIND_X86_64_MODE_MASK: u32 = 0x0F000000;
pub const UNWIND_X86_64_MODE_RBP_FRAME: u32 = 0x01000000;
pub const UNWIND_X86_64_MODE_STACK_IMMD: u32 = 0x02000000;
pub const UNWIND_X86_64_MODE_STACK_IND: u32 = 0x03000000;
pub const UNWIND_X86_64_MODE_DWARF: u32 = 0x04000000;
pub const UNWIND_X86_64_RBP_FRAME_REGISTERS: u32 = 0x00007FFF;
pub const UNWIND_X86_64_RBP_FRAME_OFFSET: u32 = 0x00FF0000;
pub const UNWIND_X86_64_FRAMELESS_STACK_SIZE: u32 = 0x00FF0000;
pub const UNWIND_X86_64_FRAMELESS_STACK_ADJUST: u32 = 0x0000E000;
pub const UNWIND_X86_64_FRAMELESS_STACK_REG_COUNT: u32 = 0x00001C00;
pub const UNWIND_X86_64_FRAMELESS_STACK_REG_PERMUTATION: u32 = 0x000003FF;
pub const UNWIND_X86_64_DWARF_SECTION_OFFSET: u32 = 0x00FFFFFF;
pub const UNWIND_X86_64_REG_NONE: u32 = 0;
pub const UNWIND_X86_64_REG_RBX: u32 = 1;
pub const UNWIND_X86_64_REG_R12: u32 = 2;
pub const UNWIND_X86_64_REG_R13: u32 = 3;
pub const UNWIND_X86_64_REG_R14: u32 = 4;
pub const UNWIND_X86_64_REG_R15: u32 = 5;
pub const UNWIND_X86_64_REG_RBP: u32 = 6;

// ARM64
//
// 1-bit: start
// 1-bit: has lsda
// 2-bit: personality index
//
// 4-bits: 4=frame-based, 3=DWARF, 2=frameless
//  frameless:
//        12-bits of stack size
//  frame-based:
//        4-bits D reg pairs saved
//        5-bits X reg pairs saved
//  DWARF:
//        24-bits offset of DWARF FDE in __eh_frame section
//
// For arm64 there are three modes for the compact unwind encoding:
// UNWIND_ARM64_MODE_FRAME:
//    This is a standard arm64 prolog where FP/LR are immediately pushed on the
//    stack, then SP is copied to FP. If there are any non-volatile registers
//    saved, then are copied into the stack frame in pairs in a contiguous
//    range right below the saved FP/LR pair.  Any subset of the five X pairs
//    and four D pairs can be saved, but the memory layout must be in register
//    number order.
// UNWIND_ARM64_MODE_FRAMELESS:
//    A "frameless" leaf function, where FP/LR are not saved. The return address
//    remains in LR throughout the function. If any non-volatile registers
//    are saved, they must be pushed onto the stack before any stack space is
//    allocated for local variables.  The stack sized (including any saved
//    non-volatile registers) divided by 16 is encoded in the bits
//    UNWIND_ARM64_FRAMELESS_STACK_SIZE_MASK.
// UNWIND_ARM64_MODE_DWARF:
//    No compact unwind encoding is available.  Instead the low 24-bits of the
//    compact encoding is the offset of the DWARF FDE in the __eh_frame section.
//    This mode is never used in object files.  It is only generated by the
//    linker in final linked images which have only DWARF unwind info for a
//    function.
pub const UNWIND_ARM64_MODE_MASK: u32 = 0x0F000000;
pub const UNWIND_ARM64_MODE_FRAMELESS: u32 = 0x02000000;
pub const UNWIND_ARM64_MODE_DWARF: u32 = 0x03000000;
pub const UNWIND_ARM64_MODE_FRAME: u32 = 0x04000000;
pub const UNWIND_ARM64_FRAME_X19_X20_PAIR: u32 = 0x00000001;
pub const UNWIND_ARM64_FRAME_X21_X22_PAIR: u32 = 0x00000002;
pub const UNWIND_ARM64_FRAME_X23_X24_PAIR: u32 = 0x00000004;
pub const UNWIND_ARM64_FRAME_X25_X26_PAIR: u32 = 0x00000008;
pub const UNWIND_ARM64_FRAME_X27_X28_PAIR: u32 = 0x00000010;
pub const UNWIND_ARM64_FRAME_D8_D9_PAIR: u32 = 0x00000100;
pub const UNWIND_ARM64_FRAME_D10_D11_PAIR: u32 = 0x00000200;
pub const UNWIND_ARM64_FRAME_D12_D13_PAIR: u32 = 0x00000400;
pub const UNWIND_ARM64_FRAME_D14_D15_PAIR: u32 = 0x00000800;
pub const UNWIND_ARM64_FRAMELESS_STACK_SIZE_MASK: u32 = 0x00FFF000;
pub const UNWIND_ARM64_DWARF_SECTION_OFFSET: u32 = 0x00FFFFFF;

// architecture independent bits
const UNWIND_IS_NOT_FUNCTION_START: u32 = 0x80000000;
const UNWIND_HAS_LSDA: u32 = 0x40000000;
const UNWIND_PERSONALITY_MASK: u32 = 0x30000000;
const UNWIND_SECTION_VERSION: u32 = 1;
const UNWIND_SECOND_LEVEL_REGULAR: u32 = 2;
const UNWIND_SECOND_LEVEL_COMPRESSED: u32 = 3;

// The compact unwind encoding is a 32-bit value which encoded in an
// architecture specific way, which registers to restore from where, and how
// to unwind out of the function.
pub type Encoding = u32;

// The __TEXT,__unwind_info section is laid out for an efficient two level lookup.
// The header of the section contains a coarse index that maps function address
// to the page (4096 byte block) containing the unwind info for that function.

#[repr(C)]
#[derive(Debug)]
struct UnwindInfoSectionHeader {
    version: u32, // UNWIND_SECTION_VERSION
    common_encodings_array_section_offset: u32,
    common_encodings_array_count: u32,
    personality_array_section_offset: u32,
    personality_array_count: u32,
    index_section_offset: u32,
    index_count: u32,
    // compact_unwind_encoding_t[]
    // uint32_t personalities[]
    // unwind_info_section_header_index_entry[]
    // unwind_info_section_header_lsda_index_entry[]
}

#[repr(C)]
#[derive(Debug)]
struct UnwindInfoSectionHeaderIndexEntry {
    function_offset: u32,
    second_level_pages_section_offset: u32, // section offset to start of regular or compress page
    lsda_index_array_section_offset: u32,   // section offset to start of lsda_index array for this range
}

#[repr(C)]
#[derive(Debug)]
struct UnwindInfoSectionHeaderLsdaIndexEntry {
    function_offset: u32,
    lsda_offset: u32,
}

// There are two kinds of second level index pages: regular and compressed.
// A compressed page can hold up to 1021 entries, but it cannot be used
// if too many different encoding types are used.  The regular page holds
// 511 entries.

#[repr(C)]
#[derive(Debug)]
struct UnwindInfoRegularSecondLevelEntry {
    function_offset: u32,
    encoding: Encoding,
}

#[repr(C)]
#[derive(Debug)]
struct UnwindInfoRegularSecondLevelPageHeader {
    kind: u32, // UNWIND_SECOND_LEVEL_REGULAR
    entry_page_offset: u16,
    entry_count: u16,
    // entry array
}

#[repr(C)]
#[derive(Debug)]
struct UnwindInfoCompressedSecondLevelPageHeader {
    kind: u32, // UNWIND_SECOND_LEVEL_COMPRESSED
    entry_page_offset: u16,
    entry_count: u16,
    encodings_page_offset: u16,
    encodings_count: u16,
    // 32-bit entry array
    // encodings array
}

#[inline]
fn unwind_info_compressed_entry_func_offset(entry: u32) -> u32 {
    entry & 0x00FFFFFF
}

#[inline]
fn unwind_info_compressed_entry_encoding_index(entry: u32) -> u16 {
    ((entry >> 24) as u16) & 0xFF
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registers::{unwind_init_registers, Registers};

    #[test]
    fn test_find_unwind_func_info() {
        let mut registers = Registers::default();
        unsafe {
            unwind_init_registers(&mut registers as _);
        }
        let sections = DyldUnwindSections::find(registers.pc()).unwrap();
        let info = UnwindFuncInfo::find(registers.pc(), sections.compact_unwind_section, sections.mach_header);
        assert!(info.is_some());
        let info = info.unwrap();
        assert_ne!(info.start, 0);
        assert_ne!(info.end, 0);
        assert_ne!(info.encoding, 0);
    }
}
