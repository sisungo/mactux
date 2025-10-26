//! A generic in-memory filesystem.

use crate::{
    app,
    device::Device,
    filesystem::{
        VPath,
        vfs::{Filesystem, LPath, NewlyOpen},
    },
    task::process::Process,
    util::symlink_abs,
    vfd::{Vfd, VfdContent},
};
use dashmap::DashMap;
use std::{
    fmt::Debug,
    path::PathBuf,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{self, AtomicU16, AtomicU32, AtomicUsize},
    },
};
use structures::{
    device::DeviceNumber,
    error::LxError,
    fs::{AccessFlags, Dirent64, FileMode, FileType, OpenFlags, Statx, StatxAttrs, StatxMask},
    time::Timespec,
};

/// An extended attribute key to specify the file to behave like a kernfs file when the value is `1`.
const XATTR_KERNFS: &[u8] = b"_mactux.kernfs";

/// Size of a block.
const BLOCK_SIZE: u32 = 4096;

pub struct Tmpfs {
    root: Arc<Dir>,
}
impl Tmpfs {
    pub fn new() -> Result<Arc<Self>, LxError> {
        todo!()
    }

    fn locate(&self, path: LPath) -> Result<Location, LxError> {
        if path.relative.parts.is_empty() {
            return Ok(Location::Direct(
                self.root.clone(),
                Some(Node::Dir(self.root.clone())),
            ));
        }

        let mut dir_name = path.relative.parts.clone();
        let file_name = dir_name.pop().expect("empty parts should return early");
        let mut dir = self.root.clone();
        for dir_part in dir_name.into_iter().rev() {
            let node = dir.children.get(&dir_part).ok_or(LxError::ENOENT)?.clone();
            dir = match node {
                Node::Dir(x) => x.clone(),
                Node::File(_) => return Err(LxError::ENOTDIR),
                Node::Symlink(symlink) => return Ok(Location::MidSymlink(symlink.solve(path))),
            };
        }
        let node = dir.children.get(&file_name).map(|x| x.clone());
        if matches!(&node, Some(Node::File(_))) && path.relative.slash_suffix {
            return Err(LxError::ENOTDIR);
        }
        Ok(Location::Direct(dir, node))
    }
}
impl Filesystem for Tmpfs {
    fn open(
        self: Arc<Self>,
        path: LPath,
        flags: OpenFlags,
        mode: FileMode,
    ) -> Result<NewlyOpen, LxError> {
        match self.locate(path.clone())? {
            Location::Direct(_, Some(node)) => match node {
                Node::Dir(dir) => dir.open_vfd(flags).map(NewlyOpen::Virtual),
                Node::File(file) => {
                    if flags.contains(OpenFlags::O_EXCL) {
                        return Err(LxError::EEXIST);
                    }
                    if flags.contains(OpenFlags::O_DIRECTORY) {
                        return Err(LxError::ENOTDIR);
                    }
                    if let Ok(vfd) = Arc::clone(&file).open_vfd(flags) {
                        return Ok(NewlyOpen::Virtual(vfd));
                    }
                    Ok(NewlyOpen::Native(
                        file.open_native()
                            .unwrap()
                            .into_os_string()
                            .into_encoded_bytes(),
                    ))
                }
                Node::Symlink(symlink) => {
                    if flags.contains(OpenFlags::O_NOFOLLOW) {
                        return Err(LxError::ELOOP);
                    }
                    Process::current()
                        .mnt
                        .locate(&symlink.solve(path))?
                        .open(flags, mode)
                }
            },
            Location::Direct(dir, None) => {
                if !flags.contains(OpenFlags::O_CREAT) {
                    return Err(LxError::ENOENT);
                }
                if flags.contains(OpenFlags::O_DIRECTORY) || path.relative.slash_suffix {
                    return Err(LxError::ENOTDIR);
                }
                let metadata = Metadata::new();
                metadata.permbits.store(mode.0, atomic::Ordering::Relaxed);
                let file = Arc::new(Reg {
                    metadata,
                    buf: RegBuf::new(),
                });
                dir.children.insert(
                    path.relative.parts.last().ok_or(LxError::EEXIST)?.clone(),
                    Node::File(file.clone()),
                );
                Ok(NewlyOpen::Virtual(file.open_vfd(flags)?))
            }
            Location::MidSymlink(vpath) => Process::current().mnt.locate(&vpath)?.open(flags, mode),
        }
    }

