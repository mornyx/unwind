use gimli::{Register, X86_64};
use std::ops::{Index, IndexMut};

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
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
        let mcontext = unsafe { (*ucontext).uc_mcontext };
        let mut registers = Self::default();
        registers[X86_64::RAX] = mcontext.gregs[libc::REG_RAX as usize] as _;
        registers[X86_64::RDX] = mcontext.gregs[libc::REG_RDX as usize] as _;
        registers[X86_64::RCX] = mcontext.gregs[libc::REG_RCX as usize] as _;
        registers[X86_64::RBX] = mcontext.gregs[libc::REG_RBX as usize] as _;
        registers[X86_64::RSI] = mcontext.gregs[libc::REG_RSI as usize] as _;
        registers[X86_64::RDI] = mcontext.gregs[libc::REG_RDI as usize] as _;
        registers[X86_64::RBP] = mcontext.gregs[libc::REG_RBP as usize] as _;
        registers[X86_64::RSP] = mcontext.gregs[libc::REG_RSP as usize] as _;
        registers[X86_64::R8] = mcontext.gregs[libc::REG_R8 as usize] as _;
        registers[X86_64::R9] = mcontext.gregs[libc::REG_R9 as usize] as _;
        registers[X86_64::R10] = mcontext.gregs[libc::REG_R10 as usize] as _;
        registers[X86_64::R11] = mcontext.gregs[libc::REG_R11 as usize] as _;
        registers[X86_64::R12] = mcontext.gregs[libc::REG_R12 as usize] as _;
        registers[X86_64::R13] = mcontext.gregs[libc::REG_R13 as usize] as _;
        registers[X86_64::R14] = mcontext.gregs[libc::REG_R14 as usize] as _;
        registers[X86_64::R15] = mcontext.gregs[libc::REG_R15 as usize] as _;
        registers[X86_64::RA] = mcontext.gregs[libc::REG_RIP as usize] as _;
        Some(registers)
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
