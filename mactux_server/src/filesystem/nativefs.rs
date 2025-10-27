//! A filesystem that maps all Linux filesystem operations to the underlying macOS one.

use crate::{
    file::{Ioctl, Stream},
    filesystem::{
        VPath,
        vfs::{Filesystem, LPath, NewlyOpen},
    },
    task::process::Process,
    util::symlink_abs,
    vfd::{Vfd, VfdContent},
};
use libc::c_int;
use std::{
    collections::VecDeque,
    ffi::{CString, OsString},
    fmt::Debug,
    fs::ReadDir,
    os::unix::{ffi::OsStringExt, fs::DirEntryExt},
    path::Path,
    sync::{Arc, Mutex},
};
use structures::{
    ToApple,
    device::DeviceNumber,
    error::LxError,
    fs::{AccessFlags, Dirent64, Dirent64Hdr, DirentType, FileMode, OpenFlags, Statx},
};

/// A nativefs mount.
pub struct NativeFs {
    base: NBase,
}
impl NativeFs {
    pub fn new(dev: &[u8], flags: u64) -> Result<Arc<Self>, LxError> {
        let dev = str::from_utf8(dev).map_err(|_| LxError::EINVAL)?;
        let path = dev.strip_prefix("native=").ok_or(LxError::EACCES)?;
        let base = NBase::new(Path::new(path))?;
        Ok(Arc::new(Self { base }))
    }
}
impl Filesystem for NativeFs {
    fn open(
        self: Arc<Self>,
        path: LPath,
        flags: OpenFlags,
        mode: FileMode,
    ) -> Result<NewlyOpen, LxError> {
        match NPath::resolve(&self.base, path)? {
            NPath::Direct(dst) => unsafe {
                let mut statbuf = std::mem::zeroed();
                match posix_result(libc::lstat(dst.as_ptr(), &mut statbuf)) {
                    Ok(()) | Err(LxError::ENOENT) => (),
                    Err(err) => return Err(err),
                }
                if statbuf.st_mode & libc::S_IFMT == libc::S_IFDIR {
                    let vfd_content = Arc::new(DirFd::new(dst, statbuf)?);
                    return Ok(NewlyOpen::Virtual(Vfd::new(vfd_content, flags)));
                }
                Ok(NewlyOpen::Native(dst.into_bytes()))
            },
            NPath::HasSymlink(symexpr) => Process::current()
                .mnt
                .locate(&symexpr.into_vpath())?
                .open(flags, mode),
            NPath::IsSymlink(_, content) => {
                if flags.contains(OpenFlags::O_NOFOLLOW) {
                    return Err(LxError::ELOOP);
                }
                Process::current().mnt.locate(&content)?.open(flags, mode)
            }
        }
    }

    fn access(&self, path: LPath, mode: AccessFlags) -> Result<(), LxError> {
        match NPath::resolve(&self.base, path)? {
            NPath::Direct(dst) => unsafe {
                posix_result(libc::access(dst.as_ptr(), mode.to_apple()?))
            },
            NPath::HasSymlink(symexpr) => Process::current()
                .mnt
                .locate(&symexpr.into_vpath())?
                .access(mode),
            NPath::IsSymlink(_, content) => Process::current().mnt.locate(&content)?.access(mode),
        }
    }

    fn symlink(&self, dst: LPath, content: &[u8]) -> Result<(), LxError> {
        match NPath::resolve(&self.base, dst)? {
            NPath::Direct(dst) | NPath::IsSymlink(dst, _) => unsafe {
                let content = bytes_to_cstring(content.to_vec())?;
                posix_result(libc::symlink(content.as_ptr(), dst.as_ptr()))
            },
            NPath::HasSymlink(symexpr) => Process::current()
                .mnt
                .locate(&symexpr.into_vpath())?
                .symlink(content),
        }
    }

    fn rmdir(&self, path: LPath) -> Result<(), LxError> {
        match NPath::resolve(&self.base, path.clone())? {
            NPath::Direct(dst) => unsafe { posix_result(libc::rmdir(dst.as_ptr())) },
            NPath::HasSymlink(symexpr) => Process::current()
                .mnt
                .locate(&symexpr.into_vpath())?
                .rmdir(),
            NPath::IsSymlink(_, content) => {
                if !path.relative.slash_suffix {
                    return Err(LxError::ENOTDIR);
                }
                Process::current().mnt.locate(&content)?.rmdir()
            }
        }
    }

