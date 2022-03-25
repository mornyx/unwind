use unwind::{unwind_init_registers, Registers, UnwindCursor};

#[test]
fn test_unwind_cursor() {
    let pcs = func1();
    assert!(pcs.len() > 3);
    let mut names = vec![];
    for pc in pcs {
        backtrace::resolve(pc as _, |s| {
            names.push(s.name().unwrap().as_str().unwrap().to_string());
        })
    }
    assert!(names.len() > 3);
    assert!(names[0].contains("func3"));
    assert!(names[1].contains("func2"));
    assert!(names[2].contains("func1"));
    assert!(names[3].contains("test_unwind_cursor"));
}

#[inline(always)]
fn func1() -> Vec<u64> {
    func2()
}

#[inline(never)]
fn func2() -> Vec<u64> {
    func3()
}

#[inline(never)]
fn func3() -> Vec<u64> {
    let mut registers = Registers::default();
    unsafe {
        unwind_init_registers(&mut registers as _);
    }
    let mut pcs = vec![registers.pc()];
    let mut cursor = UnwindCursor::new();
    while cursor.step(&mut registers).unwrap() {
        pcs.push(registers.pc());
    }
    pcs
}
