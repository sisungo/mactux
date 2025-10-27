use crate::{
    filesystem::{VPath, vfs::NewlyOpen},
    task::process::Process,
};
use mactux_ipc::response::{CtrlOutput, NetworkNames, Response, VfdAvailCtrl};
use std::sync::Arc;
use structures::{
    device::DeviceNumber,
    error::LxError,
    fs::{AccessFlags, Dirent64, FileMode, OpenFlags, Statx},
    io::{FcntlCmd, IoctlCmd, Whence},
};

pub fn open(path: Vec<u8>, flags: OpenFlags, mode: FileMode) -> Result<NewlyOpen, LxError> {
    Process::current()
        .mnt
        .locate(&VPath::parse(&path))?
        .open(flags, mode)
}

pub fn access(path: Vec<u8>, flags: AccessFlags) -> Result<(), LxError> {
    Process::current()
        .mnt
        .locate(&VPath::parse(&path))?
        .access(flags)
}

pub fn unlink(path: Vec<u8>) -> Result<(), LxError> {
    Process::current()
        .mnt
        .locate(&VPath::parse(&path))?
        .unlink()
}

pub fn rmdir(path: Vec<u8>) -> Result<(), LxError> {
    Process::current().mnt.locate(&VPath::parse(&path))?.rmdir()
}

pub fn mkdir(path: Vec<u8>, mode: FileMode) -> Result<(), LxError> {
    Process::current()
        .mnt
        .locate(&VPath::parse(&path))?
        .mkdir(mode)
}

pub fn mknod(path: Vec<u8>, mode: FileMode, dev: DeviceNumber) -> Result<(), LxError> {
    Process::current()
        .mnt
        .locate(&VPath::parse(&path))?
        .mknod(mode, dev)
}

pub fn symlink(src: &[u8], dst: &[u8]) -> Result<(), LxError> {
    Process::current()
        .mnt
        .locate(&VPath::parse(dst))?
        .symlink(src)
}

pub fn link(src: &[u8], dst: &[u8]) -> Result<(), LxError> {
    let dst = Process::current().mnt.locate(&VPath::parse(dst))?;
    let src = Process::current().mnt.locate(&VPath::parse(src))?;
    dst.link_to(src)
}

pub fn rename(src: &[u8], dst: &[u8]) -> Result<(), LxError> {
    let dst = Process::current().mnt.locate(&VPath::parse(dst))?;
    let src = Process::current().mnt.locate(&VPath::parse(src))?;
    dst.rename_to(src)
}

pub fn get_sock_path(path: Vec<u8>, create: bool) -> Result<Response, LxError> {
    Process::current()
        .mnt
        .locate(&VPath::parse(&path))?
        .get_sock_path(create)
        .map(|path| Response::SockPath(path.into_os_string().into_encoded_bytes()))
}

pub fn vfd_read(vfd: u64, bufsiz: usize) -> Result<Response, LxError> {
    let mut buf = vec![0; bufsiz];
    Process::current()
        .vfd
        .get(vfd)
        .ok_or(LxError::EBADF)?
        .read(&mut buf)?;
    Ok(Response::Read(buf))
}

pub fn vfd_write(vfd: u64, buf: &[u8]) -> Result<Response, LxError> {
    Ok(Response::Write(
        Process::current()
            .vfd
            .get(vfd)
            .ok_or(LxError::EBADF)?
            .write(buf)?,
    ))
}

pub fn vfd_lseek(vfd: u64, whence: Whence, off: i64) -> Result<Response, LxError> {
    Ok(Response::Lseek(
        Process::current()
            .vfd
            .get(vfd)
            .ok_or(LxError::EBADF)?
            .seek(whence, off)?,
    ))
}

pub fn vfd_stat(vfd: u64, mask: u32) -> Result<Statx, LxError> {
    Process::current()
        .vfd
        .get(vfd)
        .ok_or(LxError::EBADF)?
        .stat(mask)
}

pub fn vfd_getdent(vfd: u64) -> Result<Option<Dirent64>, LxError> {
    Process::current()
        .vfd
        .get(vfd)
        .ok_or(LxError::EBADF)?
        .getdent()
}

pub fn vfd_orig_path(vfd: u64) -> Result<Option<Response>, LxError> {
    Ok(Process::current()
        .vfd
        .get(vfd)
        .ok_or(LxError::EBADF)?
        .orig_path()
        .map(|x| Response::OrigPath(x.to_vec())))
}

pub fn vfd_close(vfd: u64) -> Result<(), LxError> {
    Process::current()
        .vfd
        .unregister(vfd)
        .ok_or(LxError::EBADF)
        .map(|_| ())
}

pub fn get_network_names() -> Result<NetworkNames, LxError> {
    let uts = &Process::current().uts;
    Ok(NetworkNames {
        nodename: uts.nodename(),
        domainname: uts.domainname(),
    })
}

pub fn vfd_ioctl_query(vfd: u64, cmd: IoctlCmd) -> Result<VfdAvailCtrl, LxError> {
    Process::current()
        .vfd
        .get(vfd)
        .ok_or(LxError::EBADF)?
        .ioctl_query(cmd)
}

pub fn vfd_ioctl(vfd: u64, cmd: IoctlCmd, data: &[u8]) -> Result<CtrlOutput, LxError> {
    Process::current()
        .vfd
        .get(vfd)
        .ok_or(LxError::EBADF)?
        .ioctl(cmd, data)
}

pub fn vfd_fcntl(vfd: u64, cmd: FcntlCmd, data: &[u8]) -> Result<CtrlOutput, LxError> {
    Process::current()
        .vfd
        .get(vfd)
        .ok_or(LxError::EBADF)?
        .fcntl(cmd, data)
}

pub fn before_fork() {
    crate::task::process::before_fork();
}

pub fn after_fork(native_pid: libc::pid_t) -> Result<(), LxError> {
    crate::task::process::after_fork(native_pid)
}

pub fn after_exec() {
    Process::current().exec();
}

pub trait IntoResponse {
    fn into_response(self) -> Response;
}
impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}
impl<T: IntoResponse> IntoResponse for Result<T, LxError> {
    fn into_response(self) -> Response {
        match self {
            Ok(val) => val.into_response(),
            Err(err) => Response::Error(err),
        }
    }
}
impl IntoResponse for NewlyOpen {
    fn into_response(self) -> Response {
        match self {
            Self::Native(npath) => Response::OpenNativePath(npath),
            Self::Virtual(vfd) => {
                let id = Process::current().vfd.register(Arc::new(vfd));
                Response::OpenVirtualFd(id)
            }
        }
    }
}
impl IntoResponse for Dirent64 {
    fn into_response(self) -> Response {
        Response::Dirent64(self)
    }
}
impl IntoResponse for Statx {
    fn into_response(self) -> Response {
        Response::Stat(self)
    }
}
impl IntoResponse for () {
    fn into_response(self) -> Response {
        Response::Nothing
    }
}
impl IntoResponse for NetworkNames {
    fn into_response(self) -> Response {
        Response::NetworkNames(self)
    }
}
impl IntoResponse for VfdAvailCtrl {
    fn into_response(self) -> Response {
        Response::IoctlQuery(self)
    }
}
impl IntoResponse for CtrlOutput {
    fn into_response(self) -> Response {
        Response::CtrlOutput(self)
    }
}
impl<T> IntoResponse for Option<T>
where
    T: IntoResponse,
{
    fn into_response(self) -> Response {
        match self {
            Some(val) => val.into_response(),
            None => Response::Nothing,
        }
    }
}
