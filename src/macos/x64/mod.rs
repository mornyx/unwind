use crate::macos::compact::{DyldUnwindSections, UnwindFuncInfo};
use gimli::{EndianSlice, NativeEndian, Reader, Register, UnwindContext, X86_64};
use std::ops::{Index, IndexMut};

mod compact;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Registers {
    v: [u64; 128], // rax, rdx, rcx, rbx, rsi, rdi, rbp, rsp, r8~r15, rip, ...
}

impl Default for Registers {
    fn default() -> Self {
        Self { v: [0; 128] }
    }
}

impl Registers {
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

    #[inline]
    pub fn pc(&self) -> u64 {
        self[X86_64::RA]
    }

    #[inline]
    pub fn set_pc(&mut self, v: u64) {
        self[X86_64::RA] = v;
    }

    #[inline]
    pub fn sp(&self) -> u64 {
        self[X86_64::RSP]
    }

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

pub struct UnwindCursor<R: Reader> {
    ctx: UnwindContext<R>,
}

impl UnwindCursor<EndianSlice<'static, NativeEndian>> {
    #[inline]
    pub fn new() -> Self {
        Self {
            ctx: UnwindContext::new(),
        }
    }

    pub fn step(&mut self, registers: &mut Registers) -> bool {
        let mut pc = registers.pc();
        if pc == 0 {
            return false;
        }
        pc -= 1;
        if let Some(sections) = DyldUnwindSections::find(pc as _) {
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
