use crate::Registers;

extern "C" {
    pub fn unwind_init_registers(registers: *mut Registers);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unwind_init_registers() {
        let mut registers = Registers::default();
        unsafe {
            unwind_init_registers(&mut registers as _);
        };
        assert!(registers.pc() > 0);
        assert!(registers.sp() > 0);
    }
}
