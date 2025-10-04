use super::Session;
use crate::filesystem::vfs::{NewlyOpen, VfsPath};
use mactux_ipc::response::{NetworkNames, Response};
use structures::{
    error::LxError,
    fs::{AccessFlags, OpenFlags, UmountFlags},
    io::{EventFdFlags, FcntlCmd, IoctlCmd, Whence},
};

impl Session {
    pub fn set_mount_namespace(&self, id: u64) -> Response {
        match self.process.set_mnt_ns(id) {
            Ok(()) => Response::Nothing,
            Err(err) => Response::Error(err),
        }
    }

    pub fn umount(&self, path: Vec<u8>, flags: u32) -> Response {
        match self.process.mnt_ns().umount(
            &VfsPath::from_bytes(&path),
            UmountFlags::from_bits_retain(flags),
        ) {
            Ok(()) => Response::Nothing,
            Err(err) => Response::Error(err),
        }
    }

    pub async fn open(&self, path: Vec<u8>, flags: u32, mode: u32) -> Response {
        match self
            .process
            .mnt_ns()
            .open(
                &VfsPath::from_bytes(&path),
                OpenFlags::from_bits_retain(flags),
                mode,
            )
            .await
        {
            Ok(NewlyOpen::AtNative(path)) => {
                Response::OpenNativePath(path.into_os_string().into_encoded_bytes())
            }
            Ok(NewlyOpen::Virtual(vfd)) => Response::OpenVirtualFd(self.process.vfd_register(vfd)),
            Err(err) => Response::Error(err),
        }
    }

    pub async fn access(&self, path: Vec<u8>, mode: u32) -> Response {
        match self
            .process
            .mnt_ns()
            .access(
                &VfsPath::from_bytes(&path),
                AccessFlags::from_bits_retain(mode),
            )
            .await
        {
            Ok(()) => Response::Nothing,
            Err(err) => Response::Error(err),
        }
    }

    pub async fn symlink(&self, src: Vec<u8>, dst: Vec<u8>) -> Response {
        match self
            .process
            .mnt_ns()
            .symlink(&VfsPath::from_bytes(&dst), &src)
            .await
        {
            Ok(()) => Response::Nothing,
            Err(err) => Response::Error(err),
        }
    }

    pub async fn rename(&self, src: Vec<u8>, dst: Vec<u8>) -> Response {
        match self
            .process
            .mnt_ns()
            .rename(&VfsPath::from_bytes(&src), &VfsPath::from_bytes(&dst))
            .await
        {
            Ok(()) => Response::Nothing,
            Err(err) => Response::Error(err),
        }
    }

    pub async fn unlink(&self, path: Vec<u8>) -> Response {
        match self
            .process
            .mnt_ns()
            .unlink(&VfsPath::from_bytes(&path))
            .await
        {
            Ok(()) => Response::Nothing,
            Err(err) => Response::Error(err),
        }
    }

    pub async fn rmdir(&self, path: Vec<u8>) -> Response {
        match self
            .process
            .mnt_ns()
            .rmdir(&VfsPath::from_bytes(&path))
            .await
        {
            Ok(()) => Response::Nothing,
            Err(err) => Response::Error(err),
        }
    }

    pub async fn get_sock_path(&self, path: Vec<u8>, create: bool) -> Response {
        match self
            .process
            .mnt_ns()
            .get_sock_path(&VfsPath::from_bytes(&path), create)
            .await
        {
            Ok(path) => Response::SockPath(path),
            Err(err) => Response::Error(err),
        }
    }

    pub async fn mkdir(&self, path: Vec<u8>, mode: u32) -> Response {
        match self
            .process
            .mnt_ns()
            .mkdir(&VfsPath::from_bytes(&path), mode)
            .await
        {
            Ok(()) => Response::Nothing,
            Err(err) => Response::Error(err),
        }
    }

