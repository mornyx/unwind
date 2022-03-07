use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, SIGPROF};
use smallvec::SmallVec;
use unwind::{Registers, UnwindCursor};

const MAX_STACK_DEPTH: usize = 64;

fn main() {
    // Register perf signal handler.
    let h = SigHandler::SigAction(perf_signal_handler);
    let a = SigAction::new(h, SaFlags::SA_SIGINFO, SigSet::empty());
    unsafe {
        sigaction(SIGPROF, &a).unwrap();
    }

    // Send a SIGPROF signal to the current process.
    unsafe {
        libc::kill(libc::getpid(), libc::SIGPROF);
    }

    // Block until the signal handler finishes executing.
    loop {}
}

#[no_mangle]
pub extern "C" fn perf_signal_handler(_: libc::c_int, _: *mut libc::siginfo_t, ucontext: *mut libc::c_void) {
    // In order to skip the signal frame placed by the kernel, we choose to
    // initialize the registers from ucontext.
    let mut registers = Registers::from_ucontext(ucontext).unwrap();

    // Heap allocations should be avoided in signal handlers, we choose
    // to use SmallVec instead of Vec.
    let mut pcs: SmallVec<[u64; MAX_STACK_DEPTH]> = SmallVec::new();
    pcs.push(registers.pc());

    // Do stack backtrace.
    let mut cursor = UnwindCursor::new();
    while cursor.step(&mut registers) {
        if pcs.len() >= MAX_STACK_DEPTH {
            break;
        }
        pcs.push(registers.pc());
    }

    // Resolve addresses into symbols and display.
    //
    // Usually our resolving happens lazily, we only saves the pc array in
    // the signal handler. This is just a demo, so show it directly.
    for pc in pcs {
        println!("{:#x}:", pc);
        backtrace::resolve(pc as _, |s| {
            println!("    {:?}", s.name());
        });
    }

    std::process::exit(0);
}
