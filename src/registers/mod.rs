mod consts;
pub use consts::*;

#[cfg(target_arch = "x86_64")]
mod x64;
#[cfg(target_arch = "x86_64")]
pub use x64::*;

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "aarch64")]
pub use aarch64::*;

extern "C" {
    /// Get the register context of the current thread stack and save it in `Registers`.
    ///
    /// The implementation of this function is linked to the assembly code of different
    /// platforms in `src/registers/registers.S`.
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
