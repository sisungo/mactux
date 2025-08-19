use std::ffi::c_uint;
use structures::{
    error::LxError,
    security::{UserCap, UserCapHeader},
};

pub fn uid() -> c_uint {
    unsafe { libc::getuid() }
}

pub fn gid() -> c_uint {
    unsafe { libc::getgid() }
}

pub fn euid() -> c_uint {
    unsafe { libc::geteuid() }
}

pub fn egid() -> c_uint {
    unsafe { libc::getegid() }
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
    if pid == 0 || pid == crate::process::pid() {
        todo!()
    }
    todo!()
}

pub fn capset(cap: UserCap) -> Result<(), LxError> {
    if cap.pid == 0 || cap.pid == crate::process::pid() {
        todo!()
    }
    todo!()
}
