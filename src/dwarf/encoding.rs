use crate::dwarf::consts::*;
use crate::dwarf::{load_with_protect as load, DwarfError};

/// Decode a Pointer-Encoding value.
pub fn decode_pointer(loc: &mut u64, end: u64, enc: u8, datarel_base: u64) -> Result<u64, DwarfError> {
    // Calculate relative offset.
    let offset = match enc & 0b1110000 {
        DW_EH_PE_ABSPTR => 0, // do nothing
        DW_EH_PE_PCREL => *loc,
        DW_EH_PE_DATAREL => {
            // DW_EH_PE_DATAREL is only valid in a few places, so the parameter has a
            // default value of 0, and we return an error in the event that someone calls
            // this function with a datarelBase of 0 and DW_EH_PE_DATAREL encoding.
            if datarel_base == 0 {
                return Err(DwarfError::InvalidDataRelBase);
            }
            datarel_base
        }
        v => return Err(DwarfError::InvalidPointerEncodingOffset(v)),
    };

    // Get value.
    let mut res = match enc & 0b1111 {
        DW_EH_PE_PTR => {
            let v = load::<u64>(*loc)?;
            *loc += 8;
            v + offset
        }
        DW_EH_PE_ULEB128 => decode_uleb128(loc, end)? + offset,
        DW_EH_PE_UDATA2 => {
            let v = load::<u16>(*loc)? as u64;
            *loc += 2;
            v + offset
        }
        DW_EH_PE_UDATA4 => {
            let v = load::<u32>(*loc)? as u64;
            *loc += 4;
            v + offset
        }
        DW_EH_PE_UDATA8 => {
            let v = load::<u64>(*loc)?;
            *loc += 8;
            v + offset
        }
        DW_EH_PE_SLEB128 => {
            let v = decode_sleb128(loc, end)?;
            if v > 0 {
                v as u64 + offset
            } else {
                offset - ((-v) as u64)
            }
        }
        DW_EH_PE_SDATA2 => {
            let v = load::<i16>(*loc)?;
            *loc += 2;
            if v > 0 {
                v as u64 + offset
            } else {
                offset - ((-v) as u64)
            }
        }
        DW_EH_PE_SDATA4 => {
            let v = load::<i32>(*loc)?;
            *loc += 4;
            if v > 0 {
                v as u64 + offset
            } else {
                offset - ((-v) as u64)
            }
        }
        DW_EH_PE_SDATA8 => {
            let v = load::<i64>(*loc)?;
            *loc += 8;
            if v > 0 {
                v as u64 + offset
            } else {
                offset - ((-v) as u64)
            }
        }
        v => return Err(DwarfError::InvalidPointerEncodingValue(v)),
    };

    // Dereference the pointer if necessary.
    if enc & DW_EH_PE_INDIRECT != 0 {
        res = load::<u64>(res)?;
    }
    Ok(res)
}

/// Read a ULEB128 into a 64-bit word.
pub fn decode_uleb128(loc: &mut u64, end: u64) -> Result<u64, DwarfError> {
    let mut res = 0u64;
    let mut bit = 0u64;
    loop {
        if *loc == end {
            return Err(DwarfError::TruncatedUleb128Expression(*loc));
        }
        let b = (load::<u8>(*loc)? & 0b1111111) as u64;
        if bit >= 64 || b << bit >> bit != b {
            return Err(DwarfError::MalformedUleb128Expression(*loc));
        }
        res |= b << bit;
        bit += 7;
        let brk = load::<u8>(*loc)? < 0b10000000;
        *loc += 1;
        if brk {
            break;
        }
    }
    Ok(res)
}

