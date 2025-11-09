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
    NativePath(Vec<u8>),
    LxPath(Vec<u8>),
    Vfd(u64),
    Bytes(Vec<u8>),
    Length(usize),
    Offset(i64),
    CtrlOutput(CtrlOutput),
    VfdAvailCtrl(VfdAvailCtrl),
    Stat(Statx),
    Dirent64(Dirent64),
    NetworkNames(NetworkNames),
    SysInfo(SysInfo),
    Poll(u64, u16),
    Error(LxError),
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct CtrlOutput {
    pub status: c_int,
    pub blob: Vec<u8>,
}
