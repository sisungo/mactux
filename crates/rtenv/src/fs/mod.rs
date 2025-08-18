mod vfd;

use crate::{ipc_client::with_client, posix_bi, posix_num, process, util::ipc_fail};
use arc_swap::ArcSwap;
use libc::c_int;
use mactux_ipc::{request::Request, response::Response};
use std::sync::Arc;
use structures::{
    error::LxError,
    fs::{AccessFlags, AtFlags, Dirent64, OpenFlags, Statx},
};

#[derive(Debug)]
pub struct FilesystemContext {
    pub cwd: ArcSwap<Vec<u8>>,
}
impl FilesystemContext {
    pub fn new() -> Self {
        Self {
            cwd: ArcSwap::from(Arc::new(vec![b'/'])),
        }
    }
}

#[inline]
pub fn open(path: Vec<u8>, flags: OpenFlags, mode: u32) -> Result<c_int, LxError> {
    with_client(|client| {
        match client
            .invoke(Request::Open(full_path(path), flags.bits(), mode))
            .unwrap()
        {
            Response::OpenNativePath(native) => unsafe {
                let c_path = crate::util::c_path(native);
                if flags.contains(OpenFlags::O_CREAT) {
                    posix_num!(libc::open(c_path.as_ptr().cast(), flags.to_apple(), mode))
                } else {
                    posix_num!(libc::open(c_path.as_ptr().cast(), flags.to_apple()))
                }
            },
            Response::OpenVirtualFd(vfd) => crate::vfd::create(vfd, flags),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

#[inline]
pub fn openat(
    dfd: c_int,
    path: Vec<u8>,
    mut oflags: OpenFlags,
    atflags: AtFlags,
    mode: u32,
) -> Result<c_int, LxError> {
    if atflags.contains(AtFlags::AT_SYMLINK_NOFOLLOW) {
        oflags |= OpenFlags::O_NOFOLLOW;
    }

    if path.is_empty() && atflags.contains(AtFlags::AT_EMPTY_PATH) {
        return crate::io::dup(dfd);
    }

    open(at_path(dfd, path)?, oflags, mode)
}

#[inline]
pub fn access(path: Vec<u8>, mode: AccessFlags) -> Result<(), LxError> {
    with_client(|client| {
        match client
            .invoke(Request::Access(full_path(path), mode.bits()))
            .unwrap()
        {
            Response::Nothing => Ok(()),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

#[inline]
pub fn faccessat2(
    dfd: c_int,
    mut path: Vec<u8>,
    mode: AccessFlags,
    flags: AtFlags,
) -> Result<(), LxError> {
    if flags.contains(AtFlags::AT_EMPTY_PATH) && path.is_empty() {
        return Ok(());
    }

    if let Some(dvfd) = crate::vfd::get(dfd) {
        let mut new_path = vfd::orig_path(dvfd)?;
        new_path.push(b'/');
        new_path.append(&mut path);
        access(new_path, mode)
    } else {
        Err(LxError::ENOTDIR)
    }
}

#[inline]
pub fn getdents64(fd: c_int) -> Result<Option<Dirent64>, LxError> {
    if let Some(vfd) = crate::vfd::get(fd) {
        vfd::getdents64(vfd)
    } else {
        Err(LxError::EBADF)
    }
}

#[inline]
pub unsafe fn stat(fd: c_int) -> Result<Statx, LxError> {
    if let Some(vfd) = crate::vfd::get(fd) {
        vfd::stat(vfd)
    } else {
        let mut stat = unsafe { std::mem::zeroed() };
        unsafe {
            posix_bi!(libc::fstat(fd, &mut stat))?;
            Ok(Statx::from_apple(stat))
        }
    }
}

#[inline]
pub unsafe fn chown(fd: c_int, uid: u32, gid: u32) -> Result<(), LxError> {
    if let Some(vfd) = crate::vfd::get(fd) {
        vfd::chown(vfd, uid, gid)
    } else {
        unsafe {
            posix_bi!(libc::fchown(fd, uid, gid))?;
            Ok(())
        }
    }
}

#[inline]
pub fn symlink(src: Vec<u8>, dst: Vec<u8>) -> Result<(), LxError> {
    with_client(|client| {
        match client
            .invoke(Request::Symlink(src, full_path(dst)))
            .unwrap()
        {
            Response::Nothing => Ok(()),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

#[inline]
pub fn rename(src: Vec<u8>, dst: Vec<u8>) -> Result<(), LxError> {
    with_client(|client| {
        match client
            .invoke(Request::Rename(full_path(src), full_path(dst)))
            .unwrap()
        {
            Response::Nothing => Ok(()),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

#[inline]
pub fn unlink(path: Vec<u8>) -> Result<(), LxError> {
    with_client(
        |client| match client.invoke(Request::Unlink(full_path(path))).unwrap() {
            Response::Nothing => Ok(()),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        },
    )
}

#[inline]
pub fn rmdir(path: Vec<u8>) -> Result<(), LxError> {
    with_client(
        |client| match client.invoke(Request::Rmdir(full_path(path))).unwrap() {
            Response::Nothing => Ok(()),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        },
    )
}

#[inline]
pub fn mkdir(path: Vec<u8>, mode: u32) -> Result<(), LxError> {
    with_client(|client| {
        match client
            .invoke(Request::Mkdir(full_path(path), mode))
            .unwrap()
        {
            Response::Nothing => Ok(()),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

#[inline]
pub fn getcwd() -> Vec<u8> {
    process::context().fs.cwd.load().to_vec()
}

#[inline]
pub fn chdir(new: Vec<u8>) -> Result<(), LxError> {
    _ = crate::io::close(open(
        new.clone(),
        OpenFlags::O_PATH | OpenFlags::O_DIRECTORY,
        0,
    )?);
    process::context().fs.cwd.store(Arc::new(new));
    Ok(())
}

#[inline]
pub fn fchdir(fd: c_int) -> Result<(), LxError> {
    let Some(vfd) = crate::vfd::get(fd) else {
        return Err(LxError::ENOTDIR);
    };
    chdir(vfd::orig_path(vfd)?)
}

/// Gets path of a local socket.
pub fn get_sock_path(path: Vec<u8>, create: bool) -> Result<Vec<u8>, LxError> {
    with_client(|client| {
        match client
            .invoke(Request::GetSockPath(full_path(path), create))
            .unwrap()
        {
            Response::SockPath(path) => Ok(path),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

/// Returns path relative to current root directory for given path at given file descriptor.
fn at_path(fd: c_int, mut path: Vec<u8>) -> Result<Vec<u8>, LxError> {
    if path.first() == Some(&b'/') {
        return Ok(path);
    }

    let mut new_path = at_base_path(fd)?;
    new_path.push(b'/');
    new_path.append(&mut path);
    Ok(new_path)
}

/// Returns path prefix of `fd` when using with `at` functions.
fn at_base_path(fd: c_int) -> Result<Vec<u8>, LxError> {
    if let Some(dvfd) = crate::vfd::get(fd) {
        vfd::orig_path(dvfd)
    } else if fd == -100 {
        Ok(getcwd())
    } else {
        Err(LxError::ENOTDIR)
    }
}

/// Returns a path that can be accepted by the MacTux server from a relative path.
fn full_path(mut path: Vec<u8>) -> Vec<u8> {
    if path.first().copied() == Some(b'/') {
        path
    } else {
        let mut full_path = getcwd();
        full_path.push(b'/');
        full_path.append(&mut path);
        full_path
    }
}
