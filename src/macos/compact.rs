use crate::UnwindContext;
use std::mem;
use std::slice;

#[derive(Debug)]
pub struct UnwindCursor {
    context: UnwindContext,
    func_info: UnwindFuncInfo,
    unwind_info_missing: bool,
}

impl UnwindCursor {
    pub fn new(context: UnwindContext) -> Self {
        let mut cursor = Self {
            context,
            func_info: UnwindFuncInfo::default(),
            unwind_info_missing: false,
        };
        cursor.update_func_info_based_on_pc(false);
        cursor
    }

    pub fn step(&mut self) -> Option<&UnwindContext> {
        // Bottom of stack is defined is when unwind info cannot be found.
        if self.unwind_info_missing {
            return None;
        }

        // Use unwinding info to modify register set as if function returned.
        let encoding = self.func_info.encoding;
        let mode = encoding & UNWIND_ARM64_MODE_MASK;
        match mode {
            UNWIND_ARM64_MODE_FRAME => self.step_with_frame(encoding),
            UNWIND_ARM64_MODE_FRAMELESS => self.step_with_frameless(encoding),
            _ => {
                // Err: invalid arm64 mode
                return None;
            }
        };

        // Update info based on new pc.
        self.update_func_info_based_on_pc(true);
        if self.unwind_info_missing {
            return None;
        }
        Some(&self.context)
    }

    fn step_with_frame(&mut self, encoding: Encoding) {
        self.restore_registers(encoding, self.context.fp - 8);
        let fp = self.context.fp;
        // fp points to old fp
        self.context.fp = unsafe { *mem::transmute::<_, *const u64>(fp) };
        // old sp is fp less saved fp and lr
        self.context.sp = fp + 16;
        // pop return address into pc
        self.context.pc = unsafe { *mem::transmute::<_, *const u64>(fp + 8) };
    }

    fn step_with_frameless(&mut self, encoding: Encoding) {
        let stack_size = 16 * ((encoding >> 12) & 0xFFF) as u64;
        let loc = self.restore_registers(encoding, self.context.sp + stack_size);
        // subtract stack size off of sp
        self.context.sp = loc;
        // set pc to be value in lr
        self.context.pc = self.context.lr;
    }

    fn restore_registers(&mut self, encoding: Encoding, mut loc: u64) -> u64 {
        let context = &mut self.context;
        if encoding & UNWIND_ARM64_FRAME_X19_X20_PAIR != 0 {
            context.x[19] = unsafe { *mem::transmute::<_, *const u64>(loc) };
            loc -= 8;
            context.x[20] = unsafe { *mem::transmute::<_, *const u64>(loc) };
            loc -= 8;
        }
        if encoding & UNWIND_ARM64_FRAME_X21_X22_PAIR != 0 {
            context.x[21] = unsafe { *mem::transmute::<_, *const u64>(loc) };
            loc -= 8;
            context.x[22] = unsafe { *mem::transmute::<_, *const u64>(loc) };
            loc -= 8;
        }
        if encoding & UNWIND_ARM64_FRAME_X23_X24_PAIR != 0 {
            context.x[23] = unsafe { *mem::transmute::<_, *const u64>(loc) };
            loc -= 8;
            context.x[24] = unsafe { *mem::transmute::<_, *const u64>(loc) };
            loc -= 8;
        }
        if encoding & UNWIND_ARM64_FRAME_X25_X26_PAIR != 0 {
            context.x[25] = unsafe { *mem::transmute::<_, *const u64>(loc) };
            loc -= 8;
            context.x[26] = unsafe { *mem::transmute::<_, *const u64>(loc) };
            loc -= 8;
        }
        if encoding & UNWIND_ARM64_FRAME_X27_X28_PAIR != 0 {
            context.x[27] = unsafe { *mem::transmute::<_, *const u64>(loc) };
            loc -= 8;
            context.x[28] = unsafe { *mem::transmute::<_, *const u64>(loc) };
            loc -= 8;
        }
        if encoding & UNWIND_ARM64_FRAME_D8_D9_PAIR != 0 {
            context.d[8] = unsafe { *mem::transmute::<_, *const f64>(loc) };
            loc -= 8;
            context.d[9] = unsafe { *mem::transmute::<_, *const f64>(loc) };
            loc -= 8;
        }
        if encoding & UNWIND_ARM64_FRAME_D10_D11_PAIR != 0 {
            context.d[10] = unsafe { *mem::transmute::<_, *const f64>(loc) };
            loc -= 8;
            context.d[11] = unsafe { *mem::transmute::<_, *const f64>(loc) };
            loc -= 8;
        }
        if encoding & UNWIND_ARM64_FRAME_D12_D13_PAIR != 0 {
            context.d[12] = unsafe { *mem::transmute::<_, *const f64>(loc) };
            loc -= 8;
            context.d[13] = unsafe { *mem::transmute::<_, *const f64>(loc) };
            loc -= 8;
        }
        if encoding & UNWIND_ARM64_FRAME_D14_D15_PAIR != 0 {
            context.d[14] = unsafe { *mem::transmute::<_, *const f64>(loc) };
            loc -= 8;
            context.d[15] = unsafe { *mem::transmute::<_, *const f64>(loc) };
            loc -= 8;
        }
        loc
    }

