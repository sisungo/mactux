use crate::{ipc_client::with_client, util::ipc_fail};
use mactux_ipc::{
    request::Request,
    response::{Response, VirtualFdAvailCtrl},
};
use std::ffi::c_int;
use structures::{
    error::LxError,
    io::{FcntlCmd, Whence},
};

pub fn read(vfd: u64, buf: &mut [u8]) -> Result<usize, LxError> {
    with_client(|client| {
        match client
            .invoke(Request::VirtualFdRead(vfd, buf.len()))
            .unwrap()
        {
            Response::Read(blob) => {
                debug_assert!(blob.len() <= buf.len());
                buf[..blob.len()].copy_from_slice(&blob);
                Ok(blob.len())
            }
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

pub fn pread(vfd: u64, off: i64, buf: &mut [u8]) -> Result<usize, LxError> {
    with_client(|client| {
        match client
            .invoke(Request::VirtualFdPread(vfd, off, buf.len()))
            .unwrap()
        {
            Response::Read(blob) => {
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
    with_client(|client| {
        match client
            .invoke(Request::VirtualFdWrite(vfd, buf.to_vec()))
            .unwrap()
        {
            Response::Write(n) => Ok(n),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

pub fn pwrite(vfd: u64, off: i64, buf: &[u8]) -> Result<usize, LxError> {
    with_client(|client| {
        match client
            .invoke(Request::VirtualFdPwrite(vfd, off, buf.to_vec()))
            .unwrap()
        {
            Response::Write(n) => Ok(n),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

pub fn lseek(vfd: u64, whence: Whence, off: i64) -> Result<u64, LxError> {
    with_client(|client| {
        match client
            .invoke(Request::VirtualFdLseek(vfd, whence, off))
            .unwrap()
        {
            Response::Lseek(n) => Ok(n),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

pub fn dup(vfd: u64) -> u64 {
    with_client(
        |client| match client.invoke(Request::VirtualFdDup(vfd)).unwrap() {
            Response::DupVirtualFd(x) => x,
            _ => ipc_fail(),
        },
    )
}

pub fn ioctl(vfd: u64, cmd: u32, arg: *mut u8) -> Result<c_int, LxError> {
    let avail_ctrl = with_client(|client| {
        match client
            .invoke(Request::VirtualFdIoctlQuery(vfd, cmd))
            .unwrap()
        {
            Response::VirtualFdAvailCtrl(avail_ctrl) => Ok(avail_ctrl),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })?;

    ctrl(vfd, cmd, arg as usize, avail_ctrl, Request::VirtualFdIoctl)
}

pub fn fcntl(vfd: u64, cmd: u32, arg: usize) -> Result<c_int, LxError> {
    let avail_ctrl = VirtualFdAvailCtrl {
        in_size: FcntlCmd(cmd).in_size(),
        out_size: FcntlCmd(cmd).out_size(),
    };

    ctrl(vfd, cmd, arg, avail_ctrl, Request::VirtualFdFcntl)
}

pub fn truncate(vfd: u64, len: u64) -> Result<(), LxError> {
    with_client(
        |client| match client.invoke(Request::VirtualFdTruncate(vfd, len)).unwrap() {
            Response::Nothing => Ok(()),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        },
    )
}

pub fn close(vfd: u64) {
    with_client(|client| {
        client.invoke(Request::VirtualFdClose(vfd)).unwrap();
    });
}

fn ctrl(
    vfd: u64,
    cmd: u32,
    arg: usize,
    avail_ctrl: VirtualFdAvailCtrl,
    act: fn(u64, u32, Vec<u8>) -> Request,
) -> Result<c_int, LxError> {
    with_client(|client| {
        let in_param = match avail_ctrl.in_size {
            1.. => unsafe {
                std::slice::from_raw_parts(arg as *const u8, avail_ctrl.in_size as usize).to_vec()
            },
            0 => Vec::new(),
            ..0 => (arg as usize).to_le_bytes().to_vec(),
        };
        let response = client.invoke(act(vfd, cmd, in_param)).unwrap();
        match response {
            Response::Nothing => Ok(0),
            Response::Ctrl(stat) => {
                debug_assert_eq!(avail_ctrl.out_size, 0);
                Ok(stat as _)
            }
            Response::CtrlBlob(stat, out_param) => unsafe {
                debug_assert_eq!(avail_ctrl.out_size, out_param.len());
                (arg as *mut u8).copy_from(out_param.as_ptr(), out_param.len());
                Ok(stat as _)
            },
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}
