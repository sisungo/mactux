use crate::{
    ipc_client::{call_server, with_client},
    util::ipc_fail,
};
use std::ffi::c_int;
use structures::{
    error::LxError,
    io::{FcntlCmd, IoctlCmd, VfdAvailCtrl, Whence},
    mactux_ipc::{Request, Response},
};

pub fn read(vfd: u64, buf: &mut [u8]) -> Result<usize, LxError> {
    with_client(
        |client| match client.invoke(Request::VfdRead(vfd, buf.len())).unwrap() {
            Response::Bytes(blob) => {
                debug_assert!(blob.len() <= buf.len());
                buf[..blob.len()].copy_from_slice(&blob);
                Ok(blob.len())
            }
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        },
    )
}

pub fn pread(vfd: u64, off: i64, buf: &mut [u8]) -> Result<usize, LxError> {
    with_client(|client| {
        match client
            .invoke(Request::VfdPread(vfd, off, buf.len()))
            .unwrap()
        {
            Response::Bytes(blob) => {
                debug_assert!(blob.len() <= buf.len());
                buf[..blob.len()].copy_from_slice(&blob);
                Ok(blob.len())
            }
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

pub fn write(vfd: u64, buf: &[u8]) -> Result<usize, LxError> {
    with_client(
        |client| match client.invoke(Request::VfdWrite(vfd, buf.to_vec())).unwrap() {
            Response::Length(n) => Ok(n),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        },
    )
}

pub fn pwrite(vfd: u64, off: i64, buf: &[u8]) -> Result<usize, LxError> {
    with_client(|client| {
        match client
            .invoke(Request::VfdPwrite(vfd, off, buf.to_vec()))
            .unwrap()
        {
            Response::Length(n) => Ok(n),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

pub fn seek(vfd: u64, whence: Whence, off: i64) -> Result<i64, LxError> {
    with_client(
        |client| match client.invoke(Request::VfdSeek(vfd, whence, off)).unwrap() {
            Response::Offset(n) => Ok(n),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        },
    )
}

pub fn dup(vfd: u64) -> u64 {
    with_client(
        |client| match client.invoke(Request::VfdDup(vfd)).unwrap() {
            Response::Vfd(x) => x,
            _ => ipc_fail(),
        },
    )
}

pub fn ioctl(vfd: u64, cmd: IoctlCmd, arg: *mut u8) -> Result<c_int, LxError> {
    let avail_ctrl =
        with_client(
            |client| match client.invoke(Request::VfdIoctlQuery(vfd, cmd)).unwrap() {
                Response::VfdAvailCtrl(avail_ctrl) => Ok(avail_ctrl),
                Response::Error(err) => Err(err),
                _ => ipc_fail(),
            },
        )?;

    ctrl(vfd, cmd, arg as usize, avail_ctrl, Request::VfdIoctl)
}

pub fn fcntl(vfd: u64, cmd: FcntlCmd, arg: usize) -> Result<c_int, LxError> {
    ctrl(vfd, cmd, arg, cmd.ctrl_query(), Request::VfdFcntl)
}

pub fn truncate(vfd: u64, len: u64) -> Result<(), LxError> {
    call_server(Request::VfdTruncate(vfd, len))
}

pub fn sync(vfd: u64) -> Result<(), LxError> {
    call_server(Request::VfdSync(vfd))
}

pub fn close(vfd: u64) {
    call_server(Request::VfdClose(vfd))
}

fn ctrl<C>(
    vfd: u64,
    cmd: C,
    arg: usize,
    avail_ctrl: VfdAvailCtrl,
    act: fn(u64, C, Vec<u8>) -> Request,
) -> Result<c_int, LxError> {
    with_client(|client| {
        let in_param = match avail_ctrl.in_size {
            1.. => unsafe {
                std::slice::from_raw_parts(arg as *const u8, avail_ctrl.in_size as usize).to_vec()
            },
            0 => Vec::new(),
            ..0 => arg.to_le_bytes().to_vec(),
        };
        let response = client.invoke(act(vfd, cmd, in_param)).unwrap();
        match response {
            Response::CtrlOutput(out) => unsafe {
                debug_assert_eq!(avail_ctrl.out_size, out.blob.len());
                (arg as *mut u8).copy_from_nonoverlapping(out.blob.as_ptr(), out.blob.len());
                Ok(out.status)
            },
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}
