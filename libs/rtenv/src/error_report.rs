use std::io::Write;

/// The error report port.
///
/// This is basically the same as [`std::io::Stderr`], but this guarantees that this is mapped to the POSIX primitives
/// directly and no thread-local variables or locks are used, so this can be used from the emulated context or async signal
/// handlers.
#[derive(Debug, Clone, Copy)]
pub struct ErrorReport;
impl Write for ErrorReport {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        unsafe {
            match libc::write(libc::STDERR_FILENO, buf.as_ptr().cast(), buf.len()) {
                -1 => Err(std::io::Error::last_os_error()),
                n => Ok(n as _),
            }
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Makes the process fail immediately.
#[cold]
pub fn fast_fail() -> ! {
    unsafe {
        libc::_exit(101);
    }
}
