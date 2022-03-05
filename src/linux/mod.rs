use dwarf::{SectionInfo, MAX_OBJECTS_LEN};
use gimli::{EndianSlice, NativeEndian, Reader, UnwindContext};
use smallvec::SmallVec;

mod dwarf;

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "aarch64")]
pub use aarch64::*;

#[cfg(target_arch = "x86_64")]
mod x64;
#[cfg(target_arch = "x86_64")]
pub use x64::*;

pub struct UnwindCursor<R: Reader> {
    ctx: UnwindContext<R>,
    sections: SmallVec<[SectionInfo<R>; MAX_OBJECTS_LEN]>,
}

impl UnwindCursor<EndianSlice<'static, NativeEndian>> {
    #[inline]
    pub fn new() -> Self {
        Self {
            ctx: UnwindContext::new(),
            sections: dwarf::find_sections(),
        }
    }

    #[inline]
    pub fn step(&mut self, registers: &mut Registers) -> bool {
        let mut pc = registers.pc();
        if pc == 0 {
            return false;
        }
        pc -= 1;
        for s in &self.sections {
            if s.contains(pc) {
                let row = match s.find_unwind_table_row(&mut self.ctx, pc) {
                    Some(v) => v,
                    None => return false,
                };
                return crate::dwarf::step(registers, row);
            }
        }
        false
    }
}
