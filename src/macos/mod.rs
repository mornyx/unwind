#[cfg(feature = "compact-unwind-info")]
mod compact;
#[cfg(feature = "compact-unwind-info")]
pub use compact::*;

#[cfg(not(feature = "compact-unwind-info"))]
mod fp;
#[cfg(not(feature = "compact-unwind-info"))]
pub use fp::*;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct UnwindContext {
    pub x: [u64; 29], // x0 ~ x29
    pub fp: u64,
    pub lr: u64,
    pub sp: u64,
    pub pc: u64,
    pub d: [f64; 32], // d0 ~ d31
}

impl UnwindContext {
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
}

extern "C" {
    pub fn init_unwind_context(context: *mut UnwindContext);
}
