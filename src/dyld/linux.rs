use lazy_static::lazy_static;
use smallvec::SmallVec;
use std::slice;

const MAX_OBJECTS_LEN: usize = 128;
const PF_X: u32 = 1;

lazy_static! {
    static ref SECTIONS: SmallVec<[SectionInfo; MAX_OBJECTS_LEN]> = init_sections();
}

/// Real loaded addresses of sections in virtual memory space.
#[derive(Default, Debug)]
pub struct SectionInfo {
    pub base: u64,
    pub text: u64,
    pub text_len: u64,
    pub eh_frame_hdr: u64,
    pub eh_frame_hdr_len: u64,
    pub max_addr: u64,
}

impl SectionInfo {
    /// Determine whether the target address is in the current section.
    #[inline]
    pub fn contains(&self, target: u64) -> bool {
        self.text <= target && target < self.text + self.text_len
    }
}

/// Returns a [SectionInfo] list of all libraries dynamically loaded by the current process.
#[inline]
pub fn sections() -> &'static [SectionInfo] {
    &SECTIONS
}

fn init_sections() -> SmallVec<[SectionInfo; MAX_OBJECTS_LEN]> {
    let mut data: SmallVec<[SectionInfo; MAX_OBJECTS_LEN]> = SmallVec::new();
    unsafe {
        libc::dl_iterate_phdr(Some(callback), &mut data as *mut _ as *mut libc::c_void);
    }
    data
}

extern "C" fn callback(info: *mut libc::dl_phdr_info, _size: libc::size_t, data: *mut libc::c_void) -> libc::c_int {
    unsafe {
        let data = data as *mut SmallVec<[SectionInfo; MAX_OBJECTS_LEN]>;
        if (*data).len() >= MAX_OBJECTS_LEN {
            return 0;
        }
        if (*info).dlpi_phnum == 0 {
            return 0;
        }
        match std::ffi::CStr::from_ptr((*info).dlpi_name).to_str() {
            Ok(name) => {
                // If `trace-shared-libs` is not enabled, only functions in the current
                // executable are traced. (The `dlpi_name` of the current executable is
                // an empty string)
                #[cfg(not(feature = "trace-shared-libs"))]
                if name != "" {
                    return 0;
                }
                // After testing, it was found that there is currently no .eh_frame data
                // available on aarch64 for linux-vdso.so. So we just skip it for now.
                #[cfg(feature = "trace-shared-libs")]
                if name.contains("linux-vdso.so") {
                    return 0;
                }
            }
            Err(_) => return 0,
        }
        let mut section = SectionInfo::default();
        section.base = (*info).dlpi_addr;
        let hdrs = slice::from_raw_parts((*info).dlpi_phdr, (*info).dlpi_phnum as usize);
        let mut found_text = false;
        let mut found_unwind = false;
        for hdr in hdrs {
            match hdr.p_type {
                libc::PT_LOAD => {
                    if hdr.p_flags & PF_X != 0 {
                        section.text = (*info).dlpi_addr + hdr.p_vaddr;
                        section.text_len = hdr.p_memsz;
                        found_text = true;
                    }
                    let max_addr = (*info).dlpi_addr + hdr.p_vaddr + hdr.p_filesz;
                    if section.max_addr < max_addr {
                        section.max_addr = max_addr;
                    }
                }
                libc::PT_GNU_EH_FRAME => {
                    section.eh_frame_hdr = (*info).dlpi_addr + hdr.p_vaddr;
                    section.eh_frame_hdr_len = hdr.p_memsz;
                    found_unwind = true;
                }
                _ => {}
            }
        }
        if found_text && found_unwind {
            (*data).push(section);
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sections() {
        assert!(sections().len() > 0);
        assert!(sections().len() <= MAX_OBJECTS_LEN);
    }
}
