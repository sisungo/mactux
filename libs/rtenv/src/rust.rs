//! Rust-flavored `rtenv` APIs.
//!
//! The `rtenv` APIs are provided in the Linux system call flavor to fit the system call implementations best. However, using them
//! with Rust utilities are hard. This module provides Rust-flavored wrappers.

use std::{
    io::{Read, Seek, SeekFrom, Write},
    mem::ManuallyDrop,
    ops::Deref,
};
use structures::{
    ToApple,
    error::LxError,
    fs::{AtFlags, FileMode, OpenFlags},
    io::Whence,
};

#[derive(Debug)]
pub struct OwnedRtFd(RawRtFd);
impl OwnedRtFd {
    pub fn open(path: Vec<u8>) -> Result<Self, LxError> {
        crate::fs::openat(
            -100,
            path,
            OpenFlags::O_RDONLY,
            AtFlags::empty(),
            FileMode(0),
        )
        .map(RawRtFd)
        .map(Self)
    }

    pub fn leak(self) -> RawRtFd {
        ManuallyDrop::new(self).0
    }
}
impl Read for OwnedRtFd {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}
impl Write for OwnedRtFd {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}
impl Seek for OwnedRtFd {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.0.seek(pos)
    }
}
impl Drop for OwnedRtFd {
    fn drop(&mut self) {
        self.0.close();
    }
}
impl Deref for OwnedRtFd {
    type Target = RawRtFd;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct RawRtFd(pub i32);
impl RawRtFd {
    pub fn close(self) {
        _ = crate::io::close(self.0);
    }

    pub fn is_virtual(&self) -> bool {
        crate::vfd::get(self.0).is_some()
    }
}
impl Read for RawRtFd {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        crate::io::read(self.0, buf).map_err(map_error)
    }
}
impl Write for RawRtFd {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        crate::io::write(self.0, buf).map_err(map_error)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl Seek for RawRtFd {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let (whence, off) = match pos {
            SeekFrom::Start(n) => (Whence::SEEK_SET, n as i64),
            SeekFrom::Current(n) => (Whence::SEEK_CUR, n),
            SeekFrom::End(n) => (Whence::SEEK_END, n),
        };
        crate::io::lseek(self.0, off, whence)
            .map_err(map_error)
            .map(|x| x.abs() as u64)
    }
}

fn map_error(err: LxError) -> std::io::Error {
    std::io::Error::from_raw_os_error(err.to_apple().unwrap_or(libc::EINVAL))
}
