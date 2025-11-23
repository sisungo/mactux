//! Virtual file descriptor support.

use crate::{filesystem::vfs::Filesystem, poll::PollToken};
use crossbeam::atomic::AtomicCell;
use dashmap::DashMap;
use rustc_hash::FxBuildHasher;
use std::{
    path::PathBuf,
    sync::{
        Arc, OnceLock,
        atomic::{self, AtomicI64, AtomicU64},
    },
};
use structures::{
    error::LxError,
    fs::{Dirent64, OpenFlags, StatFs, Statx, StatxMask, XATTR_NAMESPACE_PREFIXES},
    internal::mactux_ipc::CtrlOutput,
    io::{FcntlCmd, FdFlags, IoctlCmd, PollEvents, VfdAvailCtrl, Whence},
    time::Timespec,
};

pub struct Vfd {
    content: Arc<dyn VfdContent>,
    open_flags: AtomicCell<OpenFlags>,
    offset: AtomicI64,
    orig_path: OnceLock<Vec<u8>>,
}
impl Vfd {
    pub fn new(content: Arc<dyn VfdContent>, open_flags: OpenFlags) -> Self {
        Self {
            content,
            open_flags: AtomicCell::new(open_flags),
            offset: AtomicI64::new(0),
            orig_path: OnceLock::new(),
        }
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize, LxError> {
        if !self.open_flags.load().is_readable() {
            return Err(LxError::EBADF);
        }

        let mut off = self.offset.load(atomic::Ordering::Relaxed);
        let stat = self.content.read(buf, &mut off);
        self.offset.store(off, atomic::Ordering::Relaxed);
        stat
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize, LxError> {
        if !self.open_flags.load().is_writable() {
            return Err(LxError::EBADF);
        }

        let mut off = self.offset.load(atomic::Ordering::Relaxed);
        let stat = self.content.write(buf, &mut off);
        self.offset.store(off, atomic::Ordering::Relaxed);
        stat
    }

    pub fn seek(&self, whence: Whence, off: i64) -> Result<i64, LxError> {
        let orig_off = self.offset.load(atomic::Ordering::Relaxed);
        let new_off = self.content.seek(orig_off, whence, off)?;
        self.offset.store(new_off, atomic::Ordering::Relaxed);
        Ok(new_off)
    }

    pub fn stat(&self, mask: StatxMask) -> Result<Statx, LxError> {
        self.content.stat(mask)
    }

    pub fn pread(&self, buf: &mut [u8], mut off: i64) -> Result<usize, LxError> {
        if !self.open_flags.load().is_readable() {
            return Err(LxError::EBADF);
        }

        self.content.read(buf, &mut off)
    }

    pub fn pwrite(&self, buf: &[u8], mut off: i64) -> Result<usize, LxError> {
        if !self.open_flags.load().is_writable() {
            return Err(LxError::EBADF);
        }

        self.content.write(buf, &mut off)
    }

    pub fn ioctl_query(&self, cmd: IoctlCmd) -> Result<VfdAvailCtrl, LxError> {
        self.content.ioctl_query(cmd)
    }

    pub fn ioctl(&self, cmd: IoctlCmd, data: &[u8]) -> Result<CtrlOutput, LxError> {
        self.content.ioctl(cmd, data)
    }

    pub fn fcntl(&self, cmd: FcntlCmd, data: &[u8]) -> Result<CtrlOutput, LxError> {
        match cmd {
            FcntlCmd::F_GETFL => Ok(CtrlOutput {
                status: self.open_flags.load().bits() as _,
                blob: Vec::new(),
            }),
            FcntlCmd::F_GETFD => {
                if self.open_flags.load().contains(OpenFlags::O_CLOEXEC) {
                    Ok(CtrlOutput {
                        status: FdFlags::FD_CLOEXEC.bits() as _,
                        blob: Vec::new(),
                    })
                } else {
                    Ok(CtrlOutput {
                        status: 0,
                        blob: Vec::new(),
                    })
                }
            }
            FcntlCmd::F_SETFD => {
                let mut fd_flags = [0u8; size_of::<u64>()];
                fd_flags.copy_from_slice(data);
                let fd_flags = u64::from_ne_bytes(fd_flags);
                let Some(fd_flags) = FdFlags::from_bits(fd_flags as _) else {
                    return Err(LxError::EINVAL);
                };
                if fd_flags.contains(FdFlags::FD_CLOEXEC) {
                    self.open_flags
                        .store(self.open_flags.load() | OpenFlags::O_CLOEXEC);
                }
                Ok(CtrlOutput {
                    status: 0,
                    blob: Vec::new(),
                })
            }
            other => todo!("{other:?}"),
        }
    }

    pub fn getdent(&self) -> Result<Option<Dirent64>, LxError> {
        self.content.getdent()
    }

    pub fn dup(self: &Arc<Self>) -> Arc<Self> {
        let content = match self.content.dup() {
            Ok(content) => Arc::clone(&content),
            Err(_) => Arc::clone(&self.content),
        };
        Arc::new(Self {
            content,
            open_flags: AtomicCell::new(self.open_flags.load()),
            offset: AtomicI64::new(self.offset.load(atomic::Ordering::Relaxed)),
            orig_path: self.orig_path.clone(),
        })
    }

    pub fn truncate(&self, len: u64) -> Result<(), LxError> {
        if !self.open_flags.load().is_writable() {
            return Err(LxError::EBADF);
        }
        self.content.truncate(len)
    }

    pub fn chown(&self, uid: u32, gid: u32) -> Result<(), LxError> {
        self.content.chown(uid, gid)
    }

    pub fn chmod(&self, mode: u16) -> Result<(), LxError> {
        self.content.chmod(mode)
    }

    pub fn sync(&self) -> Result<(), LxError> {
        self.content.sync()
    }

    pub fn listxattr(&self) -> Result<Vec<Vec<u8>>, LxError> {
        self.content.listxattr()
    }

    pub fn getxattr(&self, name: &[u8]) -> Result<Vec<u8>, LxError> {
        self.content.getxattr(name)
    }

    pub fn setxattr(&self, name: &[u8], value: &[u8], flags: u32) -> Result<(), LxError> {
        for prefix in XATTR_NAMESPACE_PREFIXES.iter() {
            if name.starts_with(prefix) {
                return self.content.setxattr(name, value, flags);
            }
        }
        Err(LxError::EOPNOTSUPP)
    }

    pub fn removexattr(&self, name: &[u8]) -> Result<(), LxError> {
        self.content.removexattr(name)
    }

    pub fn readlink(&self) -> Result<Vec<u8>, LxError> {
        self.content.readlink()
    }

    pub fn utimens(&self, times: [Timespec; 2]) -> Result<(), LxError> {
        self.content.utimens(times)
    }

    pub fn statfs(&self) -> Result<StatFs, LxError> {
        self.content.filesystem()?.statfs()
    }

    /// Returns the original path of this VFD, if any.
    pub fn orig_path(&self) -> Option<&[u8]> {
        self.orig_path.get().map(|x| &**x)
    }

    /// Sets the original path of this VFD. Fails if it has already been set.
    ///
    /// On failure, the original path remains unchanged.
    pub fn set_orig_path(&self, path: Vec<u8>) -> Result<(), LxError> {
        self.orig_path.set(path).map_err(|_| LxError::EPERM)
    }
}

pub trait Stream {
    fn read(&self, _buf: &mut [u8], _off: &mut i64) -> Result<usize, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn write(&self, _buf: &[u8], _off: &mut i64) -> Result<usize, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn seek(&self, _orig_off: i64, _whence: Whence, _off: i64) -> Result<i64, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn ioctl_query(&self, _cmd: IoctlCmd) -> Result<VfdAvailCtrl, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn ioctl(&self, _cmd: IoctlCmd, _data: &[u8]) -> Result<CtrlOutput, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn poll(&self, _interest: PollEvents) -> Result<PollToken, LxError> {
        Err(LxError::EOPNOTSUPP)
    }
}

pub trait VfdContent: Stream + Send + Sync {
    fn stat(&self, mask: StatxMask) -> Result<Statx, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    /// Duplicates the VFD content.
    ///
    /// Note that if your implementation returns an error, duplication would not fail, instead, it just clones the [`Arc`].
    fn dup(&self) -> Result<Arc<dyn VfdContent>, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn truncate(&self, _len: u64) -> Result<(), LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn getdent(&self) -> Result<Option<Dirent64>, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn chown(&self, _uid: u32, _gid: u32) -> Result<(), LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn chmod(&self, _mode: u16) -> Result<(), LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn readlink(&self) -> Result<Vec<u8>, LxError> {
        Err(LxError::EINVAL)
    }

    fn get_socket(&self, _create: bool) -> Result<PathBuf, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn sync(&self) -> Result<(), LxError> {
        Ok(())
    }

    fn listxattr(&self) -> Result<Vec<Vec<u8>>, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn getxattr(&self, _name: &[u8]) -> Result<Vec<u8>, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn setxattr(&self, _name: &[u8], _value: &[u8], _flags: u32) -> Result<(), LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn removexattr(&self, _name: &[u8]) -> Result<(), LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn utimens(&self, _times: [Timespec; 2]) -> Result<(), LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn filesystem(&self) -> Result<Arc<dyn Filesystem>, LxError> {
        Err(LxError::EOPNOTSUPP)
    }
}

pub struct VfdTable {
    table: DashMap<u64, Arc<Vfd>, FxBuildHasher>,
    next_id: AtomicU64,
}
impl VfdTable {
    pub fn new() -> Self {
        Self {
            table: DashMap::default(),
            next_id: AtomicU64::new(1),
        }
    }

    pub fn register(&self, value: Arc<Vfd>) -> u64 {
        let id = self.next_id.fetch_add(1, atomic::Ordering::Relaxed);
        self.table.insert(id, value);
        id
    }

    pub fn get(&self, id: u64) -> Option<Arc<Vfd>> {
        self.table.get(&id).as_deref().cloned()
    }

    pub fn unregister(&self, id: u64) -> Option<Arc<Vfd>> {
        self.table.remove(&id).map(|(_, v)| v)
    }

    pub fn fork(&self) -> Self {
        Self {
            table: self
                .table
                .iter()
                .map(|x| (*x.key(), x.dup()))
                .collect::<DashMap<_, _, _>>(),
            next_id: AtomicU64::new(self.next_id.load(atomic::Ordering::Relaxed)),
        }
    }

    pub fn on_exec(&self) {
        self.table
            .retain(|_, v| !v.open_flags.load().contains(OpenFlags::O_CLOEXEC));
    }
}
