use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, SIGPROF};
use unwind::{Registers, UnwindCursor};

fn main() {
    let h = SigHandler::SigAction(perf_signal_handler);
    let a = SigAction::new(h, SaFlags::SA_SIGINFO, SigSet::empty());
    unsafe {
        sigaction(SIGPROF, &a).unwrap();
        libc::kill(libc::getpid(), libc::SIGPROF);
    }
    loop {}
}

#[no_mangle]
pub extern "C" fn perf_signal_handler(_: libc::c_int, _: *mut libc::siginfo_t, ucontext: *mut libc::c_void) {
    let mut registers = Registers::from_ucontext(ucontext).unwrap();
    show(registers.pc());
    let mut cursor = UnwindCursor::new();
    while cursor.step(&mut registers) {
        show(registers.pc());
    }
    std::process::exit(0);
}

fn show(pc: u64) {
    println!("{:#x}:", pc);
    backtrace::resolve(pc as _, |s| {
        println!("    {:?}", s.name());
    });
}
