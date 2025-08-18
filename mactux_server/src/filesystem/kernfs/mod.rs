mod typical_files;

pub use typical_files::fn_file;

use crate::{
    filesystem::vfs::{Mountable, NewlyOpen, VfsPath},
    util::FileAttrs,
    vfd::{VirtualFd, VirtualFile},
};
use async_trait::async_trait;
use mactux_ipc::response::Response;
use std::{
    borrow::Cow,
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex, RwLock},
};
use structures::{
    error::LxError,
    fs::{AccessFlags, Dirent64, Dirent64Hdr, DirentType, OpenFlags, Statx},
    io::FcntlCmd,
};

#[derive(Default)]
pub struct KernFs(pub Arc<Directory>);
impl KernFs {
    pub fn new() -> Self {
        Self::default()
    }
}
#[async_trait]
impl Mountable for KernFs {
    async fn open(
        self: Arc<Self>,
        path: &VfsPath,
        flags: OpenFlags,
        mode: u32,
    ) -> Result<NewlyOpen, LxError> {
        if path.segments.is_empty() {
            return Ok(NewlyOpen::Virtual(VirtualFd::new(
                Box::new(KernFsDirVirtualFd::new(self.0.clone(), FileAttrs::common())),
                flags,
            )));
        }
        let dir_entry = self.0._open(&path.segments)?;
        if (path.indicates_dir() || flags.contains(OpenFlags::O_DIRECTORY))
            && !matches!(dir_entry, DirEntry::Directory(_))
        {
            return Err(LxError::ENOTDIR);
        }
        match &dir_entry {
            DirEntry::Directory(dir) => Ok(NewlyOpen::Virtual(VirtualFd::new(
                Box::new(KernFsDirVirtualFd::new(dir.clone(), dir.attrs.clone())),
                flags,
            ))),
            DirEntry::RegularFile(reg) => reg.open(flags).await,
        }
    }

    async fn unlink(&self, _path: &VfsPath) -> Result<(), LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    async fn rmdir(&self, _path: &VfsPath) -> Result<(), LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    async fn symlink(&self, _dst: &VfsPath, _content: &[u8]) -> Result<(), LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    async fn mkdir(&self, _path: &VfsPath, _mode: u32) -> Result<(), LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    async fn rename(&self, _src: &VfsPath, _dst: &VfsPath) -> Result<(), LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    async fn access(&self, _path: &VfsPath, _mode: AccessFlags) -> Result<(), LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    async fn get_sock_path(&self, _path: &VfsPath, _create: bool) -> Result<PathBuf, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    async fn mount_bind(&self, _path: &VfsPath) -> Result<Box<dyn Mountable>, LxError> {
        todo!()
    }
}

pub struct Directory {
    pub table: RwLock<BTreeMap<Vec<u8>, DirEntry>>,
    pub attrs: FileAttrs,
}
impl Directory {
    pub fn new() -> Self {
        Self {
            table: RwLock::default(),
            attrs: FileAttrs::common(),
        }
    }

    fn _open(&self, sgmt: &[Cow<'_, [u8]>]) -> Result<DirEntry, LxError> {
        let reader = self.table.read().unwrap();
        if sgmt.len() == 1 {
            return reader.get(&*sgmt[0]).ok_or(LxError::ENOENT).cloned();
        }
        let dir_entry = reader.get(&*sgmt[0]).ok_or(LxError::ENOENT)?;
        match &dir_entry {
            DirEntry::Directory(dir) => dir._open(&sgmt[1..]),
            DirEntry::RegularFile(_) => Err(LxError::ENOTDIR),
        }
    }
}
impl Default for Directory {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub enum DirEntry {
    Directory(Arc<Directory>),
    RegularFile(Arc<dyn KernFsFile>),
}

#[async_trait]
pub trait KernFsFile: Send + Sync {
    async fn open(&self, flags: OpenFlags) -> Result<NewlyOpen, LxError>;
}

pub struct KernFsDirVirtualFd {
    attrs: FileAttrs,
    keys: Mutex<Vec<Vec<u8>>>,
    dir: Arc<Directory>,
}
impl KernFsDirVirtualFd {
    fn new(dir: Arc<Directory>, attrs: FileAttrs) -> Self {
        let mut keys = dir
            .table
            .read()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<Vec<u8>>>();
        keys.push(vec![b'.']);
        keys.push(vec![b'.', b'.']);
        let keys = Mutex::new(keys);

        Self { attrs, keys, dir }
    }
}
#[async_trait]
impl VirtualFile for KernFsDirVirtualFd {
    async fn stat(&self) -> Result<Statx, LxError> {
        Ok(Statx {
            stx_mask: 0,
            stx_dev_major: 0,
            stx_dev_minor: 0,
            stx_ino: 0,
            stx_nlink: 0,
            stx_uid: self.attrs.uid,
            stx_gid: self.attrs.gid,
            stx_mode: self.attrs.mode as u16 | 0o40000,
            stx_attributes: 0,
            stx_attributes_mask: 0,
            stx_rdev_major: 0,
            stx_rdev_minor: 0,
            stx_size: 0,
            stx_blksize: 0,
            stx_blocks: 0,
            stx_atime: self.attrs.atime.into(),
            stx_btime: self.attrs.btime.into(),
            stx_ctime: self.attrs.ctime.into(),
            stx_mtime: self.attrs.mtime.into(),
            stx_mnt_id: 0,
            stx_dio_mem_align: 0,
            stx_dio_offset_align: 0,
            stx_dio_read_offset_align: 0,
            stx_atomic_write_segments_max: 0,
            stx_atomic_write_unit_min: 0,
            stx_atomic_write_unit_max: 0,
            stx_subvol: 0,
        })
    }

    async fn fcntl(&self, cmd: u32, data: Vec<u8>) -> Result<Response, LxError> {
        let cmd = FcntlCmd(cmd);
        match cmd {
            FcntlCmd::F_SETFD => Ok(Response::Ctrl(0)),
            FcntlCmd::F_GETFL => Ok(Response::Ctrl((OpenFlags::O_DIRECTORY).bits() as _)),
            FcntlCmd::F_SETFL => Ok(Response::Ctrl(0)),
            _ => Err(LxError::EINVAL),
        }
    }

    async fn getdents64(&self) -> Result<Option<Dirent64>, LxError> {
        const DIRENT64HDR_REFDIR: Dirent64Hdr = Dirent64Hdr {
            d_ino: 0,
            d_off: 0,
            d_reclen: 0,
            d_type: DirentType::DT_DIR,
            _align: [0; _],
        };

        let dir_reader = self.dir.table.read().unwrap();
        let Some(next_key) = self.keys.lock().unwrap().pop() else {
            return Ok(None);
        };
        if next_key == b"." || next_key == b".." {
            return Ok(Some(Dirent64::new(DIRENT64HDR_REFDIR, next_key)));
        }

        let Some(dir_entry) = dir_reader.get(&next_key) else {
            return Ok(None);
        };
        let d_type = match &dir_entry {
            DirEntry::RegularFile(_) => DirentType::DT_REG,
            DirEntry::Directory(_) => DirentType::DT_DIR,
        };
        let hdr = Dirent64Hdr {
            d_ino: 0,
            d_off: 0,
            d_reclen: 0,
            d_type,
            _align: [0; _],
        };
        Ok(Some(Dirent64::new(hdr, next_key)))
    }
}
