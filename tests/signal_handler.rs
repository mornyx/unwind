use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, SIGPROF};
use rand::Rng;
use smallvec::SmallVec;
use std::sync::atomic::{AtomicU32, Ordering};

const MAX_STACK_DEPTH: usize = 64;
const MAX_SAMPLE_COUNT: u32 = 500; // about 5s (99Hz)

static SAMPLE_COUNT: AtomicU32 = AtomicU32::new(0);

#[test]
fn test_unwind_in_signal_handler() {
    let h = SigHandler::SigAction(perf_signal_handler);
    let a = SigAction::new(h, SaFlags::SA_SIGINFO, SigSet::empty());
    unsafe {
        sigaction(SIGPROF, &a).unwrap();
        setitimer(libc::ITIMER_PROF, &frequency(99), std::ptr::null_mut());
    }
    while SAMPLE_COUNT.load(Ordering::SeqCst) <= MAX_SAMPLE_COUNT {
        let mut rng = rand::thread_rng();
        let mut vec: Vec<i32> = vec![];
        for _ in 0..100000 {
            vec.push(rng.gen())
        }
        vec.sort();
    }
    unsafe {
        setitimer(
            libc::ITIMER_PROF,
            &libc::itimerval {
                it_interval: libc::timeval { tv_sec: 0, tv_usec: 0 },
                it_value: libc::timeval { tv_sec: 0, tv_usec: 0 },
            },
            std::ptr::null_mut(),
        );
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

extern "C" {
    fn setitimer(sig: libc::c_int, val: *const libc::itimerval, old: *mut libc::itimerval) -> libc::c_int;
}

#[no_mangle]
extern "C" fn perf_signal_handler(_: libc::c_int, _: *mut libc::siginfo_t, ucontext: *mut libc::c_void) {
    let mut pcs: SmallVec<[u64; MAX_STACK_DEPTH]> = SmallVec::new();
    unwind::trace_from_ucontext(ucontext, |registers| {
        pcs.push(registers.pc());
        true
    })
    .unwrap();
    assert!(pcs.len() > 0);
    SAMPLE_COUNT.fetch_add(1, Ordering::SeqCst);
}