    fn access(&self, path: LPath, mode: AccessFlags) -> Result<(), LxError> {
        match self.locate(path.clone())? {
            Location::Direct(_, Some(node)) => match node {
                Node::Dir(dir) => Ok(()),
                Node::File(file) => Ok(()),
                Node::Symlink(symlink) => Process::current()
                    .mnt
                    .locate(&symlink.solve(path))?
                    .access(mode),
            },
            Location::Direct(dir, None) => Err(LxError::ENOENT),
            Location::MidSymlink(vpath) => Process::current().mnt.locate(&vpath)?.access(mode),
        }
    }

    fn get_sock_path(&self, path: LPath, create: bool) -> Result<PathBuf, LxError> {
        todo!()
    }

    fn link(&self, src: LPath, dst: LPath) -> Result<(), LxError> {
        todo!()
    }

    fn mkdir(&self, path: LPath, mode: FileMode) -> Result<(), LxError> {
        match self.locate(path.clone())? {
            Location::Direct(_, Some(_)) => Err(LxError::EEXIST),
            Location::Direct(dir, None) => {
                let child = Dir {
                    metadata: Arc::new(Metadata::new()),
                    children: DashMap::new(),
                };
                child
                    .metadata
                    .permbits
                    .store(mode.permbits(), atomic::Ordering::Relaxed);
                dir.children.insert(
                    path.relative.parts.last().ok_or(LxError::EEXIST)?.clone(),
                    Node::Dir(Arc::new(child)),
                );
                Ok(())
            }
            Location::MidSymlink(vpath) => Process::current().mnt.locate(&vpath)?.mkdir(mode),
        }
    }

    fn rename(&self, src: LPath, dst: LPath) -> Result<(), LxError> {
        todo!()
    }

    fn rmdir(&self, path: LPath) -> Result<(), LxError> {
        match self.locate(path.clone())? {
            Location::Direct(dir, Some(node)) => {
                if !matches!(node, Node::Dir(_)) {
                    return Err(LxError::ENOTDIR);
                }
                dir.children
                    .remove(path.relative.parts.last().ok_or(LxError::ENOENT)?);
                Ok(())
            }
            Location::Direct(_, None) => Err(LxError::ENOENT),
            Location::MidSymlink(vpath) => Process::current().mnt.locate(&vpath)?.rmdir(),
        }
    }

    fn symlink(&self, dst: LPath, content: &[u8]) -> Result<(), LxError> {
        match self.locate(dst.clone())? {
            Location::Direct(_, Some(_)) => Err(LxError::EEXIST),
            Location::Direct(dir, None) => {
                let child = Symlink {
                    metadata: Metadata::new(),
                    target: content.to_vec(),
                };
                child
                    .metadata
                    .permbits
                    .store(0o777, atomic::Ordering::Relaxed);
                dir.children.insert(
                    dst.relative.parts.last().ok_or(LxError::EEXIST)?.clone(),
                    Node::Symlink(Arc::new(child)),
                );
                Ok(())
            }
            Location::MidSymlink(vpath) => Process::current().mnt.locate(&vpath)?.symlink(content),
        }
    }

    fn unlink(&self, path: LPath) -> Result<(), LxError> {
        match self.locate(path.clone())? {
            Location::Direct(dir, Some(node)) => {
                if !matches!(node, Node::File(_)) {
                    return Err(LxError::EISDIR);
                }
                dir.children
                    .remove(path.relative.parts.last().ok_or(LxError::ENOENT)?);
                Ok(())
            }
            Location::Direct(_, None) => Err(LxError::ENOENT),
            Location::MidSymlink(vpath) => Process::current().mnt.locate(&vpath)?.unlink(),
        }
    }

    fn mknod(&self, path: LPath, mode: FileMode, dev: DeviceNumber) -> Result<(), LxError> {
        match self.locate(path.clone())? {
            Location::Direct(_, Some(_)) => Err(LxError::EEXIST),
            Location::Direct(dir, None) => {
                let metadata = Arc::new(Metadata::new());
                metadata
                    .permbits
                    .store(mode.permbits(), atomic::Ordering::Relaxed);
                let child = match mode.file_type() {
                    FileType::BlockDevice => Arc::new(BlockDev { metadata, dev }) as _,
                    FileType::CharDevice => Arc::new(CharDev { metadata, dev }) as _,
                    _ => return Err(LxError::EOPNOTSUPP),
                };
                dir.children.insert(
                    path.relative.parts.last().ok_or(LxError::EEXIST)?.clone(),
                    Node::File(child),
                );
                Ok(())
            }
            Location::MidSymlink(vpath) => Process::current().mnt.locate(&vpath)?.mknod(mode, dev),
        }
    }
}

