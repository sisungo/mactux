use crate::ipc_client::call_server;
use crate::{ipc_client::with_client, util::ipc_fail};
use structures::fs::StatxMask;
use structures::mactux_ipc::{Request, Response};
use structures::time::Timespec;
use structures::{
    error::LxError,
    fs::{Dirent64, Statx},
};

pub fn getdents64(vfd: u64) -> Result<Option<Dirent64>, LxError> {
    call_server(Request::VfdGetdent(vfd))
}

pub fn stat(vfd: u64, mask: StatxMask) -> Result<Statx, LxError> {
    call_server(Request::VfdStat(vfd, mask))
}

pub fn chown(vfd: u64, uid: u32, gid: u32) -> Result<(), LxError> {
    call_server(Request::VfdChown(vfd, uid, gid))
}

pub fn chmod(vfd: u64, mode: u16) -> Result<(), LxError> {
    call_server(Request::VfdChmod(vfd, mode))
}

pub fn utimens(vfd: u64, times: [Timespec; 2]) -> Result<(), LxError> {
    todo!()
}

pub fn readlink(vfd: u64) -> Result<Vec<u8>, LxError> {
    with_client(|client| {
        let response = client.invoke(Request::VfdReadlink(vfd)).unwrap();
        match response {
            Response::Bytes(path) => Ok(path),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

/// Gets the path that we have used to originally open a virtual file descriptor.
pub fn orig_path(vfd: u64) -> Result<Vec<u8>, LxError> {
    with_client(
        |client| match client.invoke(Request::VfdOrigPath(vfd)).unwrap() {
            Response::LxPath(path) => Ok(path),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        },
    )
}
