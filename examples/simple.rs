use unwind::{unwind_init_registers, Registers, UnwindCursor};

fn main() {
    let mut registers = Registers::default();
    unsafe {
        unwind_init_registers(&mut registers as _);
    };
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