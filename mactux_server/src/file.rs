//! Traits for describing a file-like object.

use mactux_ipc::response::{CtrlOutput, VfdAvailCtrl};
use structures::{
    error::LxError,
    io::{IoctlCmd, Whence},
};

pub trait Stream {
    fn read(&self, _buf: &mut [u8], _off: &mut i64) -> Result<usize, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn write(&self, _buf: &[u8], _off: &mut i64) -> Result<usize, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn seek(&self, _whence: Whence, _off: i64) -> Result<u64, LxError> {
        Err(LxError::EOPNOTSUPP)
    }
}

pub trait Ioctl {
    fn ioctl_query(&self, _cmd: IoctlCmd) -> Result<VfdAvailCtrl, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn ioctl(&self, _cmd: IoctlCmd, _data: &[u8]) -> Result<CtrlOutput, LxError> {
        Err(LxError::EOPNOTSUPP)
    }
}
