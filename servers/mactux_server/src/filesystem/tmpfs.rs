//! A generic in-memory filesystem.

use crate::{
    app,
    filesystem::{
        VPath,
        vfs::{Filesystem, LPath, MakeFilesystem, NewlyOpen},
    },
    poll::PollToken,
    task::process::Process,
    util::{plain_seek, symlink_abs},
    vfd::{Stream, Vfd, VfdContent},
};
use crossbeam::atomic::AtomicCell;
use dashmap::DashMap;
use rustc_hash::FxBuildHasher;
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
    fs::{
        AccessFlags, Dirent64, Dirent64Hdr, FileMode, FileType, FsMagic, MountFlags, OpenFlags,
        OpenHow, OpenResolve, StatFs, StatFsFlags, Statx, StatxAttrs, StatxMask,
    },
    internal::mactux_ipc::CtrlOutput,
    io::{IoctlCmd, PollEvents, VfdAvailCtrl, Whence},
    time::Timespec,
};

/// Size of a block.
const BLOCK_SIZE: u32 = 4096;

/// A tmpfs tree.
#[derive(Debug)]
pub struct Tmpfs {
    root: Arc<Dir>,
    fs_magic: AtomicCell<FsMagic>,
}
impl Tmpfs {
    /// Creates a new [`Tmpfs`] instance.
    pub fn new() -> Result<Arc<Self>, LxError> {
        let metadata = Arc::new(Metadata::new());
        let vminor = (&raw const *metadata) as u32;
        metadata.vminor.store(vminor, atomic::Ordering::Relaxed);
        Ok(Arc::new(Self {
            root: Arc::new(Dir {
                metadata,
                children: DashMap::default(),
            }),
            fs_magic: AtomicCell::new(FsMagic::TMPFS_MAGIC),
        }))
    }

