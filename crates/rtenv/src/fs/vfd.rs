use crate::{ipc_client::with_client, util::ipc_fail};
use mactux_ipc::{request::Request, response::Response};
use structures::{
    error::LxError,
    fs::{Dirent64, Statx},
};

pub fn getdents64(vfd: u64) -> Result<Option<Dirent64>, LxError> {
    with_client(|client| {
        let response = client.invoke(Request::VirtualFdGetDents64(vfd)).unwrap();
        match response {
            Response::Nothing => Ok(None),
            Response::Dirent64(dent) => Ok(Some(dent)),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

pub fn stat(vfd: u64) -> Result<Statx, LxError> {
    with_client(|client| {
        let response = client.invoke(Request::VirtualFdStat(vfd)).unwrap();
        match response {
            Response::Stat(stat) => Ok(stat.into()),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

pub fn chown(vfd: u64, uid: u32, gid: u32) -> Result<(), LxError> {
    with_client(|client| {
        let response = client
            .invoke(Request::VirtualFdChown(vfd, uid, gid))
            .unwrap();
        match response {
            Response::Nothing => Ok(()),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

pub fn readlink(vfd: u64) -> Result<Vec<u8>, LxError> {
    with_client(|client| {
        let response = client.invoke(Request::VirtualFdReadlink(vfd)).unwrap();
        match response {
            Response::Readlink(path) => Ok(path),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

/// Gets the path that we have used to originally open a virtual file descriptor.
pub fn orig_path(vfd: u64) -> Result<Vec<u8>, LxError> {
    with_client(
        |client| match client.invoke(Request::VirtualFdOrigPath(vfd)).unwrap() {
            Response::OrigPath(path) => Ok(path),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        },
    )
}
