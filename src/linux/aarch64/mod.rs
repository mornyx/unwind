use gimli::Register;
use std::ops::{Index, IndexMut};

// See: https://developer.arm.com/documentation/ihi0057/b/
const FP: u16 = 29;
const RA: u16 = 30;
const SP: u16 = 31;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Registers {
    x: [u64; 128], // x0 ~ x30, sp, ...
}

impl Default for Registers {
    fn default() -> Self {
        Self { x: [0; 128] }
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
        for n in 0..31 {
            registers.x[n] = mcontext.regs[n]
        }
        registers[SP] = mcontext.sp;
        Some(registers)
    }

    #[inline]
    pub fn pc(&self) -> u64 {
        self[RA]
    }

    #[inline]
    pub fn set_pc(&mut self, v: u64) {
        self[RA] = v;
    }

    #[inline]
    pub fn sp(&self) -> u64 {
        self[SP]
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, SIGPROF};
    use std::sync::atomic::{AtomicBool, Ordering};

    static WAIT: AtomicBool = AtomicBool::new(true);

    #[test]
    fn test_from_ucontext() {
        let h = SigHandler::SigAction(signal_handler);
        let a = SigAction::new(h, SaFlags::SA_SIGINFO, SigSet::empty());
        unsafe {
            sigaction(SIGPROF, &a).unwrap();
            libc::kill(libc::getpid(), libc::SIGPROF);
        }
        while WAIT.load(Ordering::SeqCst) {}
    }

    #[no_mangle]
    extern "C" fn signal_handler(_: libc::c_int, _: *mut libc::siginfo_t, ucontext: *mut libc::c_void) {
        let registers = Registers::from_ucontext(ucontext).unwrap();
        assert!(registers.pc() > 0);
        assert!(registers.sp() > 0);
        WAIT.store(false, Ordering::SeqCst);
    }
}
