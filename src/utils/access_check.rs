use std::mem::MaybeUninit;

thread_local! {
    static CAN_ACCESS_PIPE: [libc::c_int; 2] = {
        unsafe {
            let mut fds = MaybeUninit::<[libc::c_int; 2]>::uninit();
            let res = create_pipe(fds.as_mut_ptr() as *mut libc::c_int);
            if res == 0 {
                [fds.assume_init()[0], fds.assume_init()[1]]
            } else {
                [-1, -1]
            }
        }
    };
}

/// Check whether the target address is valid.
pub fn can_access(address: u64) -> bool {
    CAN_ACCESS_PIPE.with(|pipes| unsafe {
        // The pipe initialization failed at that time.
        if pipes[0] == -1 || pipes[1] == -1 {
            return false;
        }
        // Clear data that already exists in the pipe.
        let mut buffer = [0u8; 8];
        let can_read = loop {
            let size = libc::read(pipes[0], buffer.as_mut_ptr() as _, buffer.len() as _);
            if size == -1 {
                match errno() {
                    libc::EINTR => continue,
                    libc::EAGAIN => break true,
                    _ => break false,
                }
            } else if size > 0 {
                break true;
            }
        };
        if !can_read {
            return false;
        }
        // Try to write "data" to the pipe, let the kernel access the address, if
        // the address is invalid, we will fail the write.
        loop {
            let size = libc::write(pipes[1], address as _, 1);
            if size == -1 {
                match errno() {
                    libc::EINTR => continue,
                    libc::EAGAIN => break true,
                    _ => break false,
                }
            } else if size > 0 {
                break true;
            }
        }
    })
}

#[inline]
#[cfg(target_os = "linux")]
unsafe fn create_pipe(fds: *mut libc::c_int) -> libc::c_int {
    libc::pipe2(fds, libc::O_CLOEXEC | libc::O_NONBLOCK)
}

#[cfg(target_os = "macos")]
unsafe fn create_pipe(fds: *mut libc::c_int) -> libc::c_int {
    let res = libc::pipe(fds);
    if res != 0 {
        return res;
    }
    let fds = fds as *mut [libc::c_int; 2];
    for n in 0..2 {
        let mut flags = libc::fcntl((*fds)[n], libc::F_GETFD);
        flags |= libc::O_CLOEXEC;
        let res = libc::fcntl((*fds)[n], libc::F_SETFD, flags);
        if res != 0 {
            return res;
        }
        let mut flags = libc::fcntl((*fds)[n], libc::F_GETFL);
        flags |= libc::O_NONBLOCK;
        let res = libc::fcntl((*fds)[n], libc::F_SETFL, flags);
        if res != 0 {
            return res;
        }
    }
    0
}

#[inline]
#[cfg(target_os = "linux")]
fn errno() -> libc::c_int {
    unsafe { (*libc::__errno_location()) as libc::c_int }
}

#[inline]
#[cfg(target_os = "macos")]
fn errno() -> libc::c_int {
    unsafe { (*libc::__error()) as libc::c_int }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_access() {
        let v1 = 1;
        let v2 = Box::new(1);
        assert!(can_access(&v1 as *const i32 as u64));
        assert!(can_access(v2.as_ref() as *const i32 as u64));
        assert!(!can_access(0));
        assert!(!can_access(u64::MAX));
    }
}