#[derive(Debug, Clone)]
enum Node {
    Dir(Arc<Dir>),
    File(Arc<dyn File>),
    Symlink(Arc<Symlink>),
}

#[derive(Debug, Clone)]
enum Location {
    Direct(Arc<Dir>, Option<Node>),
    MidSymlink(VPath),
}

trait File: Debug + Send + Sync {
    fn open_vfd(self: Arc<Self>, flags: OpenFlags) -> Result<Vfd, LxError>;
    fn open_native(&self) -> Option<PathBuf> {
        None
    }
}

#[derive(Debug)]
struct Dir {
    metadata: Arc<Metadata>,
    children: DashMap<Vec<u8>, Node>,
}
impl File for Dir {
    fn open_vfd(self: Arc<Self>, flags: OpenFlags) -> Result<Vfd, LxError> {
        let iter = self
            .children
            .iter()
            .map(|p| (p.key().clone(), p.value().clone()))
            .collect();
        // TODO add . and ..
        Ok(Vfd::new(
            Arc::new(DirFd {
                metadata: self.metadata.clone(),
                iter: Mutex::new(iter),
            }),
            flags,
        ))
    }
}

#[derive(Debug)]
struct DirFd {
    metadata: Arc<Metadata>,
    iter: Mutex<Vec<(Vec<u8>, Node)>>,
}
impl VfdContent for DirFd {
    fn getdent(&self) -> Result<Option<Dirent64>, LxError> {
        todo!()
    }
}

#[derive(Debug)]
struct Reg {
    metadata: Metadata,
    buf: RegBuf,
}
impl File for Reg {
    fn open_vfd(self: Arc<Self>, flags: OpenFlags) -> Result<Vfd, LxError> {
        Ok(Vfd::new(self, flags))
    }
}
impl VfdContent for Reg {
    fn read(&self, buf: &mut [u8], off: &mut i64) -> Result<usize, LxError> {
        if *off < 0 {
            return Err(LxError::EINVAL);
        }

        let orig_off = *off;

        loop {
            let id = *off / BLOCK_SIZE as i64;

            // this shall only occur in the first block to read
            if *off % BLOCK_SIZE as i64 != 0 {
                let mut block = [0; BLOCK_SIZE as usize];
                let rem = *off - id * BLOCK_SIZE as i64;
                let block_read_len = self.buf.read_block(id as _, &mut block);
                let read_len = block_read_len.min(BLOCK_SIZE as usize - rem as usize);
                buf[..read_len].copy_from_slice(&block[rem as usize..rem as usize + read_len]);
                *off += read_len as i64;
                if block_read_len != BLOCK_SIZE as _ {
                    return Ok(read_len);
                }
                continue;
            }

            let bytes_to_read =
                (BLOCK_SIZE as usize).min(buf.len() - (*off as usize - orig_off as usize));
            let actual_read = self.buf.read_block(
                id as _,
                &mut buf[(*off as usize - orig_off as usize)
                    ..(*off as usize - orig_off as usize) + bytes_to_read],
            );
            *off += bytes_to_read as i64;
            if actual_read != bytes_to_read || *off - orig_off == buf.len() as _ {
                return Ok(*off as usize - orig_off as usize);
            }
        }
    }

    fn stat(&self) -> Result<Statx, LxError> {
        let mut stat = self.metadata.stat_template(StatxMask::all());

        stat.stx_size = self.buf.size();
        stat.stx_blocks = self.buf.blocks() * (BLOCK_SIZE as u64 / 512);

        stat.stx_mode.set_file_type(FileType::RegularFile);

        Ok(stat)
    }
}

#[derive(Debug)]
struct CharDev {
    metadata: Arc<Metadata>,
    dev: DeviceNumber,
}
impl File for CharDev {
    fn open_vfd(self: Arc<Self>, flags: OpenFlags) -> Result<Vfd, LxError> {
        let device = app().devices.find_chr(self.dev)?;
        device.open(flags)?;
        Ok(Vfd::new(
            Arc::new(DevFd {
                metadata: self.metadata.clone(),
                device,
            }),
            flags,
        ))
    }
}

