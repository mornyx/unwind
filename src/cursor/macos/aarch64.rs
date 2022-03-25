use crate::registers::Registers;
use crate::utils::load;
use crate::Result;

/// `UnwindCursor` is used to trace the stack with [Registers].
///
/// `UnwindCursor` is highly platform-dependent. On macOS+aarch64 we can be sure
/// that frame pointer exists on the stack, so we simply use frame pointer to
/// unwind, which is the easiest and fastest way.
///
/// For more info about "frame pointer" on macOS+aarch64, please see:
/// https://developer.apple.com/documentation/xcode/writing-arm64-code-for-apple-platforms
///
/// [Registers]: crate::registers::Registers
pub struct UnwindCursor;

impl UnwindCursor {
    /// Creates a new `UnwindCursor`.
    #[inline]
    pub fn new() -> Self {
        Self
    }

    /// Attempts to restore the parent function's register state based on the
    /// current register state.
    ///
    /// On macOS+aarch64 platform we simply use the frame pointer to unwind.
    /// This means that only PC (Program Counter) and FP (Frame Pointer) are
    /// restored. This is enough to trace call stack.
    pub fn step(&mut self, registers: &mut Registers) -> Result<bool> {
        if registers.fp == 0 {
            return Ok(false);
        }
        registers.pc = load::<u64>(registers.fp + 8);
        registers.fp = load::<u64>(registers.fp);
        Ok(true)
    }
}
