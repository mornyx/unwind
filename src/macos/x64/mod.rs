use crate::macos::compact::{DyldUnwindSections, UnwindFuncInfo};
use gimli::{EndianSlice, NativeEndian, Reader, Register, UnwindContext, X86_64};
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
#[derive(Debug, Copy, Clone)]
pub struct Registers {
    // 128 is enough for all registers on x86_64.
    //
    // Although we don't need to restore most of these registers, it is necessary
    // to reserve space for them because we will index them numerically when restoring
    // registers from DWARF information. Reserve enough space can avoid overflow.
    v: [u64; 128], // rax, rdx, rcx, rbx, rsi, rdi, rbp, rsp, r8~r15, rip, ...
}

impl Default for Registers {
    fn default() -> Self {
        Self { v: [0; 128] }
    }
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
            let mut registers = Self::default();
            registers[X86_64::RAX] = (*mcontext).__ss.__rax;
            registers[X86_64::RDX] = (*mcontext).__ss.__rdx;
            registers[X86_64::RCX] = (*mcontext).__ss.__rcx;
            registers[X86_64::RBX] = (*mcontext).__ss.__rbx;
            registers[X86_64::RSI] = (*mcontext).__ss.__rsi;
            registers[X86_64::RDI] = (*mcontext).__ss.__rdi;
            registers[X86_64::RBP] = (*mcontext).__ss.__rbp;
            registers[X86_64::RSP] = (*mcontext).__ss.__rsp;
            registers[X86_64::R8] = (*mcontext).__ss.__r8;
            registers[X86_64::R9] = (*mcontext).__ss.__r9;
            registers[X86_64::R10] = (*mcontext).__ss.__r10;
            registers[X86_64::R11] = (*mcontext).__ss.__r11;
            registers[X86_64::R12] = (*mcontext).__ss.__r12;
            registers[X86_64::R13] = (*mcontext).__ss.__r13;
            registers[X86_64::R14] = (*mcontext).__ss.__r14;
            registers[X86_64::R15] = (*mcontext).__ss.__r15;
            registers[X86_64::RA] = (*mcontext).__ss.__rip;
            Some(registers)
        }
    }

    /// Get the value of the PC (Program Counter) register.
    /// This value is usually "Return Address" in implementation.
    #[inline]
    pub fn pc(&self) -> u64 {
        self[X86_64::RA]
    }

    /// Set the value of the PC (Program Counter) register.
    #[inline]
    pub fn set_pc(&mut self, v: u64) {
        self[X86_64::RA] = v;
    }

    /// Get the value of the SP (Stack Pointer) register.
    #[inline]
    pub fn sp(&self) -> u64 {
        self[X86_64::RSP]
    }

    /// Set the value of the SP (Stack Pointer) register.
    #[inline]
    pub fn set_sp(&mut self, v: u64) {
        self[X86_64::RSP] = v;
    }
}

impl Index<u16> for Registers {
    type Output = u64;

    fn index(&self, index: u16) -> &u64 {
        &self.v[index as usize]
    }
}

impl IndexMut<u16> for Registers {
    fn index_mut(&mut self, index: u16) -> &mut u64 {
        &mut self.v[index as usize]
    }
}

impl Index<Register> for Registers {
    type Output = u64;

    fn index(&self, index: Register) -> &u64 {
        &self.v[index.0 as usize]
    }
}

impl IndexMut<Register> for Registers {
    fn index_mut(&mut self, index: Register) -> &mut u64 {
        &mut self.v[index.0 as usize]
    }
}

/// `UnwindCursor` is used to trace the stack with [Registers].
///
/// `UnwindCursor` is highly platform-dependent. On macOS+x86_64 we use
/// a combination of "Compact Unwind Encoding" and "DWARF".
///
/// [Registers]: crate::Registers
pub struct UnwindCursor<R: Reader> {
    ctx: UnwindContext<R>,
}

impl UnwindCursor<EndianSlice<'static, NativeEndian>> {
    /// Creates a new `UnwindCursor`.
    #[inline]
    pub fn new() -> Self {
        Self {
            ctx: UnwindContext::new(),
        }
    }

    /// Attempts to restore the parent function's register state based on the
    /// current register state.
    ///
    /// On macOS we mostly use "Compact Unwind Encoding", but it may direct us
    /// to use DWARF instead.
    pub fn step(&mut self, registers: &mut Registers) -> bool {
        let mut pc = registers.pc();
        if pc == 0 {
            return false;
        }
        pc -= 1;
        if let Some(sections) = DyldUnwindSections::find(pc) {
            if sections.compact_unwind_section != 0 {
                if let Some(info) = UnwindFuncInfo::find(sections, pc as _) {
                    if info.encoding == 0 {
                        return false;
                    }
                    return compact::step(registers, info, sections, &mut self.ctx);
                }
            }
        }
        false
    }
}
