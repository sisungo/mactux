use crate::{FromApple, unixvariants};
use bincode::{Decode, Encode};
use std::ffi::c_int;

unixvariants! {
    /// A Linux error.
    #[derive(Encode, Decode)]
    pub struct LxError: u32 {
        const EPERM = 1;
        const ENOENT = 2;
        const ESRCH = 3;
        const EINTR = 4;
        const EIO = 5;
        const ENOEXEC = 8;
        const EBADF = 9;
        const ECHILD = 10;
        const EAGAIN = 11;
        const ENOMEM = 12;
        const EACCES = 13;
        const EFAULT = 14;
        const EBUSY = 16;
        const EEXIST = 17;
        const EXDEV = 18;
        const ENOTDIR = 20;
        const EISDIR = 21;
        const EINVAL = 22;
        const EMFILE = 24;
        const ENOTTY = 25;
        const ETXTBSY = 26;
        const ENOSPC = 28;
        const ESPIPE = 29;
        const EROFS = 30;
        const EPIPE = 32;
        const ERANGE = 34;
        const ENOSYS = 38;
        const ENOTEMPTY = 39;
        const ELOOP = 40;
        const ENOTSOCK = 88;
        const EPROTOTYPE = 91;
        const EOPNOTSUPP = 95;
        const EAFNOSUPPORT = 97;
        const EADDRINUSE = 98;
        const EADDRNOTAVAIL = 99;
        const ENETDOWN = 100;
        const ENETUNREACH = 101;
        const ECONNRESET = 104;
        const ENOBUFS = 105;
        const EISCONN = 106;
        const ETIMEDOUT = 110;
        const ECONNREFUSED = 111;
        const EHOSTUNREACH = 113;
        const EALREADY = 114;
        const EINPROGRESS = 115;
        #[linux_only] const EBADFD = 77;
        fn from_apple(apple: c_int) -> Result<Self, LxError>;
        fn to_apple(self) -> Result<libc::c_int, LxError>;
    }
}
impl LxError {
    /// Returns the [`LxError`] instance converted from last macOS error.
    #[inline]
    pub fn last_apple_error() -> Self {
        Self::from_apple(std::io::Error::last_os_error().raw_os_error().expect(
            "`std::io::Error::last_os_error` should always return an error that has raw OS error",
        ))
        .unwrap_or(Self::EIO)
    }
}
impl From<std::io::Error> for LxError {
    fn from(value: std::io::Error) -> Self {
        match value.raw_os_error() {
            Some(x) => Self::from_apple(x).unwrap_or(Self::EIO),
            None => Self::EIO,
        }
    }
}
