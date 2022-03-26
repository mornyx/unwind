# Unwind

A stack backtrace implementation with the goal of providing stable stack backtraces in signal handler.

## Examples

### Trace
```rust
fn main() {
    let pcs = func1_inlined();

    // Resolve addresses into symbols and display.
    for pc in pcs {
        println!("{:#x}:", pc);
        backtrace::resolve(pc as _, |s| {
            println!("    {:?}", s.name());
        });
    }
}

#[inline(always)]
fn func1_inlined() -> Vec<u64> {
    func2()
}

fn func2() -> Vec<u64> {
    let mut pcs = vec![];
    unwind::trace(|registers| {
        pcs.push(registers.pc());
        true
    }).unwrap();
    pcs
}
```

Sample output:
```text
0xaaaac1df71fc:
    Some(inlined::func2::h79ffdd095d1a256b)
0xaaaac1df7034:
    Some(inlined::func1_inlined::hb58e210bba8ae785)
    Some(inlined::main::h2db4024ddd090f67)
0xaaaac1df6298:
    Some(core::ops::function::FnOnce::call_once::h36e4b3112da7988f)
0xaaaac1df78e0:
    Some(std::sys_common::backtrace::__rust_begin_short_backtrace::h7a1eca5edf0210ff)
0xaaaac1df74f0:
    Some(std::rt::lang_start::{{closure}}::hefc9f83c65bfcfdd)
0xaaaac1ec2a54:
    Some(core::ops::function::impls::<impl core::ops::function::FnOnce<A> for &F>::call_once::hb3d72f37c4f41931)
    Some(std::panicking::try::do_call::h2f1a663a5e990cc5)
    Some(std::panicking::try::he2c301fb013aede0)
    Some(std::panic::catch_unwind::h7c79be909962b8c4)
    Some(std::rt::lang_start_internal::{{closure}}::h9ae40f978a95fb59)
    Some(std::panicking::try::do_call::h6a84ad1fe0995236)
    Some(std::panicking::try::hd485ee1df9c0a65b)
    Some(std::panic::catch_unwind::h1c7e5d249c992e03)
    Some(std::rt::lang_start_internal::h240d372a7b184ad9)
0xaaaac1df74c0:
    Some(std::rt::lang_start::h5cb7a51b1c3bc2ec)
0xaaaac1df72b8:
    Some("main")
0xffff8e6fdd50:
    Some("__libc_start_main")
0xaaaac1df5864:
```

### Trace from ucontext

```rust
use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, SIGPROF};
use smallvec::SmallVec;

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
    // Heap allocations should be avoided in signal handlers, we
    // should use SmallVec instead of Vec.
    let mut pcs: SmallVec<[u64; MAX_STACK_DEPTH]> = SmallVec::new();

    // Do stack backtrace.
    //
    // In order to skip the signal frame placed by the kernel, we
    // should use `trace_from_ucontext`.
    unwind::trace_from_ucontext(ucontext, |registers| {
        if pcs.len() >= MAX_STACK_DEPTH {
            return false;
        }
        pcs.push(registers.pc());
        true
    }).unwrap();

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
```

Sample output:
```text
0xffffa8ba8018:
    Some("kill")
0xaaaac77f7c74:
    Some(signal_handler::main::hb68337f0690c36c5)
0xaaaac77f73d0:
    Some(core::ops::function::FnOnce::call_once::h077c126ec6f1a5dc)
0xaaaac77f7310:
    Some(std::sys_common::backtrace::__rust_begin_short_backtrace::h0610489bfe198ec1)
0xaaaac77f79e4:
    Some(std::rt::lang_start::{{closure}}::h1f439f111af821b7)
0xaaaac78c5a50:
    Some(core::ops::function::impls::<impl core::ops::function::FnOnce<A> for &F>::call_once::hb3d72f37c4f41931)
    Some(std::panicking::try::do_call::h2f1a663a5e990cc5)
    Some(std::panicking::try::he2c301fb013aede0)
    Some(std::panic::catch_unwind::h7c79be909962b8c4)
    Some(std::rt::lang_start_internal::{{closure}}::h9ae40f978a95fb59)
    Some(std::panicking::try::do_call::h6a84ad1fe0995236)
    Some(std::panicking::try::hd485ee1df9c0a65b)
    Some(std::panic::catch_unwind::h1c7e5d249c992e03)
    Some(std::rt::lang_start_internal::h240d372a7b184ad9)
0xaaaac77f79b4:
    Some(std::rt::lang_start::h70906251b152fcb3)
0xaaaac77f7f50:
    Some("main")
0xffffa8b94d50:
    Some("__libc_start_main")
0xaaaac77f5b64:
```
