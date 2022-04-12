use crate::registers::consts::*;
use std::ops::{Index, IndexMut};

/// `Registers` holds the register context for a specific platform (OS+ISA).
///
/// We can use [unwind_init_registers] to initialize `Registers` based on
/// the current execution context:
/// ```
/// use unwind::{unwind_init_registers, Registers};
///
/// fn main() {
///     let mut registers = Registers::default();
///     unsafe { unwind_init_registers(&mut registers as _) };
///     assert_ne!(registers.pc(), 0);
/// }
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
/// cursor.step(&mut registers).unwrap();
/// ```
///
/// [UnwindCursor]: crate::cursor::UnwindCursor
/// [unwind_init_registers]: crate::registers::unwind_init_registers
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct Registers {
    x: [u64; 29], // x0 ~ x28
    fp: u64,
    lr: u64,
    sp: u64,
    pc: u64,
    ra_sign_state: u64,
    d: [f64; 32], // d0 ~ d31
}

impl Index<usize> for Registers {
    type Output = u64;

    fn index(&self, index: usize) -> &u64 {
        assert!(Self::valid_register(index));
        match index {
            UNW_REG_IP | UNW_ARM64_PC => &self.pc,
            UNW_REG_SP | UNW_ARM64_SP => &self.sp,
            UNW_ARM64_RA_SIGN_STATE => &self.ra_sign_state,
            UNW_ARM64_FP => &self.fp,
            UNW_ARM64_LR => &self.lr,
            0..=28 => &self.x[index],
            _ => unreachable!(),
        }
    }
}

impl IndexMut<usize> for Registers {
    fn index_mut(&mut self, index: usize) -> &mut u64 {
        assert!(Self::valid_register(index));
        match index {
            UNW_REG_IP | UNW_ARM64_PC => &mut self.pc,
            UNW_REG_SP | UNW_ARM64_SP => &mut self.sp,
            UNW_ARM64_RA_SIGN_STATE => &mut self.ra_sign_state,
            UNW_ARM64_FP => &mut self.fp,
            UNW_ARM64_LR => &mut self.lr,
            0..=28 => &mut self.x[index],
            _ => unreachable!(),
        }
    }
}

impl Registers {
    /// Initialize `Registers` with value from `ucontext`.
    #[cfg(target_os = "linux")]
    pub fn from_ucontext(ucontext: *mut libc::c_void) -> Option<Self> {
        let ucontext = ucontext as *mut libc::ucontext_t;
        if ucontext.is_null() {
            return None;
        }
        let mcontext = unsafe { (*ucontext).uc_mcontext };
        let mut registers = Self::default();
        for n in 0..=28 {
            registers.x[n] = mcontext.regs[n]
        }
        registers.fp = mcontext.regs[29];
        registers.lr = mcontext.regs[30];
        registers.sp = mcontext.sp;
        registers.pc = mcontext.pc;
        registers.ra_sign_state = mcontext.pstate;
        Some(registers)
    }

    /// Initialize `Registers` with value from `ucontext`.
    #[cfg(target_os = "macos")]
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
                ra_sign_state: 0,
                d: [0f64; 32],
            })
        }
    }

    #[inline]
    pub fn max_register_num() -> usize {
        UNW_ARM64_MAX_REG_NUM
    }

    #[inline]
    pub fn valid_register(n: usize) -> bool {
        if n == UNW_REG_IP || n == UNW_REG_SP {
            return true;
        }
        if n > UNW_ARM64_MAX_REG_NUM {
            return false;
        }
        if n == UNW_ARM64_RA_SIGN_STATE {
            return true;
        }
        if n > 32 && n < 64 {
            return false;
        }
        true
    }

    #[inline]
    pub fn valid_float_register(n: usize) -> bool {
        if n >= UNW_ARM64_D0 && n <= UNW_ARM64_D31 {
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn valid_vector_register(_n: usize) -> bool {
        false
    }

    #[inline]
    pub fn float_register(&self, n: usize) -> f64 {
        assert!(Self::valid_float_register(n));
        self.d[n - UNW_ARM64_D0]
    }

    #[inline]
    pub fn set_float_register(&mut self, n: usize, v: f64) {
        assert!(Self::valid_float_register(n));
        self.d[n - UNW_ARM64_D0] = v;
    }

    #[inline]
    pub fn vector_register(&self, _n: usize) -> bool {
        unreachable!();
    }

    #[inline]
    pub fn set_vector_register(&mut self, _n: usize, _v: u128) {
        unreachable!();
    }

    /// Get the value of the PC (Program Counter) register.
    #[inline]
    pub fn pc(&self) -> u64 {
        self[UNW_REG_IP]
    }

    /// Get the value of the SP (Stack Pointer) register.
    #[inline]
    pub fn sp(&self) -> u64 {
        self[UNW_REG_SP]
    }
}
