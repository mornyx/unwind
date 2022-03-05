use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, SIGPROF};
use rand::Rng;
use unwind::{Registers, UnwindCursor};

extern "C" {
    fn setitimer(sig: libc::c_int, val: *const libc::itimerval, old: *mut libc::itimerval) -> libc::c_int;
}

fn main() {
    let h = SigHandler::SigAction(perf_signal_handler);
    let a = SigAction::new(h, SaFlags::SA_SIGINFO, SigSet::empty());
    unsafe {
        sigaction(SIGPROF, &a).unwrap();
        setitimer(libc::ITIMER_PROF, &frequency(1), std::ptr::null_mut());
    }
    loop {
        let mut rng = rand::thread_rng();
        let mut vec: Vec<i32> = vec![];
        for _ in 0..1000000 {
            vec.push(rng.gen())
        }
        vec.sort();
    }
}

#[no_mangle]
pub extern "C" fn perf_signal_handler(_: libc::c_int, _: *mut libc::siginfo_t, ucontext: *mut libc::c_void) {
    let mut registers = Registers::from_ucontext(ucontext).unwrap();
    show(registers.pc());
    let mut cursor = UnwindCursor::new();
    while cursor.step(&mut registers) {
        show(registers.pc());
    }
}

fn show(pc: u64) {
    println!("{:#x}:", pc);
    backtrace::resolve(pc as _, |s| {
        println!("    {:?}", s.name());
    });
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