    pub async fn vfd_read(&self, vfd: u64, count: usize) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        let mut buf = vec![0; count];
        match vfd.read(&mut buf).await {
            Ok(count) => {
                buf.resize(count, 0);
                Response::Read(buf)
            }
            Err(err) => Response::Error(err),
        }
    }

    pub async fn vfd_write(&self, vfd: u64, buf: &[u8]) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.write(buf).await {
            Ok(count) => Response::Write(count),
            Err(err) => Response::Error(err),
        }
    }

    pub async fn vfd_lseek(&self, vfd: u64, whence: Whence, off: i64) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.lseek(whence, off).await {
            Ok(count) => Response::Lseek(count),
            Err(err) => Response::Error(err),
        }
    }

    pub async fn vfd_stat(&self, vfd: u64) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.stat().await {
            Ok(stat) => Response::Stat(stat),
            Err(err) => Response::Error(err),
        }
    }

    pub async fn vfd_readlink(&self, vfd: u64) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.readlink().await {
            Ok(path) => Response::Readlink(path),
            Err(err) => Response::Error(err),
        }
    }

    pub async fn vfd_sync(&self, vfd: u64) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.sync().await {
            Ok(()) => Response::Nothing,
            Err(err) => Response::Error(err),
        }
    }

    pub async fn vfd_fcntl(&self, vfd: u64, cmd: u32, data: &[u8]) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.fcntl(FcntlCmd(cmd), data).await {
            Ok(resp) => resp,
            Err(err) => Response::Error(err),
        }
    }

    pub fn vfd_ioctl_query(&self, vfd: u64, cmd: u32) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.ioctl_query(IoctlCmd(cmd)) {
            Ok(ctrl) => Response::VirtualFdAvailCtrl(ctrl),
            Err(err) => Response::Error(err),
        }
    }

    pub async fn vfd_ioctl(&self, vfd: u64, cmd: u32, data: &[u8]) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.ioctl(IoctlCmd(cmd), data).await {
            Ok(resp) => resp,
            Err(err) => Response::Error(err),
        }
    }

    pub async fn vfd_truncate(&self, vfd: u64, len: u64) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.truncate(len).await {
            Ok(()) => Response::Nothing,
            Err(err) => Response::Error(err),
        }
    }

    pub async fn vfd_chown(&self, vfd: u64, uid: u32, gid: u32) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.chown(uid, gid).await {
            Ok(()) => Response::Nothing,
            Err(err) => Response::Error(err),
        }
    }

    pub async fn vfd_getdents64(&self, vfd: u64) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.getdents64().await {
            Ok(Some(x)) => Response::Dirent64(x),
            Ok(None) => Response::Nothing,
            Err(err) => Response::Error(err),
        }
    }

    pub fn vfd_close(&self, vfd: u64) -> Response {
        self.process.vfd_close(vfd);
        Response::Nothing
    }

    pub fn vfd_orig_path(&self, vfd: u64) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.orig_path() {
            Ok(path) => Response::OrigPath(path.to_bytes()),
            Err(err) => Response::Error(err),
        }
    }

    pub async fn vfd_dup(&self, vfd: u64) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        let new_vfd = vfd.dup().await;
        let new_id = self.process.vfd_register(new_vfd);
        Response::DupVirtualFd(new_id)
    }

    pub fn eventfd(&self, initval: u64, flags: u32) -> Response {
        match crate::vfd::eventfd::create(initval, EventFdFlags::from_bits_truncate(flags)) {
            Ok(vfd) => Response::EventFd(self.process.vfd_register(vfd)),
            Err(err) => Response::Error(err),
        }
    }

    pub fn set_network_names(&self, nodename: Vec<u8>, domainname: Vec<u8>) -> Response {
        if let Err(err) = self.process.uts_ns().set_domainname(&domainname) {
            return Response::Error(err);
        }
        if let Err(err) = self.process.uts_ns().set_nodename(&nodename) {
            return Response::Error(err);
        }
        Response::Nothing
    }

    pub fn get_network_names(&self) -> Response {
        Response::NetworkNames(NetworkNames {
            nodename: self.process.uts_ns().nodename(),
            domainname: self.process.uts_ns().domainname(),
        })
    }

    pub fn sysinfo(&self) -> Response {
        match crate::sysinfo::sysinfo() {
            Ok(sysinfo) => Response::SysInfo(sysinfo),
            Err(err) => Response::Error(err),
        }
    }

    pub fn write_syslog(&self, data: &[u8]) -> Response {
        tracing::info!("{}", String::from_utf8_lossy(data));
        Response::Nothing
    }

    pub async fn before_fork(&mut self) -> Response {
        self.process = self.process.fork().await;
        Response::Nothing
    }

    pub fn after_fork(&self, pid: i32) -> Response {
        self.process.clone().set_native_pid(pid);
        Response::Nothing
    }

    pub fn after_exec(&self) -> Response {
        self.process.after_exec();
        Response::Nothing
    }
}
