use crate::dwarf;
use crate::dyld::{sections, SectionInfo};
use crate::registers::Registers;

/// `UnwindCursor` is used to trace the stack with [Registers].
///
/// `UnwindCursor` is highly platform-dependent. On Linux we use
/// the .eh_frame section (in DWARF format) to restore registers.
///
/// [Registers]: crate::registers::Registers
pub struct UnwindCursor {
    sections: &'static [SectionInfo],
    first_step: bool,
}

impl UnwindCursor {
    /// Creates a new `UnwindCursor`.
    #[inline]
    pub fn new() -> Self {
        Self {
            sections: sections(),
            first_step: true,
        }
    }

    /// Attempts to restore the parent function's register state based on the
    /// current register state.
    ///
    /// On Linux, the recovery rules for registers are described in the
    /// .eh_frame section.
    pub fn step(&mut self, registers: &mut Registers) -> crate::Result<bool> {
        let mut pc = registers.pc();
        if pc == 0 {
            return Ok(false);
        }
        if self.first_step {
            self.first_step = false;
        } else {
            // Usually when we step for the first time, the PC points to the actual
            // position that was interrupted by the signal. But then we'll use `return
            // address` to set the PC. So from now on we need to subtract 1 from the
            // PC to get the correct position before the call instruction.
            pc -= 1;
        }
        for s in self.sections {
            if s.contains(pc) {
                return match dwarf::step(pc, s, registers) {
                    Ok(_) => Ok(true),
                    Err(dwarf::DwarfError::FDENotFound) => Ok(false),
                    Err(err) => Err(err.into()),
                };
            }
        }
        Ok(false)
    }
}
