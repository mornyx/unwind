#![allow(unused)]

use crate::registers::{UNW_REG_IP, UNW_REG_SP};
use std::ops::{Index, IndexMut};

// ARM64 register numbers.
pub const UNW_ARM64_X0: usize = 0;
pub const UNW_ARM64_X1: usize = 1;
pub const UNW_ARM64_X2: usize = 2;
pub const UNW_ARM64_X3: usize = 3;
pub const UNW_ARM64_X4: usize = 4;
pub const UNW_ARM64_X5: usize = 5;
pub const UNW_ARM64_X6: usize = 6;
pub const UNW_ARM64_X7: usize = 7;
pub const UNW_ARM64_X8: usize = 8;
pub const UNW_ARM64_X9: usize = 9;
pub const UNW_ARM64_X10: usize = 10;
pub const UNW_ARM64_X11: usize = 11;
pub const UNW_ARM64_X12: usize = 12;
pub const UNW_ARM64_X13: usize = 13;
pub const UNW_ARM64_X14: usize = 14;
pub const UNW_ARM64_X15: usize = 15;
pub const UNW_ARM64_X16: usize = 16;
pub const UNW_ARM64_X17: usize = 17;
pub const UNW_ARM64_X18: usize = 18;
pub const UNW_ARM64_X19: usize = 19;
pub const UNW_ARM64_X20: usize = 20;
pub const UNW_ARM64_X21: usize = 21;
pub const UNW_ARM64_X22: usize = 22;
pub const UNW_ARM64_X23: usize = 23;
pub const UNW_ARM64_X24: usize = 24;
pub const UNW_ARM64_X25: usize = 25;
pub const UNW_ARM64_X26: usize = 26;
pub const UNW_ARM64_X27: usize = 27;
pub const UNW_ARM64_X28: usize = 28;
pub const UNW_ARM64_X29: usize = 29;
pub const UNW_ARM64_FP: usize = 29;
pub const UNW_ARM64_X30: usize = 30;
pub const UNW_ARM64_LR: usize = 30;
pub const UNW_ARM64_X31: usize = 31;
pub const UNW_ARM64_SP: usize = 31;
pub const UNW_ARM64_PC: usize = 32;
// reserved block
pub const UNW_ARM64_RA_SIGN_STATE: usize = 34;
// reserved block
pub const UNW_ARM64_D0: usize = 64;
pub const UNW_ARM64_D1: usize = 65;
pub const UNW_ARM64_D2: usize = 66;
pub const UNW_ARM64_D3: usize = 67;
pub const UNW_ARM64_D4: usize = 68;
pub const UNW_ARM64_D5: usize = 69;
pub const UNW_ARM64_D6: usize = 70;
pub const UNW_ARM64_D7: usize = 71;
pub const UNW_ARM64_D8: usize = 72;
pub const UNW_ARM64_D9: usize = 73;
pub const UNW_ARM64_D10: usize = 74;
pub const UNW_ARM64_D11: usize = 75;
pub const UNW_ARM64_D12: usize = 76;
pub const UNW_ARM64_D13: usize = 77;
pub const UNW_ARM64_D14: usize = 78;
pub const UNW_ARM64_D15: usize = 79;
pub const UNW_ARM64_D16: usize = 80;
pub const UNW_ARM64_D17: usize = 81;
pub const UNW_ARM64_D18: usize = 82;
pub const UNW_ARM64_D19: usize = 83;
pub const UNW_ARM64_D20: usize = 84;
pub const UNW_ARM64_D21: usize = 85;
pub const UNW_ARM64_D22: usize = 86;
pub const UNW_ARM64_D23: usize = 87;
pub const UNW_ARM64_D24: usize = 88;
pub const UNW_ARM64_D25: usize = 89;
pub const UNW_ARM64_D26: usize = 90;
pub const UNW_ARM64_D27: usize = 91;
pub const UNW_ARM64_D28: usize = 92;
pub const UNW_ARM64_D29: usize = 93;
pub const UNW_ARM64_D30: usize = 94;
pub const UNW_ARM64_D31: usize = 95;
pub const UNW_ARM64_MAX_REG_NUM: usize = 95;

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

    #[inline]
    pub fn valid_register(&self, n: usize) -> bool {
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
    pub fn float_register(&self, n: usize) -> f64 {
        assert!(self.valid_float_register(n));
        self.d[n - UNW_ARM64_D0]
    }

    #[inline]
    pub fn set_float_register(&mut self, n: usize, v: f64) {
        assert!(self.valid_float_register(n));
        self.d[n - UNW_ARM64_D0] = v;
    }

    #[inline]
    pub fn valid_float_register(&self, n: usize) -> bool {
        if n >= UNW_ARM64_D0 && n <= UNW_ARM64_D31 {
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn vector_register(&self, _n: usize) -> bool {
        unreachable!();
    }

    #[inline]
    pub fn set_vector_register(&mut self, _n: usize, _v: u128) {
        unreachable!();
    }

    #[inline]
    pub fn valid_vector_register(&self, _n: usize) -> bool {
        false
    }
}
