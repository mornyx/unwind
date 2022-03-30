#[cfg(target_os = "linux")]
mod maps;
#[cfg(target_os = "linux")]
pub use maps::*;

/// [start, end)
#[derive(Debug, Copy, Clone)]
pub struct AddressRange {
    pub start: u64,
    pub end: u64,
}

impl AddressRange {
    /// Determine whether the target address is in the current range.
    #[inline]
    #[cfg_attr(all(target_os = "macos", target_arch = "aarch64"), allow(unused))]
    pub fn contains(&self, target: u64) -> bool {
        self.start <= target && target < self.end
    }
}

/// Load the value at the `loc` address.
///
/// Note that although `load` is not unsafe, it is implemented by unsafe
/// internally and simply attempts to read the specified address. So the
/// correctness of the address needs to be guaranteed by the caller.
#[inline]
pub fn load<T: Copy>(address: u64) -> T {
    unsafe { *(address as *const T) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_range() {
        let ar = AddressRange { start: 1, end: 3 };
        assert!(ar.contains(1));
        assert!(ar.contains(2));
        assert!(!ar.contains(0));
        assert!(!ar.contains(3));
    }

    #[test]
    fn test_load() {
        let val = i8::MIN;
        let loc = &val as *const i8 as u64;
        assert_eq!(load::<i8>(loc), val);
        let val = u64::MAX;
        let loc = &val as *const u64 as u64;
        assert_eq!(load::<u64>(loc), val);
    }
}