#[derive(Debug)]
struct BlockDev {
    metadata: Arc<Metadata>,
    dev: DeviceNumber,
}
impl File for BlockDev {
    fn open_vfd(self: Arc<Self>, flags: OpenFlags) -> Result<Vfd, LxError> {
        let device = app().devices.find_blk(self.dev)?;
        device.open(flags)?;
        Ok(Vfd::new(
            Arc::new(DevFd {
                metadata: self.metadata.clone(),
                device,
            }),
            flags,
        ))
    }
}

struct DevFd {
    metadata: Arc<Metadata>,
    device: Arc<dyn Device>,
}
impl VfdContent for DevFd {
    fn read(&self, buf: &mut [u8], off: &mut i64) -> Result<usize, LxError> {
        self.device.read(buf, off)
    }

    fn write(&self, buf: &[u8], off: &mut i64) -> Result<usize, LxError> {
        self.device.write(buf, off)
    }
}
impl Drop for DevFd {
    fn drop(&mut self) {
        self.device.close();
    }
}

struct DynFile<R, W> {
    metadata: Metadata,
    rdf: R,
    wrf: W,
}
impl<R, W> File for DynFile<R, W>
where
    R: Fn() -> Result<Vec<u8>, LxError> + Send + Sync + 'static,
    W: Fn(&[u8]) -> Result<usize, LxError> + Send + Sync + 'static,
{
    fn open_vfd(self: Arc<Self>, flags: OpenFlags) -> Result<Vfd, LxError> {
        Ok(Vfd::new(self, flags))
    }
}
impl<R, W> VfdContent for DynFile<R, W>
where
    R: Fn() -> Result<Vec<u8>, LxError> + Send + Sync,
    W: Fn(&[u8]) -> Result<usize, LxError> + Send + Sync,
{
    fn read(&self, buf: &mut [u8], off: &mut i64) -> Result<usize, LxError> {
        let s = (self.rdf)()?;
        if *off >= s.len() as _ {
            return Ok(0);
        }
        let bytes_read = buf.len().min(s.len() - *off as usize);
        buf[..bytes_read].copy_from_slice(&s[(*off as _)..(*off as usize + bytes_read)]);
        *off += bytes_read as i64;
        Ok(bytes_read)
    }

    fn stat(&self) -> Result<Statx, LxError> {
        let mut stat = self.metadata.stat_template(StatxMask::all());

        stat.stx_mode.set_file_type(FileType::RegularFile);

        Ok(stat)
    }
}
impl<R, W> Debug for DynFile<R, W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DynFile")
    }
}

#[derive(Debug)]
struct Symlink {
    metadata: Metadata,
    target: Vec<u8>,
}
impl Symlink {
    fn solve(&self, lpath: LPath) -> VPath {
        symlink_abs(lpath, &self.target)
    }
}

#[derive(Debug)]
struct Metadata {
    xattrs: DashMap<Vec<u8>, Vec<u8>>,
    uid: AtomicU32,
    gid: AtomicU32,
    permbits: AtomicU16,
    atime: RwLock<Timespec>,
    btime: RwLock<Timespec>,
    ctime: RwLock<Timespec>,
    mtime: RwLock<Timespec>,
}
impl Metadata {
    fn new() -> Self {
        todo!()
    }

    fn kernfs(&self) -> bool {
        match self.xattrs.get(XATTR_KERNFS) {
            Some(val) if *val == b"1" => true,
            _ => false,
        }
    }

    fn set_kernfs(&self, value: bool) {
        if value {
            self.xattrs.insert(XATTR_KERNFS.to_vec(), b"1".to_vec());
        } else {
            self.xattrs.remove(XATTR_KERNFS);
        }
    }

