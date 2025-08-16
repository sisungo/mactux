use libc::{c_char, strlen};
use structures::{error::LxError, net::SockAddr};

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
