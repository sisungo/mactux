use std::ffi::c_uint;
use structures::{error::LxError, security::UserCap};

pub fn uid() -> c_uint {
    unsafe { libc::getuid() }
}

pub fn euid() -> c_uint {
    unsafe { libc::geteuid() }
}

pub fn suid() -> c_uint {
    // FIXME: macOS doesn't support this suid?
    unsafe { libc::getuid() }
}

pub fn setuid(_uid: c_uint) -> Result<(), LxError> {
    Err(LxError::EPERM)
}

pub fn setfsuid(_uid: c_uint) -> Result<(), LxError> {
    Err(LxError::EPERM)
}

pub fn gid() -> c_uint {
    unsafe { libc::getgid() }
}

pub fn egid() -> c_uint {
    unsafe { libc::getegid() }
}

pub fn sgid() -> c_uint {
    // FIXME: macOS doesn't support this suid?
    unsafe { libc::getgid() }
}

pub fn setgid(_gid: c_uint) -> Result<(), LxError> {
    Err(LxError::EPERM)
}

pub fn setfsgid(_gid: c_uint) -> Result<(), LxError> {
    Err(LxError::EPERM)
}

pub fn groups() -> Vec<c_uint> {
    loop {
        unsafe {
            let n = libc::getgroups(0, std::ptr::null_mut());
            assert_ne!(n, -1);
            let mut buf = vec![0; n as usize];
            let status = libc::getgroups(n, buf.as_mut_ptr());
            if status == -1 {
                continue;
            }
            break buf;
        }
    }
}

pub fn capget(pid: i32) -> Result<UserCap, LxError> {
    if pid == 0 || pid == crate::process::pid() {}
    Err(LxError::ENOSYS)
}

pub fn capset(cap: UserCap) -> Result<(), LxError> {
    if cap.pid == 0 || cap.pid == crate::process::pid() {}
    Err(LxError::ENOSYS)
}