    /// Generates a [`Statx`] template with this metadata.
    ///
    /// Note that the returned value is incomplete and have to be modified before returning to the user. The necessary
    /// modifications include:
    ///
    ///  - `stx_mode` should be OR-ed file type bits
    ///  - `stx_size` is 0, and should be set to the correct value
    ///  - `stx_blocks` is 0, and should be set to the correct value
    ///  - `stx_rdev_*` are not set
    ///
    /// The `inode` is determined by memory location of the [`Metadata`] object, which means, this should be called on instances
    /// inside a node on the heap.
    fn stat_template(&self, mask: StatxMask) -> Statx {
        Statx {
            stx_mask: StatxMask::all(),
            stx_blksize: BLOCK_SIZE,
            stx_attributes: StatxAttrs::empty(),
            stx_nlink: 0,
            stx_uid: self.uid.load(atomic::Ordering::Relaxed),
            stx_gid: self.gid.load(atomic::Ordering::Relaxed),
            stx_mode: FileMode(self.permbits.load(atomic::Ordering::Relaxed)),
            stx_ino: self as *const _ as usize as u64,
            stx_size: 0,
            stx_attributes_mask: 0,
            stx_atime: self.atime.read().unwrap().clone().into(),
            stx_btime: self.btime.read().unwrap().clone().into(),
            stx_ctime: self.ctime.read().unwrap().clone().into(),
            stx_mtime: self.mtime.read().unwrap().clone().into(),
            stx_blocks: 0,
            stx_rdev_major: 0,
            stx_rdev_minor: 0,
            stx_dev_major: 0,
            stx_dev_minor: 0,
            stx_mnt_id: 0,
            stx_dio_mem_align: BLOCK_SIZE,
            stx_dio_offset_align: BLOCK_SIZE,
            stx_subvol: 1,
            stx_atomic_write_unit_min: 1,
            stx_atomic_write_unit_max: BLOCK_SIZE,
            stx_atomic_write_segments_max: 1,
            stx_dio_read_offset_align: BLOCK_SIZE,
        }
    }
}
impl Clone for Metadata {
    fn clone(&self) -> Self {
        Self {
            xattrs: self.xattrs.clone(),
            uid: AtomicU32::new(self.uid.load(atomic::Ordering::Relaxed)),
            gid: AtomicU32::new(self.gid.load(atomic::Ordering::Relaxed)),
            permbits: AtomicU16::new(self.permbits.load(atomic::Ordering::Relaxed)),
            atime: RwLock::new(self.atime.read().unwrap().clone()),
            btime: RwLock::new(self.btime.read().unwrap().clone()),
            ctime: RwLock::new(self.ctime.read().unwrap().clone()),
            mtime: RwLock::new(self.mtime.read().unwrap().clone()),
        }
    }
}

/// A buffer for regular files. Supports sparse files.
#[derive(Debug)]
struct RegBuf {
    last_block_used: AtomicUsize,
    data: RwLock<Vec<Option<Box<[u8; BLOCK_SIZE as _]>>>>,
}
impl RegBuf {
    const fn new() -> Self {
        Self {
            last_block_used: AtomicUsize::new(0),
            data: RwLock::new(Vec::new()),
        }
    }

    fn blocks(&self) -> u64 {
        self.data
            .read()
            .unwrap()
            .iter()
            .filter(|block| block.is_some())
            .count() as _
    }

    fn size(&self) -> u64 {
        let data = self.data.read().unwrap();
        let blocks = data.len();
        if blocks == 0 {
            return 0;
        }
        ((blocks - 1) * BLOCK_SIZE as usize + self.last_block_used.load(atomic::Ordering::Relaxed))
            as u64
    }

    fn read_block(&self, id: u64, buf: &mut [u8]) -> usize {
        let data = self.data.read().unwrap();
        let Some(block) = data.get(id as usize) else {
            return 0;
        };
        let mut expected_len = BLOCK_SIZE;
        if id + 1 == data.len() as u64 {
            expected_len = self.last_block_used.load(atomic::Ordering::Relaxed) as _;
        }
        let actual_len = (expected_len as usize).min(buf.len()) as usize;
        let Some(block) = &*block else {
            buf.fill(0);
            return actual_len;
        };
        buf.copy_from_slice(&block[..actual_len]);
        actual_len
    }

    fn write_block(&self, id: u64, buf: &[u8]) -> usize {
        let mut data = self.data.write().unwrap();
        let write_len = (buf.len() as u64).min(BLOCK_SIZE as u64);
        if id as usize >= data.len() {
            data.resize(id as usize + 1, None);
            self.last_block_used
                .store(write_len as _, atomic::Ordering::Relaxed);
        }
        let block = data
            .get_mut(id as usize)
            .expect("reserved block should never be empty");

        let buf_is_block = buf.len() == BLOCK_SIZE as _;
        let buf_is_zeroed = buf.iter().all(|x| *x == 0);
        if block.is_none() && buf_is_zeroed {
            return write_len as _;
        }
        if block.is_some() && buf_is_zeroed && buf_is_block {
            *block = None;
        }

        if block.is_none() {
            *block = Some(Box::new([0; _]));
        }
        (block.as_mut().unwrap())[..write_len as _].copy_from_slice(&buf[..write_len as _]);

        write_len as _
    }
}
