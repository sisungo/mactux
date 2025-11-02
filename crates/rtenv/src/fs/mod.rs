mod vfd;

use crate::{ipc_client::with_client, posix_bi, posix_num, process, util::ipc_fail};
use arc_swap::ArcSwap;
use libc::c_int;
use mactux_ipc::{request::Request, response::Response};
use std::sync::Arc;
use structures::{
    ToApple,
    device::DeviceNumber,
    error::LxError,
    fs::{
        AccessFlags, AtFlags, Dirent64, FileMode, OpenFlags, OpenHow, OpenResolve, Statx,
        UmountFlags,
    },
};

#[derive(Debug)]
pub struct FilesystemContext {
    pub root: ArcSwap<Vec<u8>>,
    pub cwd: ArcSwap<Vec<u8>>,
}
impl FilesystemContext {
    pub fn new() -> Self {
        Self {
            root: ArcSwap::from(Arc::new(vec![b'/'])),
            cwd: ArcSwap::from(Arc::new(vec![b'/'])),
        }
    }
}

#[inline]
pub fn open(path: Vec<u8>, flags: OpenFlags, mode: FileMode) -> Result<c_int, LxError> {
    openat(-100, full_path(path)?, flags, AtFlags::empty(), mode)
}

#[inline]
pub fn openat(
    dfd: c_int,
    path: Vec<u8>,
    oflags: OpenFlags,
    atflags: AtFlags,
    mode: FileMode,
) -> Result<c_int, LxError> {
    if path.is_empty() && atflags.contains(AtFlags::AT_EMPTY_PATH) {
        return crate::io::dup(dfd);
    }

    let path = at_path(dfd, path)?;

    let mut resolve = OpenResolve::empty();
    if atflags.contains(AtFlags::AT_SYMLINK_NOFOLLOW) {
        resolve |= OpenResolve::RESOLVE_NO_SYMLINKS;
    }

    let how = OpenHow {
        flags: oflags.bits() as _,
        mode: mode.0 as _,
        resolve,
    };

    with_client(
        |client| match client.invoke(Request::Open(path, how)).unwrap() {
            Response::OpenNativePath(native) => open_native(native, oflags, atflags, mode.0 as _),
            Response::OpenVirtualFd(vfd) => crate::vfd::create(vfd, oflags),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        },
    )
}

#[inline]
pub fn access(path: Vec<u8>, mode: AccessFlags) -> Result<(), LxError> {
    faccessat2(-100, path, mode, AtFlags::empty())
}