    fn update_func_info_based_on_pc(&mut self, is_return_address: bool) {
        let mut pc = self.context.pc as usize;

        // Exit early if at the top of the stack.
        if pc == 0 {
            self.unwind_info_missing = true;
            return;
        }

        // If the last line of a function is a "throw" the compiler sometimes
        // emits no instructions after the call to __cxa_throw.  This means
        // the return address is actually the start of the next function.
        // To disambiguate this, back up the pc when we know it is a return
        // address.
        if is_return_address {
            pc -= 1;
        }

        // Ask address space object to find unwind sections for this pc.
        if let Some(sections) = DyldUnwindSections::find(pc) {
            if sections.compact_unwind_section != 0 {
                match UnwindFuncInfo::search(sections, pc) {
                    Some(info) => self.func_info = info,
                    None => {
                        self.unwind_info_missing = true;
                        return;
                    }
                }

                // If unwind table has entry, but entry says there is no unwind info,
                // record that we have no unwind info.
                if self.func_info.encoding == 0 {
                    self.unwind_info_missing = true;
                }
                return;
            }
        }
        self.unwind_info_missing = true;
    }
}

#[derive(Debug, Default)]
struct UnwindFuncInfo {
    func_start: usize,  // start address of function
    func_end: usize,    // address after end of function
    encoding: Encoding, // compact unwind encoding, or zero if none
}

impl std::fmt::Debug for UnwindFuncInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UnwindFuncInfo {{ func_start: {:#x}, func_end: {:#x}, encoding: {:#x} }}",
            self.func_start, self.func_end, self.encoding
        )
    }
}

