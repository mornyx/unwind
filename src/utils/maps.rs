use crate::utils::AddressRange;
use byteorder::ReadBytesExt;
use smallvec::SmallVec;
use std::ffi::CStr;
use std::fs::File;
use std::io;
use std::io::ErrorKind;

const MAX_MAPS_LEN: usize = 256;
const READ_BUFFER_SIZE: usize = 1024;

thread_local! {
    static MAPS: SmallVec<[AddressRange; MAX_MAPS_LEN]> = MapsReader::open().unwrap().read_maps().unwrap();
}

/// Determine whether the target address is readable.
pub fn address_is_readable(target: u64) -> bool {
    MAPS.with(|maps| {
        for m in maps {
            if m.contains(target) {
                return true;
            }
        }
        false
    })
}

struct MapsReader {
    file: File,
    buffer: [u8; READ_BUFFER_SIZE],
}

impl MapsReader {
    fn open() -> io::Result<Self> {
        unsafe {
            let mut buffer = [0u8; READ_BUFFER_SIZE];
            libc::sprintf(
                &mut buffer[0] as *mut u8 as _,
                "/proc/%d/task/%d/maps\0".as_ptr() as _,
                libc::getpid(),
                libc::gettid(),
            );
            let filename = CStr::from_ptr(&buffer[0] as *const u8 as _).to_str().unwrap();
            let file = File::open(filename)?;
            Ok(Self { file, buffer })
        }
    }

    fn read_maps(&mut self) -> io::Result<SmallVec<[AddressRange; MAX_MAPS_LEN]>> {
        let mut v = SmallVec::new();
        while v.len() < MAX_MAPS_LEN {
            // There may be nothing to read here, UnexpectedEof is considered
            // to have completed the parsing normally.
            if let Err(err) = self.read_into_buffer_until(b'-') {
                if err.kind() == ErrorKind::UnexpectedEof {
                    break;
                }
                return Err(err);
            }
            let s = self.str_in_buffer();
            let start = u64::from_str_radix(s, 16).unwrap();
            self.read_into_buffer_until(b' ')?;
            let s = self.str_in_buffer();
            let end = u64::from_str_radix(s, 16).unwrap();
            let mut readable = false;
            self.read_then_callback_until(b' ', |c| {
                if c == b'r' {
                    readable = true;
                }
            })?;
            // The last line may not have b'\n', and UnexpectedEof is
            // considered to be parsed normally.
            if let Err(err) = self.skip_util(b'\n') {
                if err.kind() == ErrorKind::UnexpectedEof {
                    break;
                }
                return Err(err);
            }
            if readable {
                v.push(AddressRange { start, end });
            }
        }
        Ok(v)
    }

    fn read_into_buffer_until(&mut self, target: u8) -> io::Result<()> {
        let mut index = 0;
        loop {
            let c = self.file.read_u8()?;
            if c == target {
                self.buffer[index] = 0;
                break;
            } else {
                self.buffer[index] = c;
            }
            index += 1;
        }
        Ok(())
    }

    fn read_then_callback_until(&mut self, target: u8, mut f: impl FnMut(u8)) -> io::Result<()> {
        loop {
            let c = self.file.read_u8()?;
            if c == target {
                break;
            }
            f(c);
        }
        Ok(())
    }

    fn skip_util(&mut self, target: u8) -> io::Result<()> {
        while self.file.read_u8()? != target {}
        Ok(())
    }

    fn str_in_buffer(&self) -> &str {
        unsafe { CStr::from_ptr(&self.buffer[0] as *const u8 as _).to_str().unwrap() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_maps() -> io::Result<()> {
        let maps = MapsReader::open()?.read_maps()?;
        assert!(maps.len() > 0);
        assert!(maps.len() < MAX_MAPS_LEN - 1);
        Ok(())
    }

    #[test]
    fn test_address_is_readable() {
        let v = 0;
        assert!(address_is_readable(&v as *const i32 as u64));
        assert!(!address_is_readable(0));
        assert!(!address_is_readable(0xffffffffffffffff));
    }
}