    pub fn set_fs_magic(&self, new: FsMagic) {
        self.fs_magic.store(new);
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
        for (n, dir_part) in dir_name.into_iter().enumerate() {
            let node = dir.children.get(&dir_part).ok_or(LxError::ENOENT)?.clone();
            dir = match node {
                Node::Dir(x) => x.clone(),
                Node::File(_) => return Err(LxError::ENOTDIR),
                Node::Symlink(symlink) => {
                    let mut dir_path = path.clone();
                    dir_path.relative.parts.truncate(n + 1);
                    let mut solved = symlink.solve(dir_path);
                    solved.parts.push(file_name);
                    return Ok(Location::MidSymlink(solved));
                }
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
    fn open(self: Arc<Self>, path: LPath, how: OpenHow) -> Result<NewlyOpen, LxError> {
        let map_virtual = |content| {
            NewlyOpen::Virtual(Vfd::new(
                Arc::new(WrapVfdContent {
                    content,
                    filesystem: self.clone(),
                }),
                how.flags(),
            ))
        };
        match self.locate(path.clone())? {
            Location::Direct(_, Some(node)) => match node {
                Node::Dir(dir) => dir.open_vfd(how.flags()).map(map_virtual),
                Node::File(file) => {
                    if how.flags().contains(OpenFlags::O_EXCL) {
                        return Err(LxError::EEXIST);
                    }
                    if how.flags().contains(OpenFlags::O_DIRECTORY) {
                        return Err(LxError::ENOTDIR);
                    }
                    if let Some(native) = file.open_native() {
                        return Ok(NewlyOpen::Native(
                            native.into_os_string().into_encoded_bytes(),
                        ));
                    }
                    Arc::clone(&file).open_vfd(how.flags()).map(map_virtual)
                }
                Node::Symlink(symlink) => {
                    if how.resolve.contains(OpenResolve::RESOLVE_NO_SYMLINKS) {
                        return Ok(NewlyOpen::Virtual(Vfd::new(symlink, how.flags())));
                    }
                    if how.flags().contains(OpenFlags::O_NOFOLLOW) {
                        return Err(LxError::ELOOP);
                    }
                    Process::current()
                        .mnt
                        .locate(&symlink.solve(path))?
                        .open(how)
                }
            },
            Location::Direct(dir, None) => {
                if !how.flags().contains(OpenFlags::O_CREAT) {
                    return Err(LxError::ENOENT);
                }
                if how.flags().contains(OpenFlags::O_DIRECTORY) || path.relative.slash_suffix {
                    return Err(LxError::ENOTDIR);
                }
                let mut mode = how.mode();
                mode.set_file_type(FileType::RegularFile);
                let file = Arc::new(Reg {
                    metadata: dir.metadata.fork(mode),
                    buf: RegBuf::new(),
                });
                dir.children.insert(
                    path.relative.parts.last().ok_or(LxError::EEXIST)?.clone(),
                    Node::File(file.clone()),
                );
                Ok(map_virtual(file.open_vfd(how.flags())?))
            }
            Location::MidSymlink(vpath) => Process::current().mnt.locate(&vpath)?.open(how),
        }
    }

    fn access(&self, path: LPath, mode: AccessFlags) -> Result<(), LxError> {
        match self.locate(path.clone())? {
            Location::Direct(_, Some(node)) => match node {
                Node::Dir(_) => Ok(()),
                Node::File(_) => Ok(()),
                Node::Symlink(symlink) => Process::current()
                    .mnt
                    .locate(&symlink.solve(path))?
                    .access(mode),
            },
            Location::Direct(_, None) => Err(LxError::ENOENT),
            Location::MidSymlink(vpath) => Process::current().mnt.locate(&vpath)?.access(mode),
        }
    }

    fn get_sock_path(&self, _: LPath, _: bool) -> Result<PathBuf, LxError> {
        Err(LxError::EINVAL)
    }

    fn link(&self, src: LPath, dst: LPath) -> Result<(), LxError> {
        let vlocation = |x| Process::current().mnt.locate(x);
        let src_location = self.locate(src.clone())?;
        let dst_location = self.locate(dst.clone())?;
        let src_node = match src_location {
            Location::Direct(_, Some(node)) => node,
            Location::Direct(_, None) => return Err(LxError::ENOENT),
            Location::MidSymlink(vpath) => {
                return vlocation(&vpath)?.link_to(vlocation(&dst.expand())?);
            }
        };
        match dst_location {
            Location::Direct(dir, None) => {
                dir.children.insert(
                    dst.relative.parts.last().ok_or(LxError::EEXIST)?.clone(),
                    src_node,
                );
                Ok(())
            }
            Location::Direct(_, Some(_)) => Err(LxError::ENOENT),
            Location::MidSymlink(vpath) => vlocation(&src.expand())?.link_to(vlocation(&vpath)?),
        }
    }

    fn mkdir(&self, path: LPath, mut mode: FileMode) -> Result<(), LxError> {
        match self.locate(path.clone())? {
            Location::Direct(_, Some(_)) => Err(LxError::EEXIST),
            Location::Direct(dir, None) => {
                mode.set_file_type(FileType::Directory);
                let child = Dir {
                    metadata: dir.metadata.fork(mode),
                    children: DashMap::new(),
                };
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
        let vlocation = |x| Process::current().mnt.locate(x);
        let src_location = self.locate(src.clone())?;
        let dst_location = self.locate(dst.clone())?;
        let src_filename = src.relative.parts.last().ok_or(LxError::EISDIR)?.clone();
        let dst_filename = dst.relative.parts.last().ok_or(LxError::EEXIST)?.clone();
        let src_node = match src_location {
            Location::Direct(dir, Some(node)) => {
                dir.children.remove(&src_filename);
                node
            }
            Location::Direct(_, None) => return Err(LxError::ENOENT),
            Location::MidSymlink(vpath) => {
                return vlocation(&vpath)?.rename_to(vlocation(&dst.expand())?);
            }
        };
        match dst_location {
            Location::Direct(dir, None) => {
                dir.children.insert(dst_filename, src_node);
                Ok(())
            }
            Location::Direct(_, Some(_)) => Err(LxError::ENOENT),
            Location::MidSymlink(vpath) => vlocation(&src.expand())?.rename_to(vlocation(&vpath)?),
        }
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
                let child = Symlink::fixed(content.to_vec());
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
                if matches!(node, Node::Dir(_)) {
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
                let metadata = dir.metadata.fork(mode);
                let child = match mode.file_type() {
                    FileType::BlockDevice | FileType::CharDevice => Arc::new(Dev {
                        metadata,
                        file_type: mode.file_type(),
                        dev,
                    }) as _,
                    _ => return Err(LxError::EINVAL),
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

    fn statfs(&self) -> Result<StatFs, LxError> {
        Ok(StatFs {
            f_type: self.fs_magic.load(),
            f_bsize: BLOCK_SIZE as _,
            f_blocks: 0,
            f_bfree: 0,
            f_bavail: 0,
            f_files: 0,
            f_ffree: 0,
            f_fsid: crate::util::fsid(self),
            f_namelen: 255,
            f_frsize: BLOCK_SIZE as _,
            f_flags: StatFsFlags::empty(),
            f_spare: [0; _],
        })
    }
}
impl Tmpfs {
    pub fn create_dynfile<R, W>(&self, path: VPath, obj: DynFile<R, W>) -> Result<(), LxError>
    where
        R: DynFileReadFn,
        W: DynFileWriteFn,
    {
        let lpath = LPath {
            mountpoint: VPath::parse(b"/"),
            relative: path.clone(),
        };
        match self.locate(lpath)? {
            Location::Direct(_, Some(_)) => Err(LxError::EEXIST),
            Location::Direct(dir, None) => {
                dir.children.insert(
                    path.parts.last().ok_or(LxError::EEXIST)?.clone(),
                    Node::File(Arc::new(obj)),
                );
                Ok(())
            }
            Location::MidSymlink(_) => Err(LxError::EXDEV),
        }
    }

    pub fn create_dynlink<F>(&self, path: VPath, f: F) -> Result<(), LxError>
    where
        F: Fn() -> Vec<u8> + Send + Sync + 'static,
    {
        let lpath = LPath {
            mountpoint: VPath::parse(b"/"),
            relative: path.clone(),
        };
        match self.locate(lpath)? {
            Location::Direct(_, Some(_)) => Err(LxError::EEXIST),
            Location::Direct(dir, None) => {
                dir.children.insert(
                    path.parts.last().ok_or(LxError::EEXIST)?.clone(),
                    Node::Symlink(Arc::new(Symlink::dynamic(f))),
                );
                Ok(())
            }
            Location::MidSymlink(_) => Err(LxError::EXDEV),
        }
    }

    pub fn rmdir_all(&self, path: VPath) -> Result<(), LxError> {
        let lpath = LPath {
            mountpoint: VPath::parse(b"/"),
            relative: path.clone(),
        };
        match self.locate(lpath)? {
            Location::Direct(parent, Some(Node::Dir(_))) => {
                parent
                    .children
                    .remove(path.parts.last().ok_or(LxError::EPERM)?);
                Ok(())
            }
            Location::Direct(_, Some(_)) => Err(LxError::ENOTDIR),
            Location::Direct(_, None) => Err(LxError::ENOENT),
            Location::MidSymlink(_) => Err(LxError::EXDEV),
        }
    }
}

pub struct MakeTmpfs;
impl MakeFilesystem for MakeTmpfs {
    fn make_filesystem(
        &self,
        _: &[u8],
        _: MountFlags,
        _: &[u8],
    ) -> Result<Arc<dyn Filesystem>, LxError> {
        Tmpfs::new().map(|x| x as _)
    }

    fn is_nodev(&self) -> bool {
        true
    }
}

struct WrapVfdContent {
    content: Arc<dyn VfdContent>,
    filesystem: Arc<dyn Filesystem>,
}
impl Stream for WrapVfdContent {
    fn read(&self, buf: &mut [u8], off: &mut i64) -> Result<usize, LxError> {
        self.content.read(buf, off)
    }

    fn write(&self, buf: &[u8], off: &mut i64) -> Result<usize, LxError> {
        self.content.write(buf, off)
    }

    fn seek(&self, orig_off: i64, whence: Whence, off: i64) -> Result<i64, LxError> {
        self.content.seek(orig_off, whence, off)
    }

    fn ioctl(&self, cmd: IoctlCmd, data: &[u8]) -> Result<CtrlOutput, LxError> {
        self.content.ioctl(cmd, data)
    }

    fn ioctl_query(&self, cmd: IoctlCmd) -> Result<VfdAvailCtrl, LxError> {
        self.content.ioctl_query(cmd)
    }

    fn poll(&self, interest: PollEvents) -> Result<PollToken, LxError> {
        self.content.poll(interest)
    }
}
impl VfdContent for WrapVfdContent {
    fn stat(&self, mask: StatxMask) -> Result<Statx, LxError> {
        self.content.stat(mask)
    }

    fn chmod(&self, mode: u16) -> Result<(), LxError> {
        self.content.chmod(mode)
    }

    fn chown(&self, uid: u32, gid: u32) -> Result<(), LxError> {
        self.content.chown(uid, gid)
    }

    fn dup(&self) -> Result<Arc<dyn VfdContent>, LxError> {
        self.content.dup().map(|content| {
            Arc::new(Self {
                content,
                filesystem: self.filesystem.clone(),
            }) as _
        })
    }

    fn get_socket(&self, create: bool) -> Result<PathBuf, LxError> {
        self.content.get_socket(create)
    }

    fn getdent(&self) -> Result<Option<Dirent64>, LxError> {
        self.content.getdent()
    }

    fn sync(&self) -> Result<(), LxError> {
        self.content.sync()
    }

    fn readlink(&self) -> Result<Vec<u8>, LxError> {
        self.content.readlink()
    }

    fn truncate(&self, size: u64) -> Result<(), LxError> {
        self.content.truncate(size)
    }

    fn utimens(&self, times: [Timespec; 2]) -> Result<(), LxError> {
        self.content.utimens(times)
    }

    fn filesystem(&self) -> Result<Arc<dyn Filesystem>, LxError> {
        Ok(self.filesystem.clone())
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
    fn open_vfd(self: Arc<Self>, flags: OpenFlags) -> Result<Arc<dyn VfdContent>, LxError>;
    fn open_native(&self) -> Option<PathBuf> {
        None
    }
}

struct Dir {
    metadata: Arc<Metadata>,
    children: DashMap<Vec<u8>, Node>,
}
impl File for Dir {
    fn open_vfd(self: Arc<Self>, _: OpenFlags) -> Result<Arc<dyn VfdContent>, LxError> {
        let mut iter: Vec<Dirent64> = self
            .children
            .iter()
            .filter_map(|p| {
                let vfd = match p.value().clone() {
                    Node::File(x) => x.open_vfd(OpenFlags::O_PATH),
                    Node::Dir(x) => x.open_vfd(OpenFlags::O_PATH),
                    Node::Symlink(x) => x.open_vfd(OpenFlags::O_PATH),
                };
                let stat =
                    match vfd.and_then(|x| x.stat(StatxMask::STATX_INO | StatxMask::STATX_TYPE)) {
                        Ok(x) => x,
                        Err(_) => return None,
                    };
                Some(Dirent64::new(
                    Dirent64Hdr {
                        d_ino: stat.stx_ino,
                        d_off: 0,
                        d_reclen: 0,
                        d_type: stat.stx_mode.file_type().into(),
                        _align: [0; _],
                    },
                    p.key().clone(),
                ))
            })
            .collect();
        iter.push(Dirent64::new(
            Dirent64Hdr {
                d_ino: self.metadata.stat_template(StatxMask::STATX_INO).stx_ino,
                d_off: 0,
                d_reclen: 0,
                d_type: FileType::Directory.into(),
                _align: [0; _],
            },
            b".".to_vec(),
        ));
        iter.push(Dirent64::new(
            Dirent64Hdr {
                d_ino: self.metadata.stat_template(StatxMask::STATX_INO).stx_ino - 1,
                d_off: 0,
                d_reclen: 0,
                d_type: FileType::Directory.into(),
                _align: [0; _],
            },
            b"..".to_vec(),
        ));
        Ok(Arc::new(DirFd {
            metadata: self.metadata.clone(),
            iter: Mutex::new(iter),
        }))
    }
}
impl Debug for Dir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dir")
            .field("metadata", &self.metadata)
            .field_with("children", |f| {
                let mut debug_map = f.debug_map();
                for child in self.children.iter() {
                    debug_map.entry(&String::from_utf8_lossy(child.key()), child.value());
                }
                debug_map.finish()
            })
            .finish()
    }
}

#[derive(Debug)]
struct DirFd {
    metadata: Arc<Metadata>,
    iter: Mutex<Vec<Dirent64>>,
}
impl Stream for DirFd {}
impl VfdContent for DirFd {
    fn getdent(&self) -> Result<Option<Dirent64>, LxError> {
        Ok(self.iter.lock().unwrap().pop())
    }

    fn stat(&self, mask: StatxMask) -> Result<Statx, LxError> {
        let mut statx = self.metadata.stat_template(mask);

        statx.stx_mode.set_file_type(FileType::Directory);

        statx.stx_size = BLOCK_SIZE as _;
        statx.stx_blocks = 1;

        Ok(statx)
    }
}

#[derive(Debug)]
struct Reg {
    metadata: Arc<Metadata>,
    buf: RegBuf,
}
impl File for Reg {
    fn open_vfd(self: Arc<Self>, _: OpenFlags) -> Result<Arc<dyn VfdContent>, LxError> {
        Ok(self.clone())
    }
}
impl Stream for Reg {
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
}
impl VfdContent for Reg {
    fn stat(&self, mask: StatxMask) -> Result<Statx, LxError> {
        let mut stat = self.metadata.stat_template(mask);

        stat.stx_size = self.buf.size();
        stat.stx_blocks = self.buf.blocks() * (BLOCK_SIZE as u64 / 512);

        stat.stx_mode.set_file_type(FileType::RegularFile);

        Ok(stat)
    }
}

#[derive(Debug)]
struct Dev {
    metadata: Arc<Metadata>,
    file_type: FileType,
    dev: DeviceNumber,
}
impl File for Dev {
    fn open_native(&self) -> Option<PathBuf> {
        let device = match self.file_type {
            FileType::CharDevice => app().devices.find_chr(self.dev).ok()?,
            FileType::BlockDevice => app().devices.find_blk(self.dev).ok()?,
            _ => unreachable!(),
        };
        device.macos_device()
    }

    fn open_vfd(self: Arc<Self>, flags: OpenFlags) -> Result<Arc<dyn VfdContent>, LxError> {
        let device = if flags.contains(OpenFlags::O_PATH) {
            None
        } else {
            let x = match self.file_type {
                FileType::CharDevice => app().devices.find_chr(self.dev)?.open(flags)?,
                FileType::BlockDevice => app().devices.find_blk(self.dev)?.open(flags)?,
                _ => unreachable!(),
            };
            Some(x)
        };
        Ok(Arc::new(DevFd {
            metadata: self.metadata.clone(),
            file_type: self.file_type,
            device,
            devnum: self.dev,
        }))
    }
}

struct DevFd {
    metadata: Arc<Metadata>,
    file_type: FileType,
    device: Option<Arc<dyn Stream + Send + Sync>>,
    devnum: DeviceNumber,
}
impl Stream for DevFd {
    fn read(&self, buf: &mut [u8], off: &mut i64) -> Result<usize, LxError> {
        self.device.as_ref().ok_or(LxError::EBADF)?.read(buf, off)
    }

    fn write(&self, buf: &[u8], off: &mut i64) -> Result<usize, LxError> {
        self.device.as_ref().ok_or(LxError::EBADF)?.write(buf, off)
    }

    fn seek(&self, orig_off: i64, whence: Whence, off: i64) -> Result<i64, LxError> {
        self.device
            .as_ref()
            .ok_or(LxError::EBADF)?
            .seek(orig_off, whence, off)
    }

    fn ioctl_query(&self, cmd: IoctlCmd) -> Result<VfdAvailCtrl, LxError> {
        self.device.as_ref().ok_or(LxError::EBADF)?.ioctl_query(cmd)
    }

    fn ioctl(&self, cmd: IoctlCmd, data: &[u8]) -> Result<CtrlOutput, LxError> {
        self.device.as_ref().ok_or(LxError::EBADF)?.ioctl(cmd, data)
    }
}
impl VfdContent for DevFd {
    fn stat(&self, mask: StatxMask) -> Result<Statx, LxError> {
        let mut statx = self.metadata.stat_template(mask);

        statx.stx_mode.set_file_type(self.file_type);

        statx.stx_rdev_major = self.devnum.major();
        statx.stx_rdev_minor = self.devnum.minor();

        Ok(statx)
    }
}

pub trait DynFileReadFn: Fn() -> Result<Vec<u8>, LxError> + Send + Sync + 'static {}
impl<T: Fn() -> Result<Vec<u8>, LxError> + Send + Sync + 'static> DynFileReadFn for T {}

pub trait DynFileWriteFn: Fn(Vec<u8>) -> Result<usize, LxError> + Send + Sync + 'static {}
impl<T: Fn(Vec<u8>) -> Result<usize, LxError> + Send + Sync + 'static> DynFileWriteFn for T {}

pub struct DynFile<R, W> {
    metadata: Metadata,
    rdf: R,
    wrf: W,
}
impl<R, W> DynFile<R, W> {
    pub fn new(rdf: R, wrf: W, permbits: u16) -> Self {
        let metadata = Metadata::new();
        metadata.permbits.store(permbits, atomic::Ordering::Relaxed);
        Self { rdf, wrf, metadata }
    }
}
impl<R, W> File for DynFile<R, W>
where
    R: DynFileReadFn,
    W: DynFileWriteFn,
{
    fn open_vfd(self: Arc<Self>, flags: OpenFlags) -> Result<Arc<dyn VfdContent>, LxError> {
        if flags.contains(OpenFlags::O_APPEND) {
            return Err(LxError::EINVAL);
        }

        Ok(self.clone())
    }
}
impl<R, W> Stream for DynFile<R, W>
where
    R: DynFileReadFn,
    W: DynFileWriteFn,
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

    fn write(&self, buf: &[u8], _: &mut i64) -> Result<usize, LxError> {
        (self.wrf)(buf.to_vec())
    }

    fn seek(&self, orig_off: i64, whence: Whence, off: i64) -> Result<i64, LxError> {
        plain_seek(orig_off, -1, whence, off)
    }
}
impl<R, W> VfdContent for DynFile<R, W>
where
    R: DynFileReadFn,
    W: DynFileWriteFn,
{
    fn stat(&self, mask: StatxMask) -> Result<Statx, LxError> {
        let mut stat = self.metadata.stat_template(mask);

        stat.stx_mode.set_file_type(FileType::RegularFile);

        stat.stx_size = BLOCK_SIZE as _;
        stat.stx_blocks = 1;

        Ok(stat)
    }
}
impl<R, W> Debug for DynFile<R, W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DynFile")
    }
}

struct Symlink {
    metadata: Metadata,
    target: Box<dyn Fn() -> Vec<u8> + Send + Sync + 'static>,
}
impl Symlink {
    fn fixed(target: Vec<u8>) -> Self {
        let metadata = Metadata::new();
        metadata.permbits.store(0o777, atomic::Ordering::Relaxed);
        Self {
            metadata,
            target: Box::new(move || target.clone()),
        }
    }

    fn dynamic(f: impl Fn() -> Vec<u8> + Send + Sync + 'static) -> Self {
        let metadata = Metadata::new();
        metadata.permbits.store(0o777, atomic::Ordering::Relaxed);
        Self {
            metadata,
            target: Box::new(f),
        }
    }
}
impl Debug for Symlink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Symlink")
            .field("metadata", &self.metadata)
            .finish_non_exhaustive()
    }
}
impl File for Symlink {
    fn open_vfd(self: Arc<Self>, _: OpenFlags) -> Result<Arc<dyn VfdContent>, LxError> {
        Ok(self.clone())
    }
}
impl Stream for Symlink {}
impl VfdContent for Symlink {
    fn readlink(&self) -> Result<Vec<u8>, LxError> {
        Ok((self.target)().into())
    }

    fn stat(&self, mask: StatxMask) -> Result<Statx, LxError> {
        let mut stat = self.metadata.stat_template(mask);

        stat.stx_mode.set_file_type(FileType::Symlink);

        stat.stx_size = (self.target)().len() as _;
        stat.stx_blocks = 1;

        Ok(stat)
    }
}
impl Symlink {
    fn solve(&self, lpath: LPath) -> VPath {
        symlink_abs(lpath, &(self.target)())
    }
}

#[derive(Debug)]
pub struct Metadata {
    xattrs: DashMap<Vec<u8>, Vec<u8>, FxBuildHasher>,
    uid: AtomicU32,
    gid: AtomicU32,
    permbits: AtomicU16,
    atime: RwLock<Timespec>,
    btime: RwLock<Timespec>,
    ctime: RwLock<Timespec>,
    mtime: RwLock<Timespec>,
    vminor: AtomicU32,
}
impl Metadata {
    fn new() -> Self {
        Self {
            xattrs: DashMap::default(),
            uid: AtomicU32::new(0),
            gid: AtomicU32::new(0),
            permbits: AtomicU16::new(0o777),
            atime: RwLock::new(Timespec::now()),
            btime: RwLock::new(Timespec::now()),
            ctime: RwLock::new(Timespec::now()),
            mtime: RwLock::new(Timespec::now()),
            vminor: AtomicU32::new(0),
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
            stx_mask: mask,
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
            stx_dev_minor: self.vminor.load(atomic::Ordering::Relaxed),
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

    fn fork(&self, mode: FileMode) -> Arc<Self> {
        let permbits = match mode.file_type() {
            FileType::Directory => 0o777,
            _ => 0o666,
        };
        Arc::new(Self {
            xattrs: self.xattrs.clone(),
            uid: AtomicU32::new(self.uid.load(atomic::Ordering::Relaxed)),
            gid: AtomicU32::new(self.gid.load(atomic::Ordering::Relaxed)),
            permbits: AtomicU16::new(permbits),
            atime: RwLock::new(Timespec::now()),
            btime: RwLock::new(Timespec::now()),
            ctime: RwLock::new(Timespec::now()),
            mtime: RwLock::new(Timespec::now()),
            vminor: AtomicU32::new(self.vminor.load(atomic::Ordering::Relaxed)),
        })
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
