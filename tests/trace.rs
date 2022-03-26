#[test]
fn test_trace() {
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
    assert!(names[3].contains("test_trace"));
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
    let mut pcs = vec![];
    unwind::trace(|registers| {
        pcs.push(registers.pc());
        true
    })
    .unwrap();
    pcs
}
