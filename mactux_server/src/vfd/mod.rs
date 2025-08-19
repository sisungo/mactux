pub mod eventfd;

use crate::{filesystem::vfs::VfsPath, util::Registry};
use async_trait::async_trait;
use crossbeam::atomic::AtomicCell;
use mactux_ipc::response::{Response, VirtualFdAvailCtrl};
use std::{
    fmt::Debug,
    path::PathBuf,
    sync::{
        Arc, OnceLock,
        atomic::{self, AtomicU64},
    },
};
use structures::{
    error::LxError,
    fs::{Dirent64, OpenFlags, Statx},
    io::{FcntlCmd, FdFlags, IoctlCmd, PollEvents, Whence},
};

/// A virtual file descriptor.
pub struct VirtualFd {
    inner: Box<dyn VirtualFile>,
    open_flags: AtomicCell<OpenFlags>,
    off: AtomicU64,
    orig_path: OnceLock<VfsPath<'static>>,
}
impl VirtualFd {
    pub fn new(inner: Box<dyn VirtualFile>, open_flags: OpenFlags) -> Arc<Self> {
        Arc::new(Self {
            inner,
            open_flags: AtomicCell::new(open_flags),
            off: AtomicU64::new(0),
            orig_path: OnceLock::new(),
        })
    }

    pub async fn read(&self, buf: &mut [u8]) -> Result<usize, LxError> {
        if !self.open_flags.load().is_readable() {
            return Err(LxError::EBADF);
        }

        let mut off = self.off.load(atomic::Ordering::Relaxed);
        let nbytes = self.inner.read(buf, &mut off).await?;
        self.off.store(off, atomic::Ordering::Relaxed);
        Ok(nbytes)
    }

    pub async fn write(&self, buf: &[u8]) -> Result<usize, LxError> {
        if !self.open_flags.load().is_writable() {
            return Err(LxError::EBADF);
        }

        let mut off = self.off.load(atomic::Ordering::Relaxed);
        let nbytes = self.inner.write(buf, &mut off).await?;
        self.off.store(off, atomic::Ordering::Relaxed);
        Ok(nbytes)
    }

    pub async fn lseek(&self, whence: Whence, off: i64) -> Result<u64, LxError> {
        self.inner.lseek(whence, off).await
    }

    pub async fn stat(&self) -> Result<Statx, LxError> {
        self.inner.stat().await
    }

    pub async fn fcntl(&self, cmd: FcntlCmd, buf: &[u8]) -> Result<Response, LxError> {
        match cmd {
            FcntlCmd::F_GETFL => Ok(Response::Ctrl(self.open_flags().bits() as _)),
            FcntlCmd::F_GETFD => {
                if self.open_flags().contains(OpenFlags::O_CLOEXEC) {
                    Ok(Response::Ctrl(FdFlags::FD_CLOEXEC.bits() as _))
                } else {
                    Ok(Response::Ctrl(0))
                }
            }
            FcntlCmd::F_SETFD => {
                let mut fd_flags = [0u8; size_of::<u32>()];
                fd_flags.copy_from_slice(buf);
                let fd_flags = u32::from_ne_bytes(fd_flags);
                let Some(fd_flags) = FdFlags::from_bits(fd_flags) else {
                    return Err(LxError::EINVAL);
                };
                if fd_flags.contains(FdFlags::FD_CLOEXEC) {
                    self.open_flags
                        .store(self.open_flags.load() | OpenFlags::O_CLOEXEC);
                }
                Ok(Response::Ctrl(0))
            }
            other => self.inner.fcntl(other.0, buf.to_vec()).await,
        }
    }

    pub fn ioctl_query(&self, cmd: IoctlCmd) -> Result<VirtualFdAvailCtrl, LxError> {
        self.inner.ioctl_query(cmd.0)
    }

    pub async fn ioctl(&self, cmd: IoctlCmd, buf: &[u8]) -> Result<Response, LxError> {
        self.inner.ioctl(cmd.0, buf.to_vec()).await
    }

    pub async fn dup(self: &Arc<Self>) -> Arc<Self> {
        if let Ok(inner) = self.inner.dup().await {
            Arc::new(Self {
                inner,
                open_flags: AtomicCell::new(self.open_flags.load()),
                off: AtomicU64::new(self.off.load(atomic::Ordering::Relaxed)),
                orig_path: self.orig_path.clone(),
            })
        } else {
            self.clone()
        }
    }

    pub async fn truncate(&self, len: u64) -> Result<(), LxError> {
        self.inner.truncate(len).await
    }

    pub async fn getdents64(&self) -> Result<Option<Dirent64>, LxError> {
        self.inner.getdents64().await
    }

    pub async fn chown(&self, uid: u32, gid: u32) -> Result<(), LxError> {
        self.inner.chown(uid, gid).await
    }

    pub async fn get_socket(&self, create: bool) -> Result<PathBuf, LxError> {
        self.inner.get_socket(create).await
    }

    pub async fn poll(&self, interest: PollEvents) -> Result<PollEvents, LxError> {
        self.inner.poll(interest).await
    }

    pub fn set_orig_path(&self, path: VfsPath<'static>) -> Result<(), LxError> {
        self.orig_path.set(path).map_err(|_| LxError::EACCES)
    }

    pub fn orig_path(&self) -> Result<VfsPath<'static>, LxError> {
        self.orig_path.get().ok_or(LxError::EACCES).cloned()
    }

    pub fn open_flags(&self) -> OpenFlags {
        self.open_flags.load()
    }
}
impl Debug for VirtualFd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VirtualFd")
            .field("open_flags", &self.open_flags)
            .field("off", &self.off)
            .field("orig_path", &self.orig_path)
            .finish_non_exhaustive()
    }
}

#[async_trait]
pub trait VirtualFile: Send + Sync {
    async fn read(&self, _buf: &mut [u8], _off: &mut u64) -> Result<usize, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    async fn write(&self, _buf: &[u8], _off: &mut u64) -> Result<usize, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    async fn lseek(&self, _whence: Whence, _off: i64) -> Result<u64, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    async fn stat(&self) -> Result<Statx, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    async fn fcntl(&self, _: u32, _: Vec<u8>) -> Result<Response, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn ioctl_query(&self, _cmd: u32) -> Result<VirtualFdAvailCtrl, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    async fn ioctl(&self, _cmd: u32, _data: Vec<u8>) -> Result<Response, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    async fn dup(&self) -> Result<Box<dyn VirtualFile>, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    async fn truncate(&self, _len: u64) -> Result<(), LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    async fn getdents64(&self) -> Result<Option<Dirent64>, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    async fn chown(&self, _uid: u32, _gid: u32) -> Result<(), LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    async fn get_socket(&self, _create: bool) -> Result<PathBuf, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    async fn poll(&self, _interest: PollEvents) -> Result<PollEvents, LxError> {
        Err(LxError::EOPNOTSUPP)
    }
}

/// Gets a new virtual file descriptor table after `fork()`.
pub async fn fork_table(vfd_table: &Registry<Arc<VirtualFd>>) -> Registry<Arc<VirtualFd>> {
    let (mut table, next_id) = vfd_table.snapshot();
    for (_, vfd) in table.iter_mut() {
        *vfd = vfd.dup().await;
    }
    Registry::from_snapshot((table, next_id))
}

/// Gets a new virtual file descriptor table after `exec()`.
pub fn exec_table(vfd_table: &Registry<Arc<VirtualFd>>) {
    vfd_table.eliminate(|vfd| vfd.open_flags().contains(OpenFlags::O_CLOEXEC));
}
