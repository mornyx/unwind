use crate::macos::compact::{DyldUnwindSections, UnwindFuncInfo};
use gimli::{EndianSlice, NativeEndian, Reader, Register, UnwindContext, X86_64};
use std::ops::{Index, IndexMut};

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
}

/// `UnwindCursor` is used to trace the stack with [Registers].
///
/// `UnwindCursor` is highly platform-dependent. On macOS+x86_64 we use
/// a combination of "Compact Unwind Encoding" and "DWARF".
///
/// [Registers]: crate::Registers
pub struct UnwindCursor<R: Reader> {
    ctx: UnwindContext<R>,
    first: bool,
}

impl UnwindCursor<EndianSlice<'static, NativeEndian>> {
    /// Creates a new `UnwindCursor`.
    #[inline]
    pub fn new() -> Self {
        Self {
            ctx: UnwindContext::new(),
            first: true,
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
        // TODO: Do not -1 when pc is not "return address".
        pc -= 1;
        if let Some(sections) = DyldUnwindSections::find(pc) {
            if sections.compact_unwind_section != 0 {
                if self.first {
                    self.first = false;
                    return true;
                }
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