/// Read a SLEB128 into a 64-bit word.
pub fn decode_sleb128(loc: &mut u64, end: u64) -> Result<i64, DwarfError> {
    let mut res = 0i64;
    let mut bit = 0u64;
    let mut byte;
    loop {
        if *loc == end {
            return Err(DwarfError::TruncatedSleb128Expression(*loc));
        }
        byte = load::<u8>(*loc)?;
        *loc += 1;
        res |= (((byte & 0b1111111) as u64) << bit) as i64;
        bit += 7;
        if (byte & 0b10000000) == 0 {
            break;
        }
    }
    // sign extend negative numbers
    if (byte & 0x40) != 0 && bit < 64 {
        res |= (u64::MAX << bit) as i64;
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_uleb128() {
        let mut buf = Vec::new();
        let len = leb128::write::unsigned(&mut buf, 0).unwrap();
        assert_eq!(len, buf.len());
        let start = buf.as_ptr() as u64;
        let mut loc = start;
        assert_eq!(decode_uleb128(&mut loc, start + len as u64).unwrap(), 0);
        assert_eq!(loc - start, len as u64);

        let mut buf = Vec::new();
        let len = leb128::write::unsigned(&mut buf, 0x12345678).unwrap();
        assert_eq!(len, buf.len());
        let start = buf.as_ptr() as u64;
        let mut loc = start;
        assert_eq!(decode_uleb128(&mut loc, start + len as u64).unwrap(), 0x12345678);
        assert_eq!(loc - start, len as u64);

        let mut buf = Vec::new();
        let len = leb128::write::unsigned(&mut buf, u64::MAX).unwrap();
        assert_eq!(len, buf.len());
        let start = buf.as_ptr() as u64;
        let mut loc = start;
        assert_eq!(decode_uleb128(&mut loc, start + len as u64).unwrap(), u64::MAX);
        assert_eq!(loc - start, len as u64);
    }

    #[test]
    fn test_decode_sleb128() {
        let mut buf = Vec::new();
        let len = leb128::write::signed(&mut buf, 0).unwrap();
        assert_eq!(len, buf.len());
        let start = buf.as_ptr() as u64;
        let mut loc = start;
        assert_eq!(decode_sleb128(&mut loc, start + len as u64).unwrap(), 0);
        assert_eq!(loc - start, len as u64);

        let mut buf = Vec::new();
        let len = leb128::write::signed(&mut buf, 0x12345678).unwrap();
        assert_eq!(len, buf.len());
        let start = buf.as_ptr() as u64;
        let mut loc = start;
        assert_eq!(decode_sleb128(&mut loc, start + len as u64).unwrap(), 0x12345678);
        assert_eq!(loc - start, len as u64);

        let mut buf = Vec::new();
        let len = leb128::write::signed(&mut buf, i64::MAX).unwrap();
        assert_eq!(len, buf.len());
        let start = buf.as_ptr() as u64;
        let mut loc = start;
        assert_eq!(decode_sleb128(&mut loc, start + len as u64).unwrap(), i64::MAX);
        assert_eq!(loc - start, len as u64);

        let mut buf = Vec::new();
        let len = leb128::write::signed(&mut buf, i64::MIN).unwrap();
        assert_eq!(len, buf.len());
        let start = buf.as_ptr() as u64;
        let mut loc = start;
        assert_eq!(decode_sleb128(&mut loc, start + len as u64).unwrap(), i64::MIN);
        assert_eq!(loc - start, len as u64);
    }

    #[test]
    fn test_decode_pointer() {
        let enc = DW_EH_PE_ABSPTR | DW_EH_PE_PTR;
        let val = u64::MAX;
        let mut loc = &val as *const u64 as u64;
        let start = loc;
        assert_eq!(decode_pointer(&mut loc, u64::MAX, enc, 0).unwrap(), val);
        assert_eq!(loc, start + 8);

        let enc = DW_EH_PE_ABSPTR | DW_EH_PE_UDATA2;
        let val = u16::MAX;
        let mut loc = &val as *const u16 as u64;
        let start = loc;
        assert_eq!(decode_pointer(&mut loc, u64::MAX, enc, 0).unwrap() as u16, val);
        assert_eq!(loc, start + 2);

        let enc = DW_EH_PE_ABSPTR | DW_EH_PE_UDATA4;
        let val = u32::MAX;
        let mut loc = &val as *const u32 as u64;
        let start = loc;
        assert_eq!(decode_pointer(&mut loc, u64::MAX, enc, 0).unwrap() as u32, val);
        assert_eq!(loc, start + 4);

        let enc = DW_EH_PE_ABSPTR | DW_EH_PE_UDATA8;
        let val = u64::MAX;
        let mut loc = &val as *const u64 as u64;
        let start = loc;
        assert_eq!(decode_pointer(&mut loc, u64::MAX, enc, 0).unwrap(), val);
        assert_eq!(loc, start + 8);

        let enc = DW_EH_PE_ABSPTR | DW_EH_PE_SDATA2;
        let val = i16::MAX;
        let mut loc = &val as *const i16 as u64;
        let start = loc;
        assert_eq!(decode_pointer(&mut loc, u64::MAX, enc, 0).unwrap() as i16, val);
        assert_eq!(loc, start + 2);

        let enc = DW_EH_PE_ABSPTR | DW_EH_PE_SDATA4;
        let val = i32::MAX;
        let mut loc = &val as *const i32 as u64;
        let start = loc;
        assert_eq!(decode_pointer(&mut loc, u64::MAX, enc, 0).unwrap() as i32, val);
        assert_eq!(loc, start + 4);

        let enc = DW_EH_PE_ABSPTR | DW_EH_PE_SDATA8;
        let val = i64::MAX;
        let mut loc = &val as *const i64 as u64;
        let start = loc;
        assert_eq!(decode_pointer(&mut loc, u64::MAX, enc, 0).unwrap() as i64, val);
        assert_eq!(loc, start + 8);

        let enc = DW_EH_PE_PCREL | DW_EH_PE_PTR;
        let val = 0x123;
        let mut loc = &val as *const u64 as u64;
        let start = loc;
        assert_eq!(decode_pointer(&mut loc, u64::MAX, enc, 0).unwrap(), start + val);
        assert_eq!(loc, start + 8);

        let enc = DW_EH_PE_DATAREL | DW_EH_PE_PTR;
        let val = 0x123;
        let mut loc = &val as *const u64 as u64;
        let start = loc;
        assert_eq!(decode_pointer(&mut loc, u64::MAX, enc, 0x456).unwrap(), val + 0x456);
        assert_eq!(loc, start + 8);

        let enc = DW_EH_PE_ABSPTR | DW_EH_PE_PTR | DW_EH_PE_INDIRECT;
        let val = u64::MAX;
        let loc = &val as *const u64 as u64;
        let mut loc2 = &loc as *const u64 as u64;
        let start = loc2;
        assert_eq!(decode_pointer(&mut loc2, u64::MAX, enc, 0).unwrap(), val);
        assert_eq!(loc2, start + 8);
    }

    #[test]
    fn test_decode_pointer_negative_datarel() {
        let enc = DW_EH_PE_DATAREL | DW_EH_PE_SDATA4;
        let val = -1;
        let mut loc = &val as *const i32 as u64;
        let start = loc;
        assert_eq!(decode_pointer(&mut loc, u64::MAX, enc, 0x456).unwrap(), 0x455);
        assert_eq!(loc, start + 4);
    }
}
