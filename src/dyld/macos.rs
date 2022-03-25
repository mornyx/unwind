#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct DyldUnwindSections {
    pub mach_header: u64,
    pub dwarf_section: u64,
    pub dwarf_section_length: u64,
    pub compact_unwind_section: u64,
    pub compact_unwind_section_length: u64,
}

impl DyldUnwindSections {
    pub fn find(address: u64) -> Option<Self> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registers::{unwind_init_registers, Registers};

    #[test]
    fn test_find_dyld_unwind_sections() {
        let mut registers = Registers::default();
        unsafe {
            unwind_init_registers(&mut registers as _);
        }
        let sections = DyldUnwindSections::find(registers.pc()).unwrap();
        assert_ne!(sections.mach_header, 0);
        assert_ne!(sections.compact_unwind_section, 0);
        assert_ne!(sections.compact_unwind_section_length, 0);
    }
}
