use crate::Registers;

extern "C" {
    /// Get the register context of the current thread stack and save it in `Registers`.
    ///
    /// The implementation of this function is linked to the assembly code of different
    /// platforms, such as: `src/linux/x64/registers.S`, `src/macos/aarch64/registers.S`.
    ///
    /// The definition of `Registers` also varies with the OS and ISA.
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
