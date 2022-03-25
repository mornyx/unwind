pub fn load<T: Copy>(loc: u64) -> T {
    unsafe { *(loc as *const T) }
}

#[cfg(test)]
mod tests {
    use super::*;

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