    fn get_sock_path(&self, path: LPath, create: bool) -> Result<std::path::PathBuf, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn link(&self, src: LPath, dst: LPath) -> Result<(), LxError> {
        todo!()
    }

    fn mkdir(&self, path: LPath, mode: FileMode) -> Result<(), LxError> {
        match NPath::resolve(&self.base, path)? {
            NPath::Direct(dst) => unsafe { posix_result(libc::mkdir(dst.as_ptr(), mode.0 as _)) },
            NPath::HasSymlink(symexpr) => Process::current()
                .mnt
                .locate(&symexpr.into_vpath())?
                .mkdir(mode),
            NPath::IsSymlink(_, _) => Err(LxError::EEXIST),
        }
    }

    fn rename(&self, src: LPath, dst: LPath) -> Result<(), LxError> {
        todo!()
    }

    fn unlink(&self, path: LPath) -> Result<(), LxError> {
        match NPath::resolve(&self.base, path)? {
            NPath::Direct(dst) | NPath::IsSymlink(dst, _) => unsafe {
                posix_result(libc::unlink(dst.as_ptr()))
            },
            NPath::HasSymlink(symexpr) => Process::current()
                .mnt
                .locate(&symexpr.into_vpath())?
                .unlink(),
        }
    }

    fn mknod(&self, path: LPath, mode: FileMode, dev: DeviceNumber) -> Result<(), LxError> {
        todo!()
    }
}

enum NPath {
    /// The native path is to be accessed directly.
    Direct(CString),

    /// The native path has a symlink in one of its parts. The path is splited into two parts at its first symlink. The
    /// first element is the content of the first symlink, while the second element reserves the original content of the
    /// second part.
    HasSymlink(SymlinkExpression),

    /// The native path has only one symlink in one of its parts, which is itself. Returns both native path of the symlink
    /// and content of the symlink.
    IsSymlink(CString, VPath),
}
impl NPath {
    /// Resolves a vpath to an npath, relative to nbase.
    pub fn resolve(nbase: &NBase, lpath: LPath) -> Result<Self, LxError> {
        debug_assert!(lpath.relative.slash_prefix);

        let xvpath = bytes_to_cstring(lpath.relative.express())?;
        let full_path = bytes_to_cstring([nbase.path.clone(), lpath.relative.express()].concat())?;

        unsafe {
            match libc::faccessat(nbase.dirfd, xvpath.as_ptr(), libc::F_OK, 0x2000 | 0x800) {
                -1 => match LxError::last_apple_error() {
                    LxError::ELOOP => Self::_resolve_symlink(nbase, lpath.clone()),
                    _ => Ok(Self::Direct(full_path)),
                },
                _ => {
                    if let Ok(content) =
                        readlinkat(nbase.dirfd, bytes_to_cstring(lpath.relative.express())?)
                    {
                        return Ok(Self::IsSymlink(full_path, symlink_abs(lpath, &content)));
                    }
                    Ok(Self::Direct(full_path))
                }
            }
        }
    }

    fn _resolve_symlink(nbase: &NBase, mut first: LPath) -> Result<Self, LxError> {
        let mut second = VPath {
            slash_prefix: true,
            parts: Vec::new(),
            slash_suffix: first.relative.slash_suffix,
        };
        let mut second_parts = VecDeque::new();
        loop {
            let xfirst = bytes_to_cstring(first.relative.express())?;
            unsafe {
                match libc::faccessat(nbase.dirfd, xfirst.as_ptr(), libc::F_OK, 0x2000 | 0x800) {
                    -1 => match LxError::last_apple_error() {
                        LxError::ELOOP => {
                            let element = first.relative.parts.pop().ok_or(LxError::EIO)?;
                            second_parts.push_front(element);
                            continue;
                        }
                        other => return Err(other),
                    },
                    _ => break,
                };
            }
        }
        let first_content = readlinkat(nbase.dirfd, bytes_to_cstring(first.relative.express())?)?;
        second.parts = second_parts.into();
        Ok(Self::HasSymlink(SymlinkExpression(
            symlink_abs(first, &first_content),
            second,
        )))
    }
}

