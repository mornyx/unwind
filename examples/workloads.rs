use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, SIGPROF};
use rand::Rng;
use smallvec::SmallVec;
use unwind::{Registers, UnwindCursor};

const MAX_STACK_DEPTH: usize = 64;

extern "C" {
    fn setitimer(sig: libc::c_int, val: *const libc::itimerval, old: *mut libc::itimerval) -> libc::c_int;
}

fn main() {
    // Register perf signal handler.
    let h = SigHandler::SigAction(perf_signal_handler);
    let a = SigAction::new(h, SaFlags::SA_SIGINFO, SigSet::empty());
    unsafe {
        sigaction(SIGPROF, &a).unwrap();
    }

    // Register SIGPROF that will be triggered periodically.
    unsafe {
        setitimer(libc::ITIMER_PROF, &frequency(1), std::ptr::null_mut());
    }

    // Run some workloads.
    loop {
        let mut rng = rand::thread_rng();
        let mut vec: Vec<i32> = vec![];
        for _ in 0..1000000 {
            vec.push(rng.gen())
        }
        vec.sort();
    }
}

fn frequency(v: i64) -> libc::itimerval {
    let interval = 1e6 as i64 / v;
    let it_interval = libc::timeval {
        tv_sec: interval / 1e6 as i64,
        tv_usec: (interval % 1e6 as i64) as _,
    };
    let it_value = it_interval.clone();
    libc::itimerval { it_interval, it_value }
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
    while cursor.step(&mut registers).unwrap() {
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
}
