use gimli::{
    BaseAddresses, EhFrame, EhFrameHdr, EndianSlice, NativeEndian, ParsedEhFrameHdr, Pointer, Reader, UnwindContext,
    UnwindSection, UnwindTable, UnwindTableRow,
};
use smallvec::SmallVec;
use std::slice;

pub const MAX_OBJECTS_LEN: usize = 128;
const PF_X: u32 = 1;

pub fn find_sections() -> SmallVec<[SectionInfo<EndianSlice<'static, NativeEndian>>; MAX_OBJECTS_LEN]> {
    let mut data: SmallVec<[RawSectionInfo; MAX_OBJECTS_LEN]> = SmallVec::new();
    unsafe {
        libc::dl_iterate_phdr(Some(callback), &mut data as *mut _ as *mut libc::c_void);
    }
    let mut sections = SmallVec::new();
    for s in &data {
        unsafe {
            let section_data = slice::from_raw_parts(s.eh_frame_hdr as *const u8, s.eh_frame_hdr_len as _);
            let base_address = BaseAddresses::default()
                .set_eh_frame_hdr(s.eh_frame_hdr)
                .set_text(s.text);
            let eh_frame_hdr = EhFrameHdr::new(section_data, NativeEndian)
                .parse(&base_address, 8)
                .unwrap();
            let eh_frame_ptr = match eh_frame_hdr.eh_frame_ptr() {
                Pointer::Direct(v) => v,
                Pointer::Indirect(p) => *(p as *const u64),
            };
            let base_address = base_address.set_eh_frame(eh_frame_ptr);
            let eh_frame_size = s.max_addr - eh_frame_ptr;
            let eh_frame_data = slice::from_raw_parts(eh_frame_ptr as *const u8, eh_frame_size as _);
            let eh_frame = EhFrame::new(eh_frame_data, NativeEndian);
            sections.push(SectionInfo {
                text: s.text,
                text_len: s.text_len,
                eh_frame_hdr,
                eh_frame,
                base_address,
            });
        }
    }
    sections
}

pub struct SectionInfo<R: Reader> {
    text: u64,
    text_len: u64,
    eh_frame_hdr: ParsedEhFrameHdr<R>,
    eh_frame: EhFrame<R>,
    base_address: BaseAddresses,
}

impl<R: Reader> SectionInfo<R> {
    #[inline]
    pub fn contains(&self, target: u64) -> bool {
        target >= self.text && target - self.text < self.text_len
    }

    pub fn find_unwind_table_row(&self, ctx: &mut UnwindContext<R>, target: u64) -> Option<UnwindTableRow<R>> {
        let Self {
            eh_frame,
            eh_frame_hdr,
            base_address,
            ..
        } = self;
        let tab = match eh_frame_hdr.table() {
            Some(tab) => tab,
            None => return None,
        };
        let fde = match tab.fde_for_address(eh_frame, base_address, target, EhFrame::cie_from_offset) {
            Ok(fde) => fde,
            Err(_) => return None,
        };
        let mut unwind_tab = UnwindTable::new(eh_frame, base_address, ctx, &fde).unwrap();
        while let Some(row) = match unwind_tab.next_row() {
            Ok(v) => v,
            Err(_) => return None,
        } {
            if row.contains(target) {
                return Some(row.clone());
            }
        }
        None
    }
}

#[derive(Default, Debug)]
struct RawSectionInfo {
    text: u64,
    text_len: u64,
    eh_frame_hdr: u64,
    eh_frame_hdr_len: u64,
    max_addr: u64,
}

extern "C" fn callback(info: *mut libc::dl_phdr_info, _size: libc::size_t, data: *mut libc::c_void) -> libc::c_int {
    unsafe {
        let data = data as *mut SmallVec<[RawSectionInfo; MAX_OBJECTS_LEN]>;
        if (*info).dlpi_phnum == 0 {
            return 0;
        }
        let mut section = RawSectionInfo::default();
        let hdrs = slice::from_raw_parts((*info).dlpi_phdr, (*info).dlpi_phnum as usize);
        let mut found_text = false;
        for hdr in hdrs {
            if hdr.p_type == libc::PT_LOAD {
                if hdr.p_flags & PF_X != 0 {
                    section.text = (*info).dlpi_addr + hdr.p_vaddr;
                    section.text_len = hdr.p_memsz;
                    found_text = true;
                }
                let max_addr = (*info).dlpi_addr + hdr.p_vaddr + hdr.p_memsz;
                if section.max_addr < max_addr {
                    section.max_addr = max_addr;
                }
            }
        }
        // PT_GNU_EH_FRAME and PT_ARM_EXIDX are usually near the end. Iterate backward.
        let mut found_unwind = false;
        for hdr in hdrs.iter().rev() {
            if hdr.p_type == libc::PT_GNU_EH_FRAME {
                section.eh_frame_hdr = (*info).dlpi_addr + hdr.p_vaddr;
                section.eh_frame_hdr_len = hdr.p_memsz;
                found_unwind = true;
                break;
            }
        }
        if found_text && found_unwind {
            (*data).push(section);
        }
        0
    }
}
