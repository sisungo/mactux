//! MacTux program loader.

#![feature(slice_split_once)]

mod elf;
mod shebang;

use rtenv::rust::OwnedRtFd;
use structures::error::LxError;

#[derive(Debug)]
pub enum Program {
    Elf(elf::Program),
    Shebang(shebang::Program),
}
impl Program {
    pub fn load(path: Vec<u8>) -> Result<Self, Error> {
        let mut buf = [0; 8];
        let fd = OwnedRtFd::open(path.clone()).map_err(Error::ReadImage)?;
        rtenv::io::read(fd.0, &mut buf).map_err(Error::ReadImage)?;
        drop(fd);

        if buf.starts_with(elf::Program::MAGIC) {
            return Ok(Self::Elf(elf::Program::load(path)?));
        }

        if buf.starts_with(shebang::Program::MAGIC) {
            return Ok(Self::Shebang(shebang::Program::load(path)?));
        }

        Err(Error::ImageFormat(String::from(
            "unrecognized image format",
        )))
    }

    pub unsafe fn run(&self, args: &[&[u8]], envs: &[&[u8]]) -> ! {
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
