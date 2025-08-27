//! The virtual filesystem.

use crate::{process::PidNamespace, vfd::VirtualFd};
use async_trait::async_trait;
use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    path::PathBuf,
    sync::{
        Arc, OnceLock, RwLock,
        atomic::{self, AtomicU64},
    },
};
use structures::{
    error::LxError,
    fs::{AccessFlags, OpenFlags, UmountFlags},
};

/// A path used in VFS.
#[derive(Clone)]
pub struct VfsPath<'a> {
    pub begin_slash: bool,
    pub segments: Vec<Cow<'a, [u8]>>,
    pub end_slash: bool,
}
impl<'a> VfsPath<'a> {
    /// Gets a [`VfsPath`] instance from human-readable string.
    pub fn from_bytes(mut s: &'a [u8]) -> Self {
        if s.is_empty() {
            s = b".";
        }
        Self {
            begin_slash: s.starts_with(b"/"),
            segments: s
                .split(|x| *x == b'/')
                .map(Cow::Borrowed)
                .skip_while(|x| x.is_empty())
                .collect(),
            end_slash: s.ends_with(b"/"),
        }
    }

    /// Converts the [`VfsPath`] instance to have `'static` lifetime.
    pub fn to_storable(&self) -> VfsPath<'static> {
        let mut segments: Vec<Cow<'static, [u8]>> = Vec::with_capacity(self.segments.len());
        for i in &self.segments {
            segments.push(Cow::Owned(i.to_vec()));
        }
        VfsPath {
            begin_slash: self.begin_slash,
            segments,
            end_slash: self.end_slash,
        }
    }

    /// Returns true if this is an absolute path.
    pub fn is_absolute(&self) -> bool {
        self.begin_slash
    }

    /// Returns true if this path force indicates a directory rather than a regular file.
    pub fn indicates_dir(&self) -> bool {
        self.end_slash
    }

    /// Removes all `.`-s, `..`-s from the VFS path, returning an absolute path,
    ///
    /// # Panics
    /// Panics if the path is relative.
    pub fn clearize(&self) -> Self {
        let mut dst = Vec::with_capacity(self.segments.len());
        for i in &self.segments {
            if &i[..] == &b".."[..] {
                dst.pop();
            } else if &i[..] == &b"."[..] {
                continue;
            } else if !i.is_empty() {
                dst.push(i.clone());
            }
        }
        Self {
            begin_slash: true,
            end_slash: self.end_slash && !dst.is_empty(),
            segments: dst,
        }
    }

    /// Converts the path to a human-readable format.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        if self.begin_slash {
            result.push(b'/');
        }
        for (n, i) in self.segments.iter().enumerate() {
            i.iter().for_each(|ch| result.push(*ch));
            if n != self.segments.len() - 1 {
                result.push(b'/');
            }
        }
        if self.end_slash {
            result.push(b'/');
        }
        result
    }
}
impl Debug for VfsPath<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.to_bytes()))
    }
}
impl Display for VfsPath<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.to_bytes()))
    }
}

/// A mount namespace.
#[derive(Debug)]
pub struct MountNamespace {
    id: AtomicU64,
    mounts: RwLock<Vec<Mount>>,
}
impl MountNamespace {
    /// Creates a new, empty mount namespace.
    ///
    /// Note that this does not allocate an ID. Allocate ID before use!
    pub fn new() -> Self {
        MountNamespace {
            id: AtomicU64::new(0),
            mounts: RwLock::new(Vec::with_capacity(8)),
        }
    }

    /// Returns the singleton initial mount namespace.
    pub fn initial() -> Arc<Self> {
        static SINGLETON: OnceLock<Arc<MountNamespace>> = OnceLock::new();

        SINGLETON
            .get_or_init(|| Arc::new(MountNamespace::new()))
            .clone()
    }

    /// Returns ID of the mount namespace.
    ///
    /// # Panics
    /// This function would panic if the ID has not been initialized.
    pub fn id(&self) -> u64 {
        let id = self.id.load(atomic::Ordering::Relaxed);
        assert_ne!(id, 0);
        id
    }

    /// Mounts a mountable object to a specific path.
    pub async fn mount(
        &self,
        mountpoint: VfsPath<'static>,
        mount_obj: Arc<dyn Mountable>,
    ) -> Result<(), LxError> {
        debug_assert!(mountpoint.is_absolute());
        let mountpoint = mountpoint.clearize();
        let mountpoint_exists = self.access(&mountpoint, AccessFlags::F_OK).await.is_ok();
        let mountpoint_is_root = mountpoint.segments.is_empty();
        if !mountpoint_exists && !mountpoint_is_root {
            return Err(LxError::ENOENT);
        }
        self.mounts.write().unwrap().push(Mount {
            mountpoint,
            mount_obj,
        });
        Ok(())
    }

    pub fn umount(&self, path: &VfsPath<'_>, flags: UmountFlags) -> Result<(), LxError> {
        let path = path.clearize();
        let mut mounts = self.mounts.write().unwrap();
        let mut nelem = None;
        for (n, mount) in mounts.iter().rev().enumerate() {
            if (path.segments.len() > mount.mountpoint.segments.len()) && (path.segments[..mount.mountpoint.segments.len()] == mount.mountpoint.segments) {
                return Err(LxError::EBUSY);
            }
            if mount.mountpoint.segments == path.segments {
                nelem = Some(n);
                break;
            }
        }
        if let Some(i) = nelem {
            mounts.remove(i);
            Ok(())
        } else {
            Err(LxError::EINVAL)
        }
    }

