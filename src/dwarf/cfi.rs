use crate::dwarf::consts::DW_EH_PE_OMIT;
use crate::dwarf::encoding::*;
use crate::dwarf::{load_with_protect as load, DwarfError};

#[derive(Debug, Default, Copy, Clone)]
pub struct CommonInformationEntry {
    pub cie_start: u64,
    pub cie_length: u64,
    pub cie_instructions: u64,
    pub pointer_encoding: u8,
    pub lsda_encoding: u8,
    pub personality_encoding: u8,
    pub personality_offset_in_cie: u8,
    pub personality: u64,
    pub code_align_factor: u32,
    pub data_align_factor: i32,
    pub is_signal_frame: bool,
    pub fdes_have_augmentation_data: bool,
    pub return_address_register: u8,
    #[cfg(target_arch = "aarch64")]
    pub addresses_signed_with_b_key: bool,
}

impl CommonInformationEntry {
    /// Parse a CIE into a [CommonInformationEntry].
    pub fn decode(start: u64) -> Result<CommonInformationEntry, DwarfError> {
        let mut loc = start;
        let mut cie = CommonInformationEntry::default();
        cie.cie_start = loc;
        cie.lsda_encoding = DW_EH_PE_OMIT;

        // Parse length.
        let mut length = load::<u32>(loc)? as u64;
        loc += 4;
        let mut cie_content_end = loc + length;
        if length == 0xffffffff {
            // 0xffffffff means length is really next 8 bytes.
            length = load::<u64>(loc)?;
            loc += 8;
            cie_content_end = loc + length;
        }
        if length == 0 {
            return Err(DwarfError::CIEZeroLength);
        }

        // CIE ID is always 0.
        let cie_id = load::<u32>(loc)?;
        loc += 4;
        if cie_id != 0 {
            return Err(DwarfError::CIEIdIsNotZero);
        }

        // Version is always 1 or 3.
        let version = load::<u8>(loc)?;
        loc += 1;
        if version != 1 && version != 3 {
            return Err(DwarfError::CIEInvalidVersion(version));
        }

        // Save start of augmentation string and find end.
        let augmentation_str_start = loc;
        while load::<u8>(loc)? != 0 {
            loc += 1;
        }
        loc += 1; // skip '\0'.

        // Parse code alignment factor.
        cie.code_align_factor = decode_uleb128(&mut loc, cie_content_end)? as u32;

        // Parse data alignment factor.
        cie.data_align_factor = decode_sleb128(&mut loc, cie_content_end)? as i32;

        // Parse return address register.
        cie.return_address_register = if version == 1 {
            let r = load::<u8>(loc)?;
            loc += 1;
            r
        } else {
            let r = decode_uleb128(&mut loc, cie_content_end)?;
            assert!(r < 255);
            r as u8
        };

        // Parse augmentation data based on augmentation string.
        let mut n = augmentation_str_start;
        if load::<u8>(n)? == b'z' {
            // Parse augmentation data length.
            let _ = decode_uleb128(&mut loc, cie_content_end);
            while load::<u8>(n)? != 0 {
                match load::<u8>(n)? {
                    b'z' => cie.fdes_have_augmentation_data = true,
                    b'P' => {
                        cie.personality_encoding = load::<u8>(loc)?;
                        loc += 1;
                        cie.personality_offset_in_cie = (loc - start) as u8;
                        cie.personality = decode_pointer(&mut loc, cie_content_end, cie.personality_encoding, 0)?;
                    }
                    b'L' => {
                        cie.lsda_encoding = load::<u8>(loc)?;
                        loc += 1;
                    }
                    b'R' => {
                        cie.pointer_encoding = load::<u8>(loc)?;
                        loc += 1;
                    }
                    b'S' => cie.is_signal_frame = true,
                    #[cfg(target_arch = "aarch64")]
                    b'B' => cie.addresses_signed_with_b_key = true,
                    _ => {}
                }
                n += 1;
            }
        }

        cie.cie_length = cie_content_end - cie.cie_start;
        cie.cie_instructions = loc;
        Ok(cie)
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct FrameDescriptionEntry {
    pub fde_start: u64,
    pub fde_length: u64,
    pub fde_instructions: u64,
    pub pc_start: u64,
    pub pc_end: u64,
    pub lsda: u64,
}

impl FrameDescriptionEntry {
    /// Parse a FDE into a [FrameDescriptionEntry] and a [CommonInformationEntry].
    pub fn decode(start: u64) -> Result<(Self, CommonInformationEntry), DwarfError> {
        let mut loc = start;
        let mut fde = FrameDescriptionEntry::default();
        fde.fde_start = loc;

        // Parse length.
        let mut length = load::<u32>(loc)? as u64;
        loc += 4;
        if length == 0xffffffff {
            // 0xffffffff means length is really next 8 bytes.
            length = load::<u64>(loc)?;
            loc += 8;
        }
        if length == 0 {
            return Err(DwarfError::FDEZeroLength);
        }
        let next_cfi = loc + length;

        // Parse related CIE.
        let cie_ptr = load::<u32>(loc)? as u64;
        if cie_ptr == 0 {
            return Err(DwarfError::FDEIsReallyCIE);
        }
        let cie_start = loc - cie_ptr;
        let cie = CommonInformationEntry::decode(cie_start)?;
        loc += 4;

        // Parse pc begin and range.
        let pc_start = decode_pointer(&mut loc, next_cfi, cie.pointer_encoding, 0)?;
        let pc_range = decode_pointer(&mut loc, next_cfi, cie.pointer_encoding & 0x0F, 0)?;

        // Check for augmentation length.
        if cie.fdes_have_augmentation_data {
            let augmentation_len = decode_uleb128(&mut loc, next_cfi)?;
            let end_of_augmentation = loc + augmentation_len;
            if cie.lsda_encoding != DW_EH_PE_OMIT {
                // Peek at value (without indirection).
                // Zero means no LSDA.
                let lsda_start = loc;
                if decode_pointer(&mut loc, next_cfi, cie.lsda_encoding & 0x0F, 0)? != 0 {
                    // Reset pointer and re-parse LSDA address.
                    loc = lsda_start;
                    fde.lsda = decode_pointer(&mut loc, next_cfi, cie.lsda_encoding, 0)?;
                }
            }
            loc = end_of_augmentation;
        }
        fde.fde_length = next_cfi - start;
        fde.fde_instructions = loc;
        fde.pc_start = pc_start;
        fde.pc_end = pc_start + pc_range;
        Ok((fde, cie))
    }

    /// Determine whether the target address is in the current FDE.
    #[inline]
    pub fn contains(&self, target: u64) -> bool {
        self.pc_start <= target && target < self.pc_end
    }
}

#[derive(Debug)]
pub enum CfiEntry {
    Cie(CommonInformationEntry),
    FdeCie((FrameDescriptionEntry, CommonInformationEntry)),
}

pub struct Entries {
    eh_frame: u64,
    eh_frame_end: u64,
}

impl Entries {
    pub fn new(eh_frame: u64, eh_frame_len: u64) -> Self {
        let eh_frame_end = if eh_frame_len == u64::MAX {
            u64::MAX
        } else {
            eh_frame + eh_frame_len
        };
        Self { eh_frame, eh_frame_end }
    }

    pub fn next(&mut self) -> Result<Option<CfiEntry>, DwarfError> {
        let mut loc = self.eh_frame;
        if loc >= self.eh_frame_end {
            return Ok(None);
        }

        // Parse length.
        let mut cfi_length = load::<u32>(loc)? as u64;
        loc += 4;
        if cfi_length == 0xffffffff {
            // 0xffffffff means length is really next 8 bytes.
            cfi_length = load::<u64>(loc)?;
            loc += 8;
        }
        if cfi_length == 0 {
            // Zero terminator.
            return Ok(None);
        }

        // Parse CIE ID.
        let cie_id = load::<u32>(loc)?;
        if cie_id == 0 {
            // Parse CIE.
            let cie = CommonInformationEntry::decode(self.eh_frame)?;
            self.eh_frame += cie.cie_length;
            Ok(Some(CfiEntry::Cie(cie)))
        } else {
            // Parse FDE & related CIE.
            let (fde, cie) = FrameDescriptionEntry::decode(self.eh_frame)?;
            self.eh_frame += fde.fde_length;
            Ok(Some(CfiEntry::FdeCie((fde, cie))))
        }
    }
}

/// Full scan an .eh_frame section to find a FDE for a pc.
pub fn scan(
    eh_frame: u64,
    eh_frame_len: u64,
    target: u64,
) -> Result<(FrameDescriptionEntry, CommonInformationEntry), DwarfError> {
    let mut entries = Entries::new(eh_frame, eh_frame_len);
    while let Some(entry) = entries.next()? {
        match entry {
            CfiEntry::Cie(_) => {}
            CfiEntry::FdeCie((fde, cie)) => {
                if fde.contains(target) {
                    return Ok((fde, cie));
                }
            }
        }
    }
    Err(DwarfError::FDENotFound)
}
