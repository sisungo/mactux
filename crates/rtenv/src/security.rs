use std::ffi::c_uint;

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