#[derive(Clone)]
struct SymlinkExpression(VPath, VPath);
impl SymlinkExpression {
    fn into_vpath(mut self) -> VPath {
        self.0.parts.append(&mut self.1.parts);
        self.0.slash_suffix = self.1.slash_suffix;
        self.0
    }
}

pub struct NBase {
    path: Vec<u8>,
    dirfd: c_int,
}
impl NBase {
    pub fn new(path: &Path) -> Result<Self, LxError> {
        let mut path = std::fs::canonicalize(path)?
            .into_os_string()
            .into_encoded_bytes();
        while let Some(b'/') = path.last().copied() {
            path.pop();
        }
        let dirfd = unsafe {
            let c_path = bytes_to_cstring(path.clone())?;
            match libc::open(c_path.as_ptr(), libc::O_DIRECTORY | libc::O_SEARCH) {
                -1 => return Err(LxError::last_apple_error()),
                fd => fd,
            }
        };
        Ok(Self { path, dirfd })
    }
}
impl Debug for NBase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from_utf8_lossy(&self.path).fmt(f)
    }
}

struct DirFd {
    read_dir: Mutex<ReadDir>,
    statx: Statx,
    dotself: Mutex<Vec<Dirent64>>,
}
impl DirFd {
    fn new(path: CString, statbuf: libc::stat) -> Result<Self, LxError> {
        let statx = Statx::from_apple(statbuf);
        let path = OsString::from_vec(path.into_bytes());
        let read_dir = Mutex::new(std::fs::read_dir(Path::new(&path))?);
        let dot = Dirent64::new(
            Dirent64Hdr {
                d_ino: statx.stx_ino,
                d_off: 0,
                d_reclen: 0,
                d_type: DirentType::DT_DIR,
                _align: [0; _],
            },
            b".".to_vec(),
        );
        let dotdot = Dirent64::new(
            Dirent64Hdr {
                d_ino: statx.stx_ino - 1,
                d_off: 0,
                d_reclen: 0,
                d_type: DirentType::DT_DIR,
                _align: [0; _],
            },
            b"..".to_vec(),
        );
        Ok(Self {
            read_dir,
            statx,
            dotself: Mutex::new(vec![dot, dotdot]),
        })
    }
}
impl Stream for DirFd {}
impl Ioctl for DirFd {}
impl VfdContent for DirFd {
    fn getdent(&self) -> Result<Option<Dirent64>, LxError> {
        if let Some(entry) = self.dotself.lock().unwrap().pop() {
            return Ok(Some(entry));
        }

        match self.read_dir.lock().unwrap().next() {
            Some(Ok(entry)) => {
                let filename = entry.file_name().into_encoded_bytes();
                let d_type = entry
                    .file_type()
                    .map(DirentType::from_std)
                    .unwrap_or(DirentType::DT_UNKNOWN);
                let hdr = Dirent64Hdr {
                    d_ino: entry.ino(),
                    d_off: 0,
                    d_reclen: 0,
                    d_type,
                    _align: [0; _],
                };
                Ok(Some(Dirent64::new(hdr, filename)))
            }
            Some(Err(err)) => Err(LxError::from(err)),
            None => Ok(None),
        }
    }

    fn stat(&self) -> Result<Statx, LxError> {
        Ok(self.statx.clone())
    }
}

fn bytes_to_cstring(mut data: Vec<u8>) -> Result<CString, LxError> {
    data.push(0);
    CString::from_vec_with_nul(data).map_err(|_| LxError::EINVAL)
}

fn readlinkat(dirfd: c_int, path: CString) -> Result<Vec<u8>, LxError> {
    let mut buf = vec![0u8; libc::PATH_MAX as _];
    unsafe {
        let nbytes =
            match libc::readlinkat(dirfd, path.as_ptr(), buf.as_mut_ptr().cast(), buf.len()) {
                -1 => return Err(LxError::EIO),
                n => n,
            };
        buf.resize(nbytes as _, 0);
    }
    Ok(buf)
}

fn posix_result(value: c_int) -> Result<(), LxError> {
    match value {
        -1 => Err(LxError::last_apple_error()),
        _ => Ok(()),
    }
}
