use gimli::Register;
use std::ops::{Index, IndexMut};

mod compact;

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

    #[inline]
    pub fn pc(&self) -> u64 {
        self.pc
    }

    #[inline]
    pub fn set_pc(&mut self, v: u64) {
        self.pc = v;
    }

    #[inline]
    pub fn sp(&self) -> u64 {
        self.sp
    }

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

pub struct UnwindCursor;

impl UnwindCursor {
    #[inline]
    pub fn new() -> Self {
        Self
    }

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
