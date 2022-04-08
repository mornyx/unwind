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
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rdi: u64,
    rsi: u64,
    rbp: u64,
    rsp: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    rip: u64,
}

impl Index<usize> for Registers {
    type Output = u64;

    fn index(&self, index: usize) -> &u64 {
        match index {
            UNW_REG_IP | UNW_X86_64_RIP => &self.rip,
            UNW_REG_SP | UNW_X86_64_RSP => &self.rsp,
            UNW_X86_64_RAX => &self.rax,
            UNW_X86_64_RBX => &self.rbx,
            UNW_X86_64_RCX => &self.rcx,
            UNW_X86_64_RDX => &self.rdx,
            UNW_X86_64_RSI => &self.rsi,
            UNW_X86_64_RDI => &self.rdi,
            UNW_X86_64_RBP => &self.rbp,
            UNW_X86_64_R8 => &self.r8,
            UNW_X86_64_R9 => &self.r9,
            UNW_X86_64_R10 => &self.r10,
            UNW_X86_64_R11 => &self.r11,
            UNW_X86_64_R12 => &self.r12,
            UNW_X86_64_R13 => &self.r13,
            UNW_X86_64_R14 => &self.r14,
            UNW_X86_64_R15 => &self.r15,
            _ => unreachable!(),
        }
    }
}

impl IndexMut<usize> for Registers {
    fn index_mut(&mut self, index: usize) -> &mut u64 {
        match index {
            UNW_REG_IP | UNW_X86_64_RIP => &mut self.rip,
            UNW_REG_SP | UNW_X86_64_RSP => &mut self.rsp,
            UNW_X86_64_RAX => &mut self.rax,
            UNW_X86_64_RBX => &mut self.rbx,
            UNW_X86_64_RCX => &mut self.rcx,
            UNW_X86_64_RDX => &mut self.rdx,
            UNW_X86_64_RSI => &mut self.rsi,
            UNW_X86_64_RDI => &mut self.rdi,
            UNW_X86_64_RBP => &mut self.rbp,
            UNW_X86_64_R8 => &mut self.r8,
            UNW_X86_64_R9 => &mut self.r9,
            UNW_X86_64_R10 => &mut self.r10,
            UNW_X86_64_R11 => &mut self.r11,
            UNW_X86_64_R12 => &mut self.r12,
            UNW_X86_64_R13 => &mut self.r13,
            UNW_X86_64_R14 => &mut self.r14,
            UNW_X86_64_R15 => &mut self.r15,
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
        registers[UNW_X86_64_RAX] = mcontext.gregs[libc::REG_RAX as usize] as _;
        registers[UNW_X86_64_RBX] = mcontext.gregs[libc::REG_RBX as usize] as _;
        registers[UNW_X86_64_RCX] = mcontext.gregs[libc::REG_RCX as usize] as _;
        registers[UNW_X86_64_RDX] = mcontext.gregs[libc::REG_RDX as usize] as _;
        registers[UNW_X86_64_RDI] = mcontext.gregs[libc::REG_RDI as usize] as _;
        registers[UNW_X86_64_RSI] = mcontext.gregs[libc::REG_RSI as usize] as _;
        registers[UNW_X86_64_RBP] = mcontext.gregs[libc::REG_RBP as usize] as _;
        registers[UNW_X86_64_RSP] = mcontext.gregs[libc::REG_RSP as usize] as _;
        registers[UNW_X86_64_R8] = mcontext.gregs[libc::REG_R8 as usize] as _;
        registers[UNW_X86_64_R9] = mcontext.gregs[libc::REG_R9 as usize] as _;
        registers[UNW_X86_64_R10] = mcontext.gregs[libc::REG_R10 as usize] as _;
        registers[UNW_X86_64_R11] = mcontext.gregs[libc::REG_R11 as usize] as _;
        registers[UNW_X86_64_R12] = mcontext.gregs[libc::REG_R12 as usize] as _;
        registers[UNW_X86_64_R13] = mcontext.gregs[libc::REG_R13 as usize] as _;
        registers[UNW_X86_64_R14] = mcontext.gregs[libc::REG_R14 as usize] as _;
        registers[UNW_X86_64_R15] = mcontext.gregs[libc::REG_R15 as usize] as _;
        registers[UNW_X86_64_RIP] = mcontext.gregs[libc::REG_RIP as usize] as _;
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
            let mut registers = Self::default();
            registers[UNW_X86_64_RAX] = (*mcontext).__ss.__rax;
            registers[UNW_X86_64_RBX] = (*mcontext).__ss.__rbx;
            registers[UNW_X86_64_RCX] = (*mcontext).__ss.__rcx;
            registers[UNW_X86_64_RDX] = (*mcontext).__ss.__rdx;
            registers[UNW_X86_64_RDI] = (*mcontext).__ss.__rdi;
            registers[UNW_X86_64_RSI] = (*mcontext).__ss.__rsi;
            registers[UNW_X86_64_RBP] = (*mcontext).__ss.__rbp;
            registers[UNW_X86_64_RSP] = (*mcontext).__ss.__rsp;
            registers[UNW_X86_64_R8] = (*mcontext).__ss.__r8;
            registers[UNW_X86_64_R9] = (*mcontext).__ss.__r9;
            registers[UNW_X86_64_R10] = (*mcontext).__ss.__r10;
            registers[UNW_X86_64_R11] = (*mcontext).__ss.__r11;
            registers[UNW_X86_64_R12] = (*mcontext).__ss.__r12;
            registers[UNW_X86_64_R13] = (*mcontext).__ss.__r13;
            registers[UNW_X86_64_R14] = (*mcontext).__ss.__r14;
            registers[UNW_X86_64_R15] = (*mcontext).__ss.__r15;
            registers[UNW_X86_64_RIP] = (*mcontext).__ss.__rip;
            Some(registers)
        }
    }

    #[inline]
    pub fn max_register_num() -> usize {
        UNW_X86_64_MAX_REG_NUM
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
        if n > UNW_X86_64_MAX_REG_NUM {
            return false;
        }
        true
    }

    #[inline]
    pub fn float_register(&self, _n: usize) -> f64 {
        unreachable!()
    }

    #[inline]
    pub fn set_float_register(&mut self, _n: usize, _v: f64) {
        unreachable!()
    }

    #[inline]
    pub fn valid_float_register(&self, _n: usize) -> bool {
        false
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
