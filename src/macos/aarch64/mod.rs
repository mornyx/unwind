use gimli::Register;
use std::ops::{Index, IndexMut};

mod compact;

/// `Registers` holds the register context for a specific platform (OS+ISA).
///
/// We can use [unwind_init_registers] to initialize `Registers` based on
/// the current execution context:
/// ```ignore
/// let mut registers = Registers::default();
/// unsafe { unwind_init_registers(&mut registers as _) };
/// assert_ne!(registers.pc(), 0);
/// ```
///
/// But more suitable for this crate usage scenario is to use an existing
/// `ucontext`. Usually the kernel provides an `ucontext` for the signal
/// handler:
/// ```ignore
/// extern "C" fn signal_handler(_: libc::c_int, _: *mut libc::siginfo_t, ucontext: *mut libc::c_void) {
///     let registers = Registers::from_ucontext(ucontext);
///     assert_ne!(registers.pc(), 0);
/// }
/// ```
///
/// We can restore `Registers` through [UnwindCursor] to get the execution
/// context of the **parent** function:
/// ```ignore
/// let mut cursor = UnwindCursor::new();
/// cursor.step(&mut registers);
/// ```
///
/// [UnwindCursor]: crate::UnwindCursor
/// [unwind_init_registers]: crate::unwind_init_registers
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct Registers {
    pub x: [u64; 29], // x0 ~ x29
    pub fp: u64,
    pub lr: u64,
    pub sp: u64,
    pub pc: u64,
    pub d: [f64; 32], // d0 ~ d31
}

impl Registers {
    /// Initialize `Registers` with value from `ucontext`.
    pub fn from_ucontext(ucontext: *mut libc::c_void) -> Option<Self> {
        let ucontext = ucontext as *mut libc::ucontext_t;
        if ucontext.is_null() {
            return None;
        }
        unsafe {
            let mcontext = (*ucontext).uc_mcontext;
            if mcontext.is_null() {
                return None;
            }
            Some(Self {
                x: (*mcontext).__ss.__x,
                fp: (*mcontext).__ss.__fp,
                lr: (*mcontext).__ss.__lr,
                sp: (*mcontext).__ss.__sp,
                pc: (*mcontext).__ss.__pc,
                d: [0f64; 32], // TODO: extract from (*mcontext).__ns.__v
            })
        }
    }

    /// Get the value of the PC (Program Counter) register.
    /// This value is usually "Return Address" in implementation.
    #[inline]
    pub fn pc(&self) -> u64 {
        self.pc
    }

    /// Set the value of the PC (Program Counter) register.
    #[inline]
    pub fn set_pc(&mut self, v: u64) {
        self.pc = v;
    }

    /// Get the value of the SP (Stack Pointer) register.
    #[inline]
    pub fn sp(&self) -> u64 {
        self.sp
    }

    /// Set the value of the SP (Stack Pointer) register.
    #[inline]
    pub fn set_sp(&mut self, v: u64) {
        self.sp = v;
    }
}

impl Index<u16> for Registers {
    type Output = u64;

    fn index(&self, index: u16) -> &u64 {
        &self.x[index as usize]
    }
}

impl IndexMut<u16> for Registers {
    fn index_mut(&mut self, index: u16) -> &mut u64 {
        &mut self.x[index as usize]
    }
}

impl Index<Register> for Registers {
    type Output = u64;

    fn index(&self, index: Register) -> &u64 {
        &self.x[index.0 as usize]
    }
}

impl IndexMut<Register> for Registers {
    fn index_mut(&mut self, index: Register) -> &mut u64 {
        &mut self.x[index.0 as usize]
    }
}

/// `UnwindCursor` is used to trace the stack with [Registers].
///
/// `UnwindCursor` is highly platform-dependent. On macOS+aarch64 we can be sure
/// that frame pointer exists on the stack, so we simply use frame pointer to
/// unwind, which is the easiest and fastest way.
///
/// For more info about "frame pointer" on macOS+aarch64, please see:
/// https://developer.apple.com/documentation/xcode/writing-arm64-code-for-apple-platforms
///
/// [Registers]: crate::Registers
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
    /// restored. This is enough for "Profiling".
    pub fn step(&mut self, registers: &mut Registers) -> bool {
        if registers.fp == 0 {
            return false;
        }
        unsafe {
            registers.pc = *((registers.fp + 8) as *const u64);
            registers.fp = *(registers.fp as *const u64);
        }
        true
    }
}
