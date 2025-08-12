use libc::{c_char, strlen};

pub unsafe fn rust_bytes<'a>(s: *const c_char) -> &'a [u8] {
    unsafe { std::slice::from_raw_parts(s.cast::<u8>(), strlen(s)) }
}
