/// Makes the process fail immediately.
#[cold]
pub fn fast_fail() -> ! {
    unsafe {
        libc::_exit(101);
    }
}
