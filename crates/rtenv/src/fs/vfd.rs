use crate::ipc_client::with_client;
use mactux_ipc::{request::Request, response::Response};
use structures::{
    error::LxError,
    fs::{Dirent64, Stat},
};

pub fn getdents64(vfd: u64) -> Result<Option<Dirent64>, LxError> {
    with_client(|client| {
        let response = client.invoke(Request::VirtualFdGetDents64(vfd)).unwrap();
        match response {
            Response::Nothing => Ok(None),
            Response::Dirent64(dent) => Ok(Some(dent)),
            Response::Error(err) => Err(err),
            _ => panic!("unexpected server response"),
        }
    })
}

pub fn stat(vfd: u64) -> Result<Stat, LxError> {
    with_client(|client| {
        let response = client.invoke(Request::VirtualFdStat(vfd)).unwrap();
        match response {
            Response::Stat(stat) => Ok(stat),
            Response::Error(err) => Err(err),
            _ => panic!("unexpected server response"),
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
            _ => panic!("unexpected server response"),
        }
    })
}

/// Gets the path that we have used to originally open a virtual file descriptor.
pub fn orig_path(vfd: u64) -> Result<Vec<u8>, LxError> {
    with_client(
        |client| match client.invoke(Request::VirtualFdOrigPath(vfd)).unwrap() {
            Response::OrigPath(path) => Ok(path),
            Response::Error(err) => Err(err),
            _ => panic!("unexpected server response"),
        },
    )
}
