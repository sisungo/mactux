use crate::vfd::{Stream, Vfd, VfdContent};
use std::sync::Arc;
use structures::{error::LxError, fs::OpenFlags};

pub fn open(flags: OpenFlags) -> Result<Vfd, LxError> {
    Ok(Vfd::new(Arc::new(InvalidFd), flags))
}

#[derive(Debug)]
struct InvalidFd;
impl Stream for InvalidFd {}
impl VfdContent for InvalidFd {}
