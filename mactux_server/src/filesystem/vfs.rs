use crate::{filesystem::VPath, vfd::Vfd};
use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};
use structures::{
    device::DeviceNumber,
    error::LxError,
    fs::{AccessFlags, FileMode, OpenFlags, OpenHow, OpenResolve},
};

/// A mount namespace.
pub struct MountNamespace {
    mounts: RwLock<Vec<Mount>>,
}
impl MountNamespace {
    pub fn new() -> Self {
        Self {
            mounts: RwLock::new(Vec::with_capacity(16)),
        }
    }

    pub fn mount(
        &self,
        source: &[u8],
        target: &VPath,
        fs: &str,
        flags: u64,
        data: u8,
    ) -> Result<(), LxError> {
        let is_root = target.parts.is_empty();
        let path_exists = crate::util::test_path(
            self,
            target,
            OpenHow {
                flags: OpenFlags::O_PATH.bits() as _,
                mode: 0,
                resolve: OpenResolve::RESOLVE_NO_SYMLINKS,
            },
        );
        if !path_exists && !is_root {
            return Err(LxError::ENOENT);
        }

        let filesystem = crate::filesystem::mount(fs, source, flags, data)?;

        let mut mountpoint = target.clone();
        mountpoint.slash_suffix = false;

        let mount = Mount {
            mountpoint,
            filesystem,
        };
        self.mounts.write().unwrap().push(mount);

        Ok(())
    }

    pub fn umount(&self, path: &VPath) -> Result<(), LxError> {
        let mut lock = self.mounts.write().unwrap();
        let index = lock
            .iter()
            .enumerate()
            .rev()
            .find(|(_, mount)| mount.mountpoint.parts == path.parts)
            .ok_or(LxError::EINVAL)?
            .0;
        lock.remove(index);
        Ok(())
    }

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
                });
            }
        }
        Err(LxError::ENOENT)
    }
}

/// A mounted filesystem.
struct Mount {
    mountpoint: VPath,
    filesystem: Arc<dyn Filesystem>,
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
    pub fn expand(mut self) -> VPath {
        self.mountpoint.slash_suffix = self.relative.slash_suffix;
        self.mountpoint.parts.append(&mut self.relative.parts);
        self.mountpoint
    }
}

pub struct Location {
    filesystem: Arc<dyn Filesystem>,
    path: LPath,
}
impl Location {
    pub fn open(self, how: OpenHow) -> Result<NewlyOpen, LxError> {
        self.filesystem.open(self.path.clone(), how).inspect(|x| {
            if let NewlyOpen::Virtual(vfd) = x {
                // We allow the filesystem driver to set the original path ahead of this.
                _ = vfd.set_orig_path(self.path.expand().express());
            }
        })
    }

    pub fn access(self, mode: AccessFlags) -> Result<(), LxError> {
        self.filesystem.access(self.path, mode)
    }

    pub fn unlink(self) -> Result<(), LxError> {
        self.filesystem.unlink(self.path)
    }

    pub fn rmdir(self) -> Result<(), LxError> {
        self.filesystem.rmdir(self.path)
    }

    pub fn symlink(self, content: &[u8]) -> Result<(), LxError> {
        self.filesystem.symlink(self.path, content)
    }

    pub fn mkdir(self, mode: FileMode) -> Result<(), LxError> {
        self.filesystem.mkdir(self.path, mode)
    }

    pub fn mknod(self, mode: FileMode, dev: DeviceNumber) -> Result<(), LxError> {
        self.filesystem.mknod(self.path, mode, dev)
    }

    pub fn get_sock_path(self, create: bool) -> Result<PathBuf, LxError> {
        self.filesystem.get_sock_path(self.path, create)
    }

    pub fn rename_to(self, new: Self) -> Result<(), LxError> {
        if !Arc::ptr_eq(&self.filesystem, &new.filesystem) {
            return Err(LxError::EXDEV);
        }
        self.filesystem.rename(new.path, self.path)
    }

    pub fn link_to(self, new: Self) -> Result<(), LxError> {
        if !Arc::ptr_eq(&self.filesystem, &new.filesystem) {
            return Err(LxError::EXDEV);
        }
        self.filesystem.link(new.path, self.path)
    }
}

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
}

pub enum NewlyOpen {
    Native(Vec<u8>),
    Virtual(Vfd),
}
