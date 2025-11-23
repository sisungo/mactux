//! The VFS abstraction layer.

use crate::{app, filesystem::VPath, vfd::Vfd};
use rustc_hash::FxHashMap;
use std::{
    fmt::Write,
    path::PathBuf,
    sync::{Arc, RwLock},
};
use structures::{
    device::DeviceNumber,
    error::LxError,
    fs::{AccessFlags, FileMode, MountFlags, OpenFlags, OpenHow, OpenResolve, StatFs, UmountFlags},
};

/// Registry of all supported mountable filesystems in the kernel.
pub struct FsRegistry(FxHashMap<&'static str, Box<dyn MakeFilesystem>>);
impl FsRegistry {
    /// Creates a new filesystem registry.
    ///
    /// The newly-created filesystem registry is not empty. Instead, it contains all supported filesystems in the server
    /// registered. This means, the registry is basically read-only, because we have no plans to support plugins.
    pub fn new() -> Self {
        let mut this = Self(FxHashMap::default());
        this.0
            .insert("proc", Box::new(crate::filesystem::procfs::MakeProcfs));
        this.0
            .insert("tmpfs", Box::new(crate::filesystem::tmpfs::MakeTmpfs));
        this.0.insert(
            "nativefs",
            Box::new(crate::filesystem::nativefs::MakeNativefs),
        );
        this
    }

    /// Attempts to mount a filesystem, searching filesystems from the registry.
    pub fn mount(
        &self,
        fs: &str,
        dev: &[u8],
        flags: MountFlags,
        data: &[u8],
    ) -> Result<Arc<dyn Filesystem>, LxError> {
        self.0
            .get(fs)
            .ok_or(LxError::ENODEV)?
            .make_filesystem(dev, flags, data)
    }

    /// Lists all filesystems in the registry, as is represented in `/proc/filesystems`.
    pub fn list(&self) -> String {
        let mut s = String::with_capacity(512);
        for (&k, v) in self.0.iter() {
            let prefix = match v.is_nodev() {
                true => "nodev ",
                false => "      ",
            };
            writeln!(&mut s, "{prefix}{k}").unwrap();
        }
        s
    }
}

/// A mount namespace.
pub struct MountNamespace {
    mounts: RwLock<Vec<Mount>>,
}
impl MountNamespace {
    /// Creates a new, empty mount namespace.
    pub fn new() -> Self {
        Self {
            mounts: RwLock::new(Vec::with_capacity(16)),
        }
    }

    /// Mounts a new filesystem in the mount namespace.
    pub fn mount(
        &self,
        source: &[u8],
        target: &VPath,
        fs: &str,
        flags: MountFlags,
        data: &[u8],
    ) -> Result<(), LxError> {
        let target = target.clearize()?;
        let is_root = target.parts.is_empty();
        let path_exists = crate::util::test_path(
            self,
            &target,
            OpenHow {
                flags: OpenFlags::O_PATH.bits() as _,
                mode: 0,
                resolve: OpenResolve::RESOLVE_NO_SYMLINKS,
            },
        );
        if !path_exists && !is_root {
            return Err(LxError::ENOENT);
        }

        let filesystem = app().filesystems.mount(fs, source, flags, data)?;

        let mut mountpoint = target.clone();
        mountpoint.slash_suffix = false;

        let mount = Mount {
            source: source.to_vec(),
            mountpoint,
            filesystem,
            flags,
        };
        self.mounts.write().unwrap().push(mount);

        Ok(())
    }

    /// Unmounts a filesystem.
    pub fn umount(&self, path: &VPath, _flags: UmountFlags) -> Result<(), LxError> {
        let submount_busy = |p: &VPath, m: &Mount| {
            (p.parts.len() > m.mountpoint.parts.len())
                && (p.parts[..m.mountpoint.parts.len()] == m.mountpoint.parts)
        };

        let path = path.clearize()?;
        let mut nelem = None;

        let mut mounts = self.mounts.write().unwrap();

        for (n, mount) in mounts.iter().rev().enumerate() {
            if submount_busy(&path, mount) {
                return Err(LxError::EBUSY);
            }
            if mount.mountpoint.parts == path.parts {
                nelem = Some(n);
                break;
            }
        }

        match nelem {
            Some(i) => {
                if Arc::strong_count(&mounts[i].filesystem) > 1 {
                    return Err(LxError::EBUSY);
                }
                mounts.remove(i);
                Ok(())
            }
            None => Err(LxError::EINVAL),
        }
    }

    /// Locates a file in the VFS tree.
    pub fn locate(&self, full_path: &VPath) -> Result<Location, LxError> {
        let full_path = full_path.clearize()?;
        let mounts = self.mounts.read().unwrap();
        for mount in mounts.iter().rev() {
            if full_path.parts.len() < mount.mountpoint.parts.len() {
                continue;
            }

            if full_path.parts[..mount.mountpoint.parts.len()] == mount.mountpoint.parts {
                let parts = full_path.parts[mount.mountpoint.parts.len()..].to_vec();
                let relative = VPath {
                    slash_prefix: true,
                    slash_suffix: full_path.slash_suffix && !parts.is_empty(),
                    parts,
                };
                let lpath = LPath {
                    mountpoint: mount.mountpoint.clone(),
                    relative,
                };
                return Ok(Location {
                    filesystem: mount.filesystem.clone(),
                    path: lpath,
                    mount_flags: mount.flags,
                });
            }
        }
        Err(LxError::ENOENT)
    }

    /// Lists all mounts in the VFS tree.
    pub fn mounts(&self) -> Vec<Mount> {
        self.mounts.read().unwrap().clone()
    }
}

/// A mounted filesystem.
#[derive(Clone)]
pub struct Mount {
    pub source: Vec<u8>,
    pub mountpoint: VPath,
    pub filesystem: Arc<dyn Filesystem>,
    pub flags: MountFlags,
}

/// A path containing both the located mountpoint [`VPath`] and the relative [`VPath`].
///
/// This structure, instead of [`VPath`] directly, is used in filesystem operations to solve symbolic links.
#[derive(Debug, Clone)]
pub struct LPath {
    pub mountpoint: VPath,
    pub relative: VPath,
}
impl LPath {
    /// Expands the located path to a full [`VPath`] including the mountpoint and the relative path.
    pub fn expand(mut self) -> VPath {
        self.mountpoint.slash_suffix = self.relative.slash_suffix;
        self.mountpoint.parts.append(&mut self.relative.parts);
        self.mountpoint
    }
}

/// Location of a file in the VFS tree.
pub struct Location {
    filesystem: Arc<dyn Filesystem>,
    path: LPath,
    mount_flags: MountFlags,
}
impl Location {
    pub fn open(self, how: OpenHow) -> Result<NewlyOpen, LxError> {
        if how.flags().is_writable() {
            self.will_write()?;
        }

        self.filesystem.open(self.path.clone(), how).inspect(|x| {
            if let NewlyOpen::Virtual(vfd) = x {
                // We allow the filesystem driver to set the original path ahead of this.
                //
                // And for symlinks, we just make the `orig_path` be the finally solved one, matching the Linux behavior
                // better.
                _ = vfd.set_orig_path(self.path.expand().express());
            }
        })
    }

    pub fn access(self, mode: AccessFlags) -> Result<(), LxError> {
        if mode.contains(AccessFlags::W_OK) {
            self.will_write()?;
        }

        self.filesystem.access(self.path, mode)
    }

    pub fn unlink(self) -> Result<(), LxError> {
        self.will_write()?;
        self.filesystem.unlink(self.path)
    }

    pub fn rmdir(self) -> Result<(), LxError> {
        self.will_write()?;
        self.filesystem.rmdir(self.path)
    }

    pub fn symlink(self, content: &[u8]) -> Result<(), LxError> {
        self.will_write()?;
        self.filesystem.symlink(self.path, content)
    }

    pub fn mkdir(self, mode: FileMode) -> Result<(), LxError> {
        self.will_write()?;
        self.filesystem.mkdir(self.path, mode)
    }

    pub fn mknod(self, mode: FileMode, dev: DeviceNumber) -> Result<(), LxError> {
        self.will_write()?;
        self.filesystem.mknod(self.path, mode, dev)
    }

    pub fn get_sock_path(self, create: bool) -> Result<PathBuf, LxError> {
        if create {
            self.will_write()?;
        }
        self.filesystem.get_sock_path(self.path, create)
    }

    pub fn rename_to(self, new: Self) -> Result<(), LxError> {
        self.will_write()?;
        if !Arc::ptr_eq(&self.filesystem, &new.filesystem) {
            return Err(LxError::EXDEV);
        }
        self.filesystem.rename(new.path, self.path)
    }

    pub fn link_to(self, new: Self) -> Result<(), LxError> {
        self.will_write()?;
        if !Arc::ptr_eq(&self.filesystem, &new.filesystem) {
            return Err(LxError::EXDEV);
        }
        self.filesystem.link(new.path, self.path)
    }

    fn will_write(&self) -> Result<(), LxError> {
        if self.mount_flags.contains(MountFlags::MS_RDONLY) {
            Err(LxError::EROFS)
        } else {
            Ok(())
        }
    }
}

/// Content of a filesystem.
pub trait Filesystem: Send + Sync {
    fn open(self: Arc<Self>, path: LPath, how: OpenHow) -> Result<NewlyOpen, LxError>;
    fn access(&self, path: LPath, mode: AccessFlags) -> Result<(), LxError>;
    fn unlink(&self, path: LPath) -> Result<(), LxError>;
    fn rmdir(&self, path: LPath) -> Result<(), LxError>;
    fn symlink(&self, dst: LPath, content: &[u8]) -> Result<(), LxError>;
    fn mkdir(&self, path: LPath, mode: FileMode) -> Result<(), LxError>;
    fn mknod(&self, path: LPath, mode: FileMode, dev: DeviceNumber) -> Result<(), LxError>;
    fn get_sock_path(&self, path: LPath, create: bool) -> Result<PathBuf, LxError>;
    fn rename(&self, src: LPath, dst: LPath) -> Result<(), LxError>;
    fn link(&self, src: LPath, dst: LPath) -> Result<(), LxError>;

    fn statfs(&self) -> Result<StatFs, LxError>;
}

/// A factory of (mounted) filesystems.
pub trait MakeFilesystem: Send + Sync {
    /// Creates a mounted filesystem.
    fn make_filesystem(
        &self,
        dev: &[u8],
        flags: MountFlags,
        data: &[u8],
    ) -> Result<Arc<dyn Filesystem>, LxError>;

    fn is_nodev(&self) -> bool {
        false
    }
}

/// A newly-open file.
pub enum NewlyOpen {
    Native(Vec<u8>),
    Virtual(Vfd),
}
