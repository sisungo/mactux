#![feature(slice_split_once)]

mod elf;
mod shebang;

use std::{
    io::{Read, Seek, SeekFrom},
    os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd},
};
use structures::error::LxError;

#[derive(Debug)]
pub enum Program {
    Elf(elf::Program),
    Shebang(shebang::Program),
}
impl Program {
    pub fn load<R: Into<OwnedFd>>(exec_fd: R) -> Result<Self, Error> {
        let owned_fd = exec_fd.into();
        let mut io_fd = IoFd(owned_fd.as_fd());

        let mut buf = [0; 8];
        io_fd
            .read_exact(&mut buf)
            .map_err(|x| Error::ReadImage(x.into()))?;
        io_fd
            .seek(SeekFrom::Start(0))
            .map_err(|x| Error::ReadImage(x.into()))?;

        if buf.starts_with(elf::Program::MAGIC) {
            return Ok(Self::Elf(elf::Program::load(owned_fd)?));
        }

        if buf.starts_with(shebang::Program::MAGIC) {
            return Ok(Self::Shebang(shebang::Program::load(owned_fd)?));
        }

        Err(Error::ImageFormat(String::from(
            "unrecognized image format",
        )))
    }

    pub unsafe fn run<'a, 'b>(&self, args: &[&[u8]], envs: &[&[u8]]) {
        unsafe {
            match self {
                Self::Elf(x) => x.run(args, envs),
                Self::Shebang(x) => x.run(args, envs),
            }
        }
    }
}

#[derive(Debug)]
pub enum Error {
    ReadImage(LxError),
    ImageFormat(String),
    LoadImage(LxError),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadImage(e) => write!(f, "failed to read image: {e}"),
            Self::ImageFormat(e) => write!(f, "exec format error: {e}"),
            Self::LoadImage(e) => write!(f, "failed to map image segments: {e}"),
        }
    }
}
impl std::error::Error for Error {}

/// A file descriptor that implements standard Rust IO traits.
#[derive(Debug)]
struct IoFd<'a>(BorrowedFd<'a>);
impl Read for IoFd<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        unsafe {
            match libc::read(self.0.as_raw_fd(), buf.as_mut_ptr() as _, buf.len()) {
                -1 => Err(std::io::Error::last_os_error()),
                n => Ok(n as usize),
            }
        }
    }
}
impl Seek for IoFd<'_> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        let offset = match pos {
            std::io::SeekFrom::Start(o) => o as _,
            std::io::SeekFrom::End(o) => o as _,
            std::io::SeekFrom::Current(o) => o as _,
        };
        let whence = match pos {
            std::io::SeekFrom::Start(_) => libc::SEEK_SET,
            std::io::SeekFrom::End(_) => libc::SEEK_END,
            std::io::SeekFrom::Current(_) => libc::SEEK_CUR,
        };
        unsafe {
            match libc::lseek(self.0.as_raw_fd(), offset, whence) {
                -1 => Err(std::io::Error::last_os_error()),
                n => Ok(n as u64),
            }
        }
    }
}
