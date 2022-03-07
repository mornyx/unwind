use gimli::Register;
use std::ops::{Index, IndexMut};

// See: https://developer.arm.com/documentation/ihi0057/b/
const RA: u16 = 30;
const SP: u16 = 31;

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
#[derive(Debug, Copy, Clone)]
pub struct Registers {
    // 128 is enough for all registers on x86_64.
    //
    // Although we don't need to restore most of these registers, it is necessary
    // to reserve space for them because we will index them numerically when restoring
    // registers from DWARF information. Reserve enough space can avoid overflow.
    x: [u64; 128], // x0 ~ x30, sp, ...
}

impl Default for Registers {
    fn default() -> Self {
        Self { x: [0; 128] }
    }
}

impl Registers {
    /// Initialize `Registers` with value from `ucontext`.
    pub fn from_ucontext(ucontext: *mut libc::c_void) -> Option<Self> {
        let ucontext = ucontext as *mut libc::ucontext_t;
        if ucontext.is_null() {
            return None;
        }
        let mcontext = unsafe { (*ucontext).uc_mcontext };
        let mut registers = Self::default();
        for n in 0..31 {
            registers.x[n] = mcontext.regs[n]
        }
        registers[SP] = mcontext.sp;
        Some(registers)
    }

    /// Get the value of the PC (Program Counter) register.
    /// This value is usually "Return Address" in implementation.
    #[inline]
    pub fn pc(&self) -> u64 {
        self[RA]
    }

    /// Set the value of the PC (Program Counter) register.
    #[inline]
    pub fn set_pc(&mut self, v: u64) {
        self[RA] = v;
    }

    /// Get the value of the SP (Stack Pointer) register.
    #[inline]
    pub fn sp(&self) -> u64 {
        self[SP]
    }

    /// Set the value of the SP (Stack Pointer) register.
    #[inline]
    pub fn set_sp(&mut self, v: u64) {
        self[SP] = v;
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
