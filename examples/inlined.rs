use unwind::{unwind_init_registers, Registers, UnwindCursor};

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
    // Get the current register context.
    let mut registers = Registers::default();
    unsafe {
        unwind_init_registers(&mut registers as _);
    };

    // Do stack backtrace.
    let mut pcs = vec![registers.pc()];
    let mut cursor = UnwindCursor::new();
    while cursor.step(&mut registers) {
        pcs.push(registers.pc());
    }
    pcs
}
