use crate::UnwindContext;

#[derive(Debug)]
pub struct UnwindCursor {
    context: UnwindContext,
}

impl UnwindCursor {
    #[inline]
    pub fn new(context: UnwindContext) -> Self {
        Self { context }
    }

    pub fn step(&mut self) -> Option<&UnwindContext> {
        if self.context.fp == 0 {
            return None;
        }
        unsafe {
            self.context.pc = *std::mem::transmute::<_, *const u64>(self.context.fp + 8);
            self.context.fp = *std::mem::transmute::<_, *const u64>(self.context.fp);
        }
        Some(&self.context)
    }
}
