use crate::newtype_impl_from_apple;
use bincode::{Decode, Encode};
use std::ffi::c_int;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode)]
pub struct LxError(pub u32);
impl LxError {
    pub const EPERM: Self = Self(1);
    pub const ENOENT: Self = Self(2);
    pub const ESRCH: Self = Self(3);
    pub const EINTR: Self = Self(4);
    pub const EIO: Self = Self(5);
    pub const ENOEXEC: Self = Self(8);
    pub const EBADF: Self = Self(9);
    pub const ECHILD: Self = Self(10);
    pub const EAGAIN: Self = Self(11);
    pub const ENOMEM: Self = Self(12);
    pub const EACCES: Self = Self(13);
    pub const EFAULT: Self = Self(14);
    pub const EBUSY: Self = Self(16);
    pub const EEXIST: Self = Self(17);
    pub const EXDEV: Self = Self(18);
    pub const ENOTDIR: Self = Self(20);
    pub const EISDIR: Self = Self(21);
    pub const EINVAL: Self = Self(22);
    pub const EMFILE: Self = Self(24);
    pub const ENOTTY: Self = Self(25);
    pub const ETXTBSY: Self = Self(26);
    pub const ENOSPC: Self = Self(28);
    pub const ESPIPE: Self = Self(29);
    pub const EROFS: Self = Self(30);
    pub const EPIPE: Self = Self(32);
    pub const ENOSYS: Self = Self(38);
    pub const ENOTEMPTY: Self = Self(39);
    pub const ELOOP: Self = Self(40);
    pub const EOPNOTSUPP: Self = Self(95);
    pub const EBADFD: Self = Self(77);
    pub const EAFNOSUPPORT: Self = Self(97);

    pub fn from_apple(apple: c_int) -> Self {
        newtype_impl_from_apple!(
            apple = EPERM,
            ENOENT,
            ESRCH,
            EINTR,
            EIO,
            ENOEXEC,
            EBADF,
            ECHILD,
            EAGAIN,
            ENOMEM,
            EACCES,
            EFAULT,
            EBUSY,
            EEXIST,
            EXDEV,
            ENOTDIR,
            EISDIR,
            EINVAL,
            EMFILE,
            ENOTTY,
            ETXTBSY,
            ENOSPC,
            ESPIPE,
            EROFS,
            EPIPE,
            ENOSYS,
            ENOTEMPTY,
            ELOOP,
            EOPNOTSUPP,
            EAFNOSUPPORT
        )
        .unwrap_or(Self::EIO)
    }

    pub fn last_apple_error() -> Self {
        Self::from_apple(std::io::Error::last_os_error().raw_os_error().expect(
            "`std::io::Error::last_os_error` should always return an error that has raw OS error",
        ))
    }
}
impl From<std::io::Error> for LxError {
    fn from(value: std::io::Error) -> Self {
        match value.raw_os_error() {
            Some(x) => Self::from_apple(x),
            None => Self::EIO,
        }
    }
}