    pub async fn open(
        &self,
        full_path: &VfsPath<'_>,
        flags: OpenFlags,
        mode: u32,
    ) -> Result<NewlyOpen, LxError> {
        let (mountable, relpath) = self.find_node(&full_path)?;
        mountable.open(&relpath, flags, mode).await.inspect(|x| {
            if let NewlyOpen::Virtual(vfd) = x {
                _ = vfd.set_orig_path(full_path.to_storable());
            }
        })
    }

    pub async fn access(&self, full_path: &VfsPath<'_>, mode: AccessFlags) -> Result<(), LxError> {
        let (mountable, relpath) = self.find_node(&full_path)?;
        mountable.access(&relpath, mode).await
    }

    pub async fn unlink(&self, full_path: &VfsPath<'_>) -> Result<(), LxError> {
        let (mountable, relpath) = self.find_node(&full_path)?;
        mountable.unlink(&relpath).await
    }

    pub async fn rmdir(&self, full_path: &VfsPath<'_>) -> Result<(), LxError> {
        let (mountable, relpath) = self.find_node(&full_path)?;
        mountable.rmdir(&relpath).await
    }

    pub async fn symlink(&self, full_path: &VfsPath<'_>, content: &[u8]) -> Result<(), LxError> {
        let (mountable, relpath) = self.find_node(&full_path)?;
        mountable.symlink(&relpath, content).await
    }

    pub async fn mkdir(&self, full_path: &VfsPath<'_>, mode: u32) -> Result<(), LxError> {
        let (mountable, relpath) = self.find_node(&full_path)?;
        mountable.mkdir(&relpath, mode).await
    }

    pub async fn rename(&self, src: &VfsPath<'_>, dst: &VfsPath<'_>) -> Result<(), LxError> {
        let (src_mountable, src_relpath) = self.find_node(&src)?;
        let (dst_mountable, dst_relpath) = self.find_node(&dst)?;
        if !Arc::ptr_eq(&src_mountable, &dst_mountable) {
            return Err(LxError::EXDEV);
        }
        src_mountable.rename(&src_relpath, &dst_relpath).await
    }

    pub async fn get_sock_path(
        &self,
        full_path: &VfsPath<'_>,
        create: bool,
    ) -> Result<Vec<u8>, LxError> {
        let (mountable, relpath) = self.find_node(&full_path)?;
        mountable
            .get_sock_path(&relpath, create)
            .await
            .map(|x| x.into_os_string().into_encoded_bytes())
    }

    /// Finds a node by full path in the mount namespace, returning the mountable object and relative path on success.
    fn find_node(
        &self,
        full_path: &VfsPath,
    ) -> Result<(Arc<dyn Mountable>, VfsPath<'static>), LxError> {
        let full_path = full_path.clearize();
        let mounts = self.mounts.read().unwrap();
        for mount in mounts.iter().rev() {
            if full_path.segments.len() < mount.mountpoint.segments.len() {
                continue;
            }

            if full_path.segments[..mount.mountpoint.segments.len()] == mount.mountpoint.segments {
                let segments: Vec<_> = full_path.segments[mount.mountpoint.segments.len()..]
                    .iter()
                    .map(|x| Cow::Owned(x.clone().into_owned()))
                    .collect();
                return Ok((
                    mount.mount_obj.clone(),
                    VfsPath {
                        begin_slash: true,
                        end_slash: full_path.end_slash && !segments.is_empty(),
                        segments,
                    },
                ));
            }
        }
        Err(LxError::ENOENT)
    }
}

/// A mounted filesystem.
pub struct Mount {
    mountpoint: VfsPath<'static>,
    mount_obj: Arc<dyn Mountable>,
}
impl Debug for Mount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mount")
            .field("mountpoint", &self.mountpoint)
            .finish_non_exhaustive()
    }
}

/// A mountable object.
#[async_trait]
pub trait Mountable: Send + Sync {
    async fn open(
        self: Arc<Self>,
        path: &VfsPath,
        flags: OpenFlags,
        mode: u32,
    ) -> Result<NewlyOpen, LxError>;
    async fn access(&self, path: &VfsPath, mode: AccessFlags) -> Result<(), LxError>;
    async fn unlink(&self, path: &VfsPath) -> Result<(), LxError>;
    async fn rmdir(&self, path: &VfsPath) -> Result<(), LxError>;
    async fn symlink(&self, dst: &VfsPath, content: &[u8]) -> Result<(), LxError>;
    async fn mkdir(&self, path: &VfsPath, mode: u32) -> Result<(), LxError>;
    async fn get_sock_path(&self, path: &VfsPath, create: bool) -> Result<PathBuf, LxError>;
    async fn rename(&self, src: &VfsPath, dst: &VfsPath) -> Result<(), LxError>;
    async fn mount_bind(&self, path: &VfsPath) -> Result<Box<dyn Mountable>, LxError>;
}

/// A newly-open file.
#[derive(Clone)]
pub enum NewlyOpen {
    AtNative(PathBuf),
    Virtual(Arc<VirtualFd>),
}
impl Debug for NewlyOpen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NewlyOpen::AtNative(path) => write!(f, "AtNative({})", path.display()),
            NewlyOpen::Virtual(_) => write!(f, "Virtual"),
        }
    }
}

/// Representation of a device name used in `mount`.
#[derive(Debug, Clone)]
pub enum MountDev {
    Freeform(String),
}

/// Gets mountable object.
pub async fn mountable(fs: &str, dev: MountDev, opts: &str) -> Result<Arc<dyn Mountable>, LxError> {
    match fs {
        "nativefs" => super::nativefs::mountable(dev, opts),
        "devtmpfs" => super::devtmpfs::mountable(),
        "procfs" => crate::process::InitPid::instance()
            .procfs()
            .ok_or(LxError::EPERM),
        "sysfs" => super::sysfs::mountable(),
        _ => Err(LxError::ENOENT),
    }
}
