//! The `nativefs` filesystem.
//!
//! This is a filesystem that maps all Linux filesystem accesses to macOS ones.

use crate::{
    filesystem::vfs::{MountDev, Mountable, NewlyOpen, VfsPath},
    util::c_str,
    vfd::{VirtualFd, VirtualFile},
};
use async_trait::async_trait;
use mactux_ipc::response::Response;
use std::{path::PathBuf, sync::Arc};
use structures::{
    error::LxError,
    fs::{AccessFlags, Dirent64, Dirent64Hdr, DirentType, OpenFlags, Statx},
    io::FcntlCmd, ToApple,
};
use tokio::{fs::ReadDir, sync::Mutex};

#[derive(Debug)]
pub struct NativeFs {
    native_path: PathBuf,
}
impl NativeFs {
    pub fn new(native_path: PathBuf) -> std::io::Result<Self> {
        Ok(Self {
            native_path: std::fs::canonicalize(native_path)?,
        })
    }

    fn interpret_vpath(&self, vpath: &VfsPath) -> PathBuf {
        require_secure_vfs_path(vpath);
        self.native_path.join(&vpath.to_string()[1..])
    }
}
#[async_trait]
impl Mountable for NativeFs {
    async fn open(
        self: Arc<Self>,
        path: &VfsPath,
        flags: OpenFlags,
        _mode: u32,
    ) -> Result<NewlyOpen, LxError> {
        let path = self.interpret_vpath(path);
        if path.is_dir() {
            Ok(NewlyOpen::Virtual(VirtualFd::new(
                Box::new(NativeDirVirtualFd {
                    path: path.clone(),
                    read_dir: Mutex::new(tokio::fs::read_dir(&path).await?),
                }),
                flags,
            )))
        } else {
            Ok(NewlyOpen::AtNative(path))
        }
    }

    async fn access(&self, path: &VfsPath, mode: AccessFlags) -> Result<(), LxError> {
        let path = self.interpret_vpath(path);
        let path = c_str(path.into_os_string().into_encoded_bytes());
        unsafe {
            match libc::access(path.as_ptr().cast(), mode.to_apple()?) {
                -1 => Err(LxError::last_apple_error()),
                _ => Ok(()),
            }
        }
    }

    async fn unlink(&self, path: &VfsPath) -> Result<(), LxError> {
        let path = self.interpret_vpath(path);
        tokio::fs::remove_file(path).await.map_err(LxError::from)
    }

    async fn rmdir(&self, path: &VfsPath) -> Result<(), LxError> {
        let path = self.interpret_vpath(path);
        tokio::fs::remove_dir(path).await.map_err(LxError::from)
    }

    async fn symlink(&self, dst: &VfsPath, content: &[u8]) -> Result<(), LxError> {
        let dst = self.interpret_vpath(dst);
        let content = PathBuf::from(&*String::from_utf8_lossy(content)); // TODO
        tokio::fs::symlink(content, dst)
            .await
            .map_err(LxError::from)
    }

    async fn mkdir(&self, path: &VfsPath, mode: u32) -> Result<(), LxError> {
        let path = self.interpret_vpath(path);
        let path = c_str(path.into_os_string().into_encoded_bytes());
        unsafe {
            // TODO mode
            match libc::mkdir(path.as_ptr().cast(), mode as _) {
                -1 => Err(LxError::last_apple_error()),
                _ => Ok(()),
            }
        }
    }

    async fn get_sock_path(&self, path: &VfsPath, _create: bool) -> Result<PathBuf, LxError> {
        Ok(self.interpret_vpath(path))
    }

    async fn rename(&self, src: &VfsPath, dst: &VfsPath) -> Result<(), LxError> {
        let src = self.interpret_vpath(src);
        let dst = self.interpret_vpath(dst);
        tokio::fs::rename(src, dst).await.map_err(From::from)
    }

    async fn mount_bind(&self, path: &VfsPath) -> Result<Box<dyn Mountable>, LxError> {
        todo!()
    }
}

pub struct NativeDirVirtualFd {
    path: PathBuf,
    read_dir: Mutex<ReadDir>,
}
#[async_trait]
impl VirtualFile for NativeDirVirtualFd {
    async fn getdents64(&self) -> Result<Option<Dirent64>, LxError> {
        match self.read_dir.lock().await.next_entry().await? {
            Some(entry) => Ok(Some(Dirent64::new(
                Dirent64Hdr {
                    d_ino: entry.ino(),
                    d_off: 0,
                    d_reclen: 0,
                    d_type: entry
                        .file_type()
                        .await
                        .map(DirentType::from_std)
                        .unwrap_or(DirentType::DT_UNKNOWN),
                    _align: [0; _],
                },
                entry.file_name().into_encoded_bytes(),
            ))),
            None => Ok(None),
        }
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

    async fn stat(&self) -> Result<Statx, LxError> {
        let c_path = c_str(self.path.clone().into_os_string().into_encoded_bytes());
        tokio::task::spawn_blocking(move || unsafe {
            let mut stat = std::mem::zeroed();
            match libc::stat(c_path.as_ptr().cast(), &mut stat) {
                -1 => Err(LxError::last_apple_error()),
                _ => Ok(Statx::from_apple(stat)),
            }
        })
        .await
        .unwrap()
    }
}

pub fn mountable(dev: MountDev, opts: &str) -> Result<Arc<dyn Mountable>, LxError> {
    let MountDev::Freeform(np) = dev else {
        return Err(LxError::EINVAL);
    };
    let Some(np) = np.strip_prefix("native=") else {
        return Err(LxError::EINVAL);
    };
    let native_path = PathBuf::from(np);
    if !native_path.is_absolute() {
        return Err(LxError::EINVAL);
    }
    if !native_path.exists() {
        return Err(LxError::ENOENT);
    }
    Ok(Arc::new(NativeFs::new(native_path)?))
}

fn require_secure_vfs_path(path: &VfsPath) {
    // The provided path must be "clearized", so it should not contain any ".." or ".".
    // However, occurrence of "." is secure, so for avoiding panics, we do not check ".".
    // It is expected to break performance, so it is only enabled in debug mode.
    #[cfg(debug_assertions)]
    {
        assert!(path.segments.iter().any(|x| &x[..] == &b".."[..]) == false);
    }
}
