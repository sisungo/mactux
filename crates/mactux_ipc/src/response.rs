use crate::types::NetworkNames;
use bincode::{Decode, Encode};
use std::ffi::c_int;
use structures::{
    error::LxError,
    fs::{Dirent64, Statx},
    io::VfdAvailCtrl,
    misc::SysInfo,
};

/// A response to a MacTux IPC request.
#[derive(Debug, Clone, Encode, Decode)]
pub enum Response {
    Nothing,

    OpenNativePath(Vec<u8>),
    OpenVirtualFd(u64),
    SockPath(Vec<u8>),
    EventFd(u64),

    Read(Vec<u8>),
    Write(usize),
    Lseek(u64),
    CtrlOutput(CtrlOutput),
    DupVirtualFd(u64),
    OrigPath(Vec<u8>),
    IoctlQuery(VfdAvailCtrl),
    Stat(Statx),
    Dirent64(Dirent64),
    Readlink(Vec<u8>),

    NetworkNames(NetworkNames),
    SysInfo(SysInfo),
    ThreadName(Vec<u8>),

    Poll(u64, u16),

    Error(LxError),
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct CtrlOutput {
    pub status: c_int,
    pub blob: Vec<u8>,
}
