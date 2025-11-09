use crate::{posix_num, process};
use libc::c_int;
use structures::{error::LxError, fs::OpenFlags};

/// Gets registered virtual file descriptor by native file descriptor.
pub fn get(fd: c_int) -> Option<u64> {
    process::context().vfd_table.pin().get(&fd).copied()
}

/// Unmaps a native file descriptor to its associated virtual file descriptor.
pub fn take(fd: c_int) -> Option<u64> {
    process::context().vfd_table.pin().remove(&fd).copied()
}

/// Registers a virtual file descriptor to a native file descriptor.
pub fn register(fd: c_int, vfd: u64) {
    let status = process::context().vfd_table.pin().insert(fd, vfd).copied();
    debug_assert!(status.is_none());
}

/// Registers a virtual file descriptor, creating the associated native file descriptor automatically, returning the
/// generated native file descriptor.
pub fn create(vfd: u64, flags: OpenFlags) -> Result<c_int, LxError> {
    let mut apple_flags = libc::O_RDONLY;
    if flags.contains(OpenFlags::O_CLOEXEC) {
        apple_flags |= libc::O_CLOEXEC;
    }

    let fd = unsafe { posix_num!(libc::open(c"/dev/null".as_ptr(), apple_flags))? };
    register(fd, vfd);
    Ok(fd)
}

/// Parses the `init-vfd-table` format, filling to the current process' one.
pub fn fill_table(s: &str) -> Result<(), LxError> {
    for entry in s.split(',') {
        if entry.is_empty() {
            continue;
        }
        let Some((fd, vfd)) = entry.split_once(':') else {
            return Err(LxError::EINVAL);
        };
        let Ok(fd): Result<c_int, _> = fd.parse() else {
            return Err(LxError::EINVAL);
        };
        let Ok(vfd): Result<u64, _> = vfd.parse() else {
            return Err(LxError::EINVAL);
        };
        register(fd, vfd);
    }
    Ok(())
}

/// Exports current process' virtual file descriptor table to the `init-vfd-table` format.
pub fn export_table() -> Result<String, LxError> {
    let mut result = String::new();
    for (&fd, &vfd) in process::context().vfd_table.pin().iter() {
        let fd_flags: c_int = unsafe { posix_num!(libc::fcntl(fd, libc::F_GETFD))? };
        if (fd_flags & libc::FD_CLOEXEC) == 0 {
            result.push_str(&format!("{fd}:{vfd},"));
        }
    }
    Ok(result)
}