#[inline]
pub fn faccessat2(
    dfd: c_int,
    path: Vec<u8>,
    mode: AccessFlags,
    flags: AtFlags,
) -> Result<(), LxError> {
    if flags.contains(AtFlags::AT_EMPTY_PATH) && path.is_empty() {
        return Ok(());
    }

    with_client(|client| {
        match client
            .invoke(Request::Access(at_path(dfd, path)?, mode))
            .unwrap()
        {
            Response::Nothing => Ok(()),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

#[inline]
pub fn getdents64(fd: c_int) -> Result<Option<Dirent64>, LxError> {
    match crate::vfd::get(fd) {
        Some(vfd) => vfd::getdents64(vfd),
        None => Err(LxError::EBADF),
    }
}

#[inline]
pub unsafe fn stat(fd: c_int) -> Result<Statx, LxError> {
    match crate::vfd::get(fd) {
        Some(vfd) => vfd::stat(vfd),
        None => unsafe {
            let mut stat = std::mem::zeroed();
            posix_bi!(libc::fstat(fd, &mut stat))?;
            Ok(Statx::from_apple(stat))
        },
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
pub fn symlinkat(src: Vec<u8>, newdfd: c_int, dst: Vec<u8>) -> Result<(), LxError> {
    with_client(|client| {
        match client
            .invoke(Request::Symlink(src, at_path(newdfd, dst)?))
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
            .invoke(Request::Rename(full_path(src)?, full_path(dst)?))
            .unwrap()
        {
            Response::Nothing => Ok(()),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

#[inline]
pub fn linkat(
    sdfd: c_int,
    src: Vec<u8>,
    ddfd: c_int,
    dst: Vec<u8>,
    flags: AtFlags,
) -> Result<(), LxError> {
    let full_src = at_path(sdfd, src)?;
    let full_dst = at_path(sdfd, dst)?;
    with_client(
        |client| match client.invoke(Request::Link(full_src, full_dst)).unwrap() {
            Response::Nothing => Ok(()),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        },
    )
}

#[inline]
pub fn unlink(path: Vec<u8>) -> Result<(), LxError> {
    with_client(
        |client| match client.invoke(Request::Unlink(full_path(path)?)).unwrap() {
            Response::Nothing => Ok(()),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        },
    )
}

#[inline]
pub fn unlinkat(dfd: c_int, path: Vec<u8>, flags: AtFlags) -> Result<(), LxError> {
    let full_path = at_path(dfd, path)?;
    let method = if flags.contains(AtFlags::AT_REMOVEDIR) {
        Request::Rmdir
    } else {
        Request::Unlink
    };
    with_client(|client| match client.invoke(method(full_path)).unwrap() {
        Response::Nothing => Ok(()),
        Response::Error(err) => Err(err),
        _ => ipc_fail(),
    })
}

#[inline]
pub fn rmdir(path: Vec<u8>) -> Result<(), LxError> {
    with_client(
        |client| match client.invoke(Request::Rmdir(full_path(path)?)).unwrap() {
            Response::Nothing => Ok(()),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        },
    )
}

#[inline]
pub fn mkdir(path: Vec<u8>, mode: FileMode) -> Result<(), LxError> {
    with_client(|client| {
        match client
            .invoke(Request::Mkdir(full_path(path)?, mode))
            .unwrap()
        {
            Response::Nothing => Ok(()),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

#[inline]
pub fn mknodat(
    dfd: c_int,
    path: Vec<u8>,
    mode: FileMode,
    dev: DeviceNumber,
) -> Result<(), LxError> {
    with_client(|client| {
        match client
            .invoke(Request::Mknod(at_path(dfd, path)?, mode, dev))
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
pub fn chdir(fd: c_int) -> Result<(), LxError> {
    let Some(vfd) = crate::vfd::get(fd) else {
        return Err(LxError::ENOTDIR);
    };
    process::context()
        .fs
        .cwd
        .store(Arc::new(vfd::orig_path(vfd)?));
    Ok(())
}

#[inline]
pub fn init_cwd(new: Vec<u8>) -> Result<(), LxError> {
    if !new.starts_with(b"/") {
        return Err(LxError::EINVAL);
    }
    process::context().fs.cwd.store(Arc::new(new));
    Ok(())
}

#[inline]
pub fn listxattr(fd: c_int) -> Result<Vec<u8>, LxError> {
    Ok(Vec::new())
}

#[inline]
pub fn umount(path: Vec<u8>, flags: UmountFlags) -> Result<(), LxError> {
    with_client(|client| {
        match client
            .invoke(Request::Umount(full_path(path)?, flags.bits()))
            .unwrap()
        {
            Response::Nothing => Ok(()),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

#[inline]
pub fn readlink(fd: c_int) -> Result<Vec<u8>, LxError> {
    match crate::vfd::get(fd) {
        Some(vfd) => vfd::readlink(vfd),
        None => unsafe {
            let mut buf = vec![0u8; libc::PATH_MAX as usize];
            let nbytes: usize =
                posix_num!(libc::freadlink(fd, buf.as_mut_ptr().cast(), buf.len()))?;
            buf.truncate(nbytes);
            Ok(buf)
        },
    }
}

/// Gets path of a local socket.
pub fn get_sock_path(path: Vec<u8>, create: bool) -> Result<Vec<u8>, LxError> {
    with_client(|client| {
        match client
            .invoke(Request::GetSockPath(full_path(path)?, create))
            .unwrap()
        {
            Response::SockPath(path) => Ok(path),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

fn open_native(
    native: Vec<u8>,
    oflags: OpenFlags,
    atflags: AtFlags,
    mode: u32,
) -> Result<c_int, LxError> {
    unsafe {
        let c_path = crate::util::c_path(native);
        let mut oflags = oflags.to_apple()?;

        if atflags.contains(AtFlags::AT_SYMLINK_NOFOLLOW) {
            oflags |= libc::O_SYMLINK;
        }

        if (oflags & libc::O_CREAT) != 0 {
            posix_num!(libc::open(c_path.as_ptr().cast(), oflags, mode))
        } else {
            posix_num!(libc::open(c_path.as_ptr().cast(), oflags))
        }
    }
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
fn full_path(path: Vec<u8>) -> Result<Vec<u8>, LxError> {
    at_path(-100, path)
}
