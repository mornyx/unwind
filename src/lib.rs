//! This crate provides the basic stacktrace facility, the main purpose of which is to
//! provide a safe way to trace the stack in signal handler. (The main scenario for
//! this requirement is CPU Profiling)
//!
//! Simple usage:
//! ```
//! fn main() {
//!     // Do stack backtrace.
//!     let mut pcs = vec![];
//!     unwind::trace(|registers| {
//!         pcs.push(registers.pc());
//!         true
//!     }).unwrap();
//!
//!     // Resolve addresses into symbols and display.
//!     for pc in pcs {
//!         println!("{:#x}:", pc);
//!         backtrace::resolve(pc as _, |s| {
//!             println!("    {:?}", s.name());
//!         });
//!     }
//! }
//! ```
//!
//! Sample output:
//! ```text
//! 0x10257e168:
//!     Some(simple::main::h96cb5b684bfd5074)
//! 0x10257e6f8:
//!     Some(core::ops::function::FnOnce::call_once::h05ee70444f3bf8f8)
//! 0x1025800dc:
//!     Some(std::sys_common::backtrace::__rust_begin_short_backtrace::h5d1c209f3f13fbb0)
//! 0x10257e124:
//!     Some(std::rt::lang_start::{{closure}}::h6f7f83facaf9e8d5)
//! 0x102643cbc:
//!     Some(core::ops::function::impls::<impl core::ops::function::FnOnce<A> for &F>::call_once::h10f2582b16e2b13c)
//!     Some(std::panicking::try::do_call::hd3dfc31f9ced2f42)
//!     Some(std::panicking::try::h584945b02ec0e15d)
//!     Some(std::panic::catch_unwind::h1138cecd37279bb6)
//!     Some(std::rt::lang_start_internal::{{closure}}::hf94f7401539e24a6)
//!     Some(std::panicking::try::do_call::ha8b5def05088e3d3)
//!     Some(std::panicking::try::h3ce579dae5f3a6fb)
//!     Some(std::panic::catch_unwind::h29ecbe0d385e9017)
//!     Some(std::rt::lang_start_internal::h35c587f98e9244f6)
//! 0x10257e0f0:
//!     Some(std::rt::lang_start::ha773ed231ef1ec5d)
//! 0x10257e37c:
//!     None
//! 0x102a890f4:
//! 0xa020800000000000:
//! ```
//!
//! For more examples, please refer to ../examples/.

#[cfg(all(target_arch = "x86_64", target_os = "macos"))]
mod compact;
mod cursor;
#[cfg(not(all(target_arch = "aarch64", target_os = "macos")))]
mod dwarf;
#[cfg(not(all(target_arch = "aarch64", target_os = "macos")))]
mod dyld;
mod registers;
mod utils;

pub use cursor::UnwindCursor;
pub use registers::{unwind_init_registers, Registers};

/// A result type that wraps [Error].
pub type Result<T> = std::result::Result<T, Error>;

/// Error definition.
#[derive(thiserror::Error, Debug, Copy, Clone)]
pub enum Error {
    #[cfg(not(all(target_arch = "aarch64", target_os = "macos")))]
    #[error("{0}")]
    Dwarf(#[from] dwarf::DwarfError),

    #[error("invalid ucontext")]
    InvalidUcontext,
}

/// Inspects the current call-stack, passing all active frames into the closure
/// provided to calculate a stack trace.
///
/// The closure's return value is an indication of whether the backtrace should
/// continue. A return value of `false` will terminate the backtrace and return
/// immediately.
#[inline(never)]
pub fn trace<F>(mut f: F) -> Result<bool>
where
    F: FnMut(&Registers) -> bool,
{
    let mut registers = Registers::default();
    unsafe {
        unwind_init_registers(&mut registers as _);
    }
    let mut cursor = UnwindCursor::new();
    // Step directly, so that we can skip the current function (`unwind::trace`).
    while cursor.step(&mut registers)? {
        if !f(&registers) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Inspects the call-stack from `ucontext`, passing all active frames into the closure
/// provided to calculate a stack trace.
///
/// The closure's return value is an indication of whether the backtrace should
/// continue. A return value of `false` will terminate the backtrace and return
/// immediately.
pub fn trace_from_ucontext<F>(ucontext: *mut libc::c_void, mut f: F) -> Result<bool>
where
    F: FnMut(&Registers) -> bool,
{
    if let Some(mut registers) = Registers::from_ucontext(ucontext) {
        // Since our backtracking starts from ucontext, we need to
        // call `f` once before `step`.
        if !f(&registers) {
            return Ok(false);
        }
        let mut cursor = UnwindCursor::new();
        while cursor.step(&mut registers)? {
            if !f(&registers) {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    Err(Error::InvalidUcontext)
}
