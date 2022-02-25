use unwind::{init_unwind_context, UnwindContext};

fn main() {
    let context = unsafe {
        let mut context = UnwindContext::default();
        init_unwind_context(&mut context as _);
        context
    };
    println!("{:#x}", context.pc);
    unsafe {
        backtrace::resolve(std::mem::transmute(context.pc), |s| {
            println!("{:?}", s.name());
        });
    }
}
