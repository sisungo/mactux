use libc::{c_char, c_int, strlen};
use structures::{
    error::LxError,
    fs::{AtFlags, OpenFlags},
    net::SockAddr,
};

/// Converts from a C-style string to a rust byte slice.
pub unsafe fn rust_bytes<'a>(s: *const c_char) -> &'a [u8] {
    unsafe { std::slice::from_raw_parts(s.cast::<u8>(), strlen(s)) }
}

/// Returns socket address `addr` to userspace.
pub unsafe fn ret_sockaddr(addr: SockAddr, buf: *mut u8, len: *mut u32) -> Result<(), LxError> {
    if buf.is_null() || len.is_null() {
        return Ok(());
    }
    unsafe {
        let size = addr.write_to(std::slice::from_raw_parts_mut(buf, len.read() as usize))?;
        len.write(size as _);
    }
    Ok(())
}

pub fn with_openat<T>(
    dfd: c_int,
    path: Vec<u8>,
    oflags: OpenFlags,
    atflags: AtFlags,
    mode: u32,
    f: impl FnOnce(c_int) -> Result<T, LxError>,
) -> Result<T, LxError> {
    let fd = rtenv::fs::openat(dfd, path, oflags, atflags, mode)?;
    let ret = f(fd).inspect_err(|_| _ = rtenv::io::close(fd))?;
    _ = rtenv::io::close(fd);
    Ok(ret)
}

pub unsafe fn ret_buf(buf: &[u8], ptr: *mut u8, len: usize) -> Result<usize, LxError> {
    unsafe {
        if buf.len() > len {
            return Err(LxError::ERANGE);
        }
        ptr.copy_from(buf.as_ptr().cast(), buf.len());
        Ok(buf.len())
    }
}
