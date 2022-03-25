use crate::dwarf::cfi::{CommonInformationEntry, FrameDescriptionEntry};
use crate::dwarf::encoding::*;
use crate::dwarf::DwarfError;
use std::mem;

#[repr(C)]
#[derive(Debug)]
struct RawEhFrameHeader {
    version: u8,
    eh_frame_ptr_enc: u8,
    fde_count_enc: u8,
    table_enc: u8,
    // eh_frame_ptr: encoded
    // fde_count: encoded
    // table: [[encoded; 2]] // binary search table of: [ initial_location | address ]
}

/// Information encoded in the .eh_frame_hdr section.
#[derive(Debug)]
pub struct EhFrameHeader {
    pub start: u64,
    pub end: u64,
    pub eh_frame: u64,
    pub fde_count: usize,
    pub table: u64,
    pub table_enc: u8,
}

// impl std::fmt::Debug for EhFrameHeader {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let &Self {
//             start,
//             end,
//             eh_frame,
//             fde_count,
//             table,
//             table_enc,
//         } = self;
//         write!(
//             f,
//             "EhFrameHeader {{ start={:#x}, end={:#x}, eh_frame={:#x}, fde_count={}, table={:#x}, table_enc={:#x} }}\n",
//             start, end, eh_frame, fde_count, table, table_enc
//         )?;
//         let entry_size = decode_table_entry_size(self.table_enc);
//         for n in 0..fde_count {
//             let mut entry_loc = table + (n * entry_size) as u64;
//             let pc = decode_pointer(&mut entry_loc, end, table_enc, start);
//             let fde = decode_pointer(&mut entry_loc, end, table_enc, start);
//             write!(f, "    pc={:#x}, fde={:#x}\n", pc, fde)?;
//         }
//         Ok(())
//     }
// }

impl EhFrameHeader {
    pub fn decode(start: u64, end: u64) -> Result<Self, DwarfError> {
        let mut loc = start;
        let raw = unsafe { mem::transmute::<_, &RawEhFrameHeader>(loc as *const u8) };
        loc += mem::size_of::<RawEhFrameHeader>() as u64;
        if raw.version != 1 {
            return Err(DwarfError::HeaderInvalidVersion(raw.version));
        }
        let eh_frame = decode_pointer(&mut loc, end, raw.eh_frame_ptr_enc, start);
        let fde_count = if raw.fde_count_enc != DW_EH_PE_OMIT {
            decode_pointer(&mut loc, end, raw.fde_count_enc, start)
        } else {
            0
        };
        Ok(Self {
            start,
            end,
            eh_frame,
            fde_count: fde_count as usize,
            table: loc,
            table_enc: raw.table_enc,
        })
    }

    pub fn search(&self, target: u64) -> Result<(FrameDescriptionEntry, CommonInformationEntry), DwarfError> {
        let &Self {
            start,
            end,
            fde_count,
            table,
            table_enc,
            ..
        } = self;
        let entry_size = match table_enc & 0b1111 {
            DW_EH_PE_OMIT => 0,
            DW_EH_PE_UDATA2 | DW_EH_PE_SDATA2 => 4,
            DW_EH_PE_UDATA4 | DW_EH_PE_SDATA4 => 8,
            DW_EH_PE_UDATA8 | DW_EH_PE_SDATA8 => 16,
            _ => unreachable!(),
        };
        let mut low = 0;
        let mut len = fde_count;
        while len > 1 {
            let mid = low + (len / 2);
            let mut entry_loc = table + (mid * entry_size) as u64;
            let entry_target = decode_pointer(&mut entry_loc, end, table_enc, start);
            if entry_target == target {
                low = mid;
                break;
            } else if entry_target < target {
                low = mid;
                len -= len / 2;
            } else {
                len /= 2;
            }
        }
        let mut entry_loc = table + (low * entry_size) as u64;
        let _ = decode_pointer(&mut entry_loc, end, table_enc, start);
        let fde = decode_pointer(&mut entry_loc, end, table_enc, start);
        match FrameDescriptionEntry::decode(fde) {
            Ok((fde, cie)) => {
                if target < fde.pc_start || target >= fde.pc_end {
                    Err(DwarfError::FDENotFound)
                } else {
                    Ok((fde, cie))
                }
            }
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn test_decode_header() {
        let sects = crate::dyld::sections();
        assert!(sects.len() > 0);
        for s in sects {
            let hdr_end = s.eh_frame_hdr + s.eh_frame_hdr_len;
            let hdr = EhFrameHeader::decode(s.eh_frame_hdr, hdr_end).unwrap();
            assert!(hdr.eh_frame > 0);
        }
    }
}
