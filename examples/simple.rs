use unwind::{unwind_init_registers, Registers, UnwindCursor};

fn main() {
    // Get the current register context.
    let mut registers = Registers::default();
    unsafe {
        unwind_init_registers(&mut registers as _);
    };

    // Do stack backtrace.
    let mut pcs = vec![registers.pc()];
    let mut cursor = UnwindCursor::new();
    while cursor.step(&mut registers).unwrap() {
        pcs.push(registers.pc());
    }

    // Resolve addresses into symbols and display.
    for pc in pcs {
        println!("{:#x}:", pc);
        backtrace::resolve(pc as _, |s| {
            println!("    {:?}", s.name());
        });
    }
}

/*
Sample output on macOS+aarch64:

0x10257e168:
    Some(simple::main::h96cb5b684bfd5074)
0x10257e6f8:
    Some(core::ops::function::FnOnce::call_once::h05ee70444f3bf8f8)
0x1025800dc:
    Some(std::sys_common::backtrace::__rust_begin_short_backtrace::h5d1c209f3f13fbb0)
0x10257e124:
    Some(std::rt::lang_start::{{closure}}::h6f7f83facaf9e8d5)
0x102643cbc:
    Some(core::ops::function::impls::<impl core::ops::function::FnOnce<A> for &F>::call_once::h10f2582b16e2b13c)
    Some(std::panicking::try::do_call::hd3dfc31f9ced2f42)
    Some(std::panicking::try::h584945b02ec0e15d)
    Some(std::panic::catch_unwind::h1138cecd37279bb6)
    Some(std::rt::lang_start_internal::{{closure}}::hf94f7401539e24a6)
    Some(std::panicking::try::do_call::ha8b5def05088e3d3)
    Some(std::panicking::try::h3ce579dae5f3a6fb)
    Some(std::panic::catch_unwind::h29ecbe0d385e9017)
    Some(std::rt::lang_start_internal::h35c587f98e9244f6)
0x10257e0f0:
    Some(std::rt::lang_start::ha773ed231ef1ec5d)
0x10257e37c:
    None
0x102a890f4:
0xa020800000000000:
*/
