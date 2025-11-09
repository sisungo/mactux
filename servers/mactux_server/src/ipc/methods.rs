use crate::{
    app,
    filesystem::{VPath, vfs::NewlyOpen},
    syslog::WriteLogRequest,
    task::{process::Process, thread::Thread},
    util::Shared,
    vfd::Vfd,
};
use std::{io::Write, sync::Arc};
use structures::{
    device::DeviceNumber,
    error::LxError,
    fs::{AccessFlags, Dirent64, FileMode, OpenFlags, OpenHow, Statx, UmountFlags},
    io::{FcntlCmd, IoctlCmd, VfdAvailCtrl, Whence},
    misc::{LogLevel, SysInfo},
};
use structures::{
    io::EventFdFlags,
    mactux_ipc::{CtrlOutput, NetworkNames, Response},
};

pub fn open(path: Vec<u8>, how: OpenHow) -> Result<NewlyOpen, LxError> {
    Process::current()
        .mnt
        .locate(&VPath::parse(&path))?
        .open(how)
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

pub fn umount(path: &[u8], flags: UmountFlags) -> Result<(), LxError> {
    Process::current().mnt.umount(&VPath::parse(path), flags)
}

pub fn get_sock_path(path: Vec<u8>, create: bool) -> Result<Response, LxError> {
    Process::current()
        .mnt
        .locate(&VPath::parse(&path))?
        .get_sock_path(create)
        .map(|path| Response::NativePath(path.into_os_string().into_encoded_bytes()))
}

pub fn vfd_dup(vfd: u64) -> Result<Arc<Vfd>, LxError> {
    Ok(Process::current().vfd.get(vfd).ok_or(LxError::EBADF)?.dup())
}

pub fn vfd_read(vfd: u64, bufsiz: usize) -> Result<Response, LxError> {
    let mut buf = vec![0; bufsiz];
    let nbytes = Process::current()
        .vfd
        .get(vfd)
        .ok_or(LxError::EBADF)?
        .read(&mut buf)?;
    buf.truncate(nbytes);
    Ok(Response::Bytes(buf))
}

pub fn vfd_pread(vfd: u64, bufsiz: usize, off: i64) -> Result<Response, LxError> {
    let mut buf = vec![0; bufsiz];
    let nbytes = Process::current()
        .vfd
        .get(vfd)
        .ok_or(LxError::EBADF)?
        .pread(&mut buf, off)?;
    buf.truncate(nbytes);
    Ok(Response::Bytes(buf))
}

pub fn vfd_write(vfd: u64, buf: &[u8]) -> Result<Response, LxError> {
    Process::current()
        .vfd
        .get(vfd)
        .ok_or(LxError::EBADF)?
        .write(buf)
        .map(Response::Length)
}

pub fn vfd_pwrite(vfd: u64, buf: &[u8], off: i64) -> Result<Response, LxError> {
    Process::current()
        .vfd
        .get(vfd)
        .ok_or(LxError::EBADF)?
        .pwrite(buf, off)
        .map(Response::Length)
}

pub fn vfd_lseek(vfd: u64, whence: Whence, off: i64) -> Result<Response, LxError> {
    Process::current()
        .vfd
        .get(vfd)
        .ok_or(LxError::EBADF)?
        .seek(whence, off)
        .map(Response::Offset)
}

pub fn vfd_truncate(vfd: u64, len: u64) -> Result<(), LxError> {
    Process::current()
        .vfd
        .get(vfd)
        .ok_or(LxError::EBADF)?
        .truncate(len)
}

pub fn vfd_chown(vfd: u64, uid: u32, gid: u32) -> Result<(), LxError> {
    Process::current()
        .vfd
        .get(vfd)
        .ok_or(LxError::EBADF)?
        .chown(uid, gid)
}

pub fn vfd_sync(vfd: u64) -> Result<(), LxError> {
    Process::current()
        .vfd
        .get(vfd)
        .ok_or(LxError::EBADF)?
        .sync()
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

pub fn vfd_readlink(vfd: u64) -> Result<Response, LxError> {
    Process::current()
        .vfd
        .get(vfd)
        .ok_or(LxError::EBADF)?
        .readlink()
        .map(Response::Bytes)
}

pub fn vfd_orig_path(vfd: u64) -> Result<Option<Response>, LxError> {
    Ok(Process::current()
        .vfd
        .get(vfd)
        .ok_or(LxError::EBADF)?
        .orig_path()
        .map(|x| Response::LxPath(x.to_vec())))
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

pub fn set_network_names(set: NetworkNames) -> Result<(), LxError> {
    let uts = &Process::current().uts;
    uts.set_nodename(set.nodename)?;
    uts.set_domainname(set.domainname)?;
    Ok(())
}

pub fn sysinfo() -> Result<SysInfo, LxError> {
    crate::sysinfo::sysinfo()
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

pub fn after_fork(native_pid: libc::pid_t) -> Result<(), LxError> {
    crate::task::process::after_fork(native_pid)
}

pub fn after_exec() {
    Process::current().exec();
}

pub fn set_thread_name(name: Vec<u8>) {
    *Thread::current().comm.write().unwrap() = Some(name);
}

pub fn get_thread_name() -> Result<Response, LxError> {
    Ok(Response::Bytes(
        Thread::current()
            .comm
            .read()
            .unwrap()
            .clone()
            .unwrap_or_default(),
    ))
}

pub fn write_syslog(level: LogLevel, mut content: Vec<u8>) {
    let pid = Shared::id(&Process::current());
    _ = write!(&mut content, " {{ pid={pid} }}");
    app().syslog.write(WriteLogRequest { level, content });
}

pub fn set_mnt_namespace(ns: u64) -> Result<(), LxError> {
    todo!()
}

pub fn set_pid_namespace(ns: u64) -> Result<(), LxError> {
    todo!()
}

pub fn set_uts_namespace(ns: u64) -> Result<(), LxError> {
    todo!()
}

pub fn eventfd(count: u64, flags: EventFdFlags) -> Result<Vfd, LxError> {
    crate::filesystem::eventfd::open(count, flags)
}

pub fn invalidfd(flags: OpenFlags) -> Result<Vfd, LxError> {
    crate::filesystem::invalidfd::open(flags)
}

pub fn pid_linux_to_native(linux: i32) -> Result<Response, LxError> {
    Process::current().pid.lton(linux).map(Response::Pid)
}

pub fn pid_native_to_linux(native: i32) -> Result<Response, LxError> {
    Process::current().pid.ntol(native).map(Response::Pid)
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
            Self::Native(npath) => Response::NativePath(npath),
            Self::Virtual(vfd) => vfd.into_response(),
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
        Response::Stat(Box::new(self))
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
        Response::VfdAvailCtrl(self)
    }
}
impl IntoResponse for CtrlOutput {
    fn into_response(self) -> Response {
        Response::CtrlOutput(self)
    }
}
impl IntoResponse for SysInfo {
    fn into_response(self) -> Response {
        Response::SysInfo(Box::new(self))
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
impl IntoResponse for Vfd {
    fn into_response(self) -> Response {
        Arc::new(self).into_response()
    }
}
impl IntoResponse for Arc<Vfd> {
    fn into_response(self) -> Response {
        Response::Vfd(Process::current().vfd.register(self))
    }
}