impl UnwindFuncInfo {
    fn search(sections: DyldUnwindSections, pc: usize) -> Option<UnwindFuncInfo> {
        let base_address = sections.mach_header as usize;
        let section_address = sections.compact_unwind_section as usize;

        let header = unsafe { mem::transmute::<_, &UnwindInfoSectionHeader>(section_address) };
        if header.version != UNWIND_SECTION_VERSION {
            // Err: invalid version of section header
            return None;
        }

        let indexes: &[UnwindInfoSectionHeaderIndexEntry] = unsafe {
            slice::from_raw_parts(
                mem::transmute(section_address + header.index_section_offset as usize),
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
        let l1_next_page_function_offset = indexes[low + 1].function_offset as usize;
        let l2_address = section_address + indexes[low].second_level_pages_section_offset as usize;
        let l2_kind = unsafe { *mem::transmute::<_, *const u32>(l2_address) };
        if l2_kind == UNWIND_SECOND_LEVEL_REGULAR {
            let l2_header = unsafe { mem::transmute::<_, &UnwindInfoRegularSecondLevelPageHeader>(l2_address) };
            let l2_indexes: &[UnwindInfoRegularSecondLevelEntry] = unsafe {
                slice::from_raw_parts(
                    mem::transmute(l2_address + l2_header.entry_page_offset as usize),
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
                        func_end = base_address + l2_indexes[low + 1].function_offset;
                        break;
                    } else {
                        low = mid + 1;
                    }
                } else {
                    high = mid;
                }
            }
            let encoding = l2_indexes[low].encoding;
            let func_start = base_address + l2_indexes[low].function_offset as usize;
            if pc < func_start || pc > func_end {
                // Err: invalid func range
                return None;
            }
            Some(UnwindFuncInfo {
                func_start,
                func_end,
                encoding,
            })
        } else if l2_kind == UNWIND_SECOND_LEVEL_COMPRESSED {
            let l2_header = unsafe { mem::transmute::<_, &UnwindInfoCompressedSecondLevelPageHeader>(l2_address) };
            let l2_indexes: &[u32] = unsafe {
                slice::from_raw_parts(
                    mem::transmute(l2_address + l2_header.entry_page_offset as usize),
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
                + l1_function_offset as usize
                + unwind_info_compressed_entry_func_offset(l2_indexes[low]) as usize;
            let func_end = if low < last {
                base_address
                    + l1_function_offset as usize
                    + unwind_info_compressed_entry_func_offset(l2_indexes[low + 1]) as usize
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
                    let encodings_address = section_address + header.common_encodings_array_section_offset as usize;
                    let encodings_ptr = mem::transmute::<_, *const Encoding>(encodings_address);
                    let encodings_len = header.common_encodings_array_count as usize;
                    let encodings = slice::from_raw_parts(encodings_ptr, encodings_len);
                    encodings[encoding_index as usize]
                }
            } else {
                // encoding is in page specific table
                unsafe {
                    let encodings_address = l2_address + l2_header.encodings_page_offset as usize;
                    let encodings_ptr = mem::transmute::<_, *const Encoding>(encodings_address);
                    let encodings_len = l2_header.encodings_count as usize;
                    let encodings = slice::from_raw_parts(encodings_ptr, encodings_len);
                    encodings[(encoding_index - header.common_encodings_array_count) as usize]
                }
            };
            Some(UnwindFuncInfo {
                func_start,
                func_end,
                encoding,
            })
        } else {
            // Err: invalid second level header kind
            None
        }
    }
}

#[repr(C)]
#[derive(Debug, Default)]
struct DyldUnwindSections {
    mach_header: u64,
    dwarf_section: u64,
    dwarf_section_length: u64,
    compact_unwind_section: u64,
    compact_unwind_section_length: u64,
}

impl DyldUnwindSections {
    fn find(address: usize) -> Option<Self> {
        let mut sections = Self::default();
        unsafe {
            if _dyld_find_unwind_sections(address as _, &mut sections as _) {
                Some(sections)
            } else {
                None
            }
        }
    }
}

extern "C" {
    // In 10.7.0 or later, libSystem.dylib implements this function.
    fn _dyld_find_unwind_sections(address: *mut libc::c_void, sections: *mut DyldUnwindSections) -> bool;
}

// The compact unwind encoding is a 32-bit value which encoded in an
// architecture specific way, which registers to restore from where, and how
// to unwind out of the function.
type Encoding = u32;

// architecture independent bits
const UNWIND_IS_NOT_FUNCTION_START: u32 = 0x80000000;
const UNWIND_HAS_LSDA: u32 = 0x40000000;
const UNWIND_PERSONALITY_MASK: u32 = 0x30000000;

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

const UNWIND_ARM64_MODE_MASK: u32 = 0x0F000000;
const UNWIND_ARM64_MODE_FRAMELESS: u32 = 0x02000000;
const UNWIND_ARM64_MODE_DWARF: u32 = 0x03000000;
const UNWIND_ARM64_MODE_FRAME: u32 = 0x04000000;

const UNWIND_ARM64_FRAME_X19_X20_PAIR: u32 = 0x00000001;
const UNWIND_ARM64_FRAME_X21_X22_PAIR: u32 = 0x00000002;
const UNWIND_ARM64_FRAME_X23_X24_PAIR: u32 = 0x00000004;
const UNWIND_ARM64_FRAME_X25_X26_PAIR: u32 = 0x00000008;
const UNWIND_ARM64_FRAME_X27_X28_PAIR: u32 = 0x00000010;
const UNWIND_ARM64_FRAME_D8_D9_PAIR: u32 = 0x00000100;
const UNWIND_ARM64_FRAME_D10_D11_PAIR: u32 = 0x00000200;
const UNWIND_ARM64_FRAME_D12_D13_PAIR: u32 = 0x00000400;
const UNWIND_ARM64_FRAME_D14_D15_PAIR: u32 = 0x00000800;

const UNWIND_ARM64_FRAMELESS_STACK_SIZE_MASK: u32 = 0x00FFF000;
const UNWIND_ARM64_DWARF_SECTION_OFFSET: u32 = 0x00FFFFFF;

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

// The __TEXT,__unwind_info section is laid out for an efficient two level lookup.
// The header of the section contains a coarse index that maps function address
// to the page (4096 byte block) containing the unwind info for that function.

const UNWIND_SECTION_VERSION: u32 = 1;

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

const UNWIND_SECOND_LEVEL_REGULAR: u32 = 2;

#[repr(C)]
#[derive(Debug)]
struct UnwindInfoRegularSecondLevelPageHeader {
    kind: u32, // UNWIND_SECOND_LEVEL_REGULAR
    entry_page_offset: u16,
    entry_count: u16,
    // entry array
}

const UNWIND_SECOND_LEVEL_COMPRESSED: u32 = 3;

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

    #[test]
    fn test_unwind_context() {
        let context = unsafe {
            let mut context = UnwindContext::default();
            init_unwind_context(&mut context as _);
            context
        };
        assert_ne!(context.pc, 0);
        assert_ne!(context.fp, 0);
    }
}
