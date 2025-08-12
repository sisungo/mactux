use crate::{
    process::ProcessCtx,
    filesystem::vfs::{NewlyOpen, VfsPath},
};
use anyhow::anyhow;
use mactux_ipc::{
    handshake::{HandshakeRequest, HandshakeResponse},
    request::Request,
    response::{NetworkNames, Response},
};
use std::{path::Path, sync::Arc};
use structures::{
    error::LxError,
    fs::{AccessFlags, OpenFlags},
    io::{FcntlCmd, IoctlCmd, Whence},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
};

pub struct Server {
    listener: UnixListener,
}
impl Server {
    pub fn bind(path: &Path) -> std::io::Result<Self> {
        Ok(Self {
            listener: UnixListener::bind(path)?,
        })
    }

    pub async fn run(self) -> std::io::Result<()> {
        loop {
            let Ok((conn, _)) = self.listener.accept().await else {
                continue;
            };
            let pid = conn
                .peer_cred()?
                .pid()
                .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidData))?;

            let session = Session {
                conn,
                process: crate::process::ctx_by_pid(pid as _),
            };
            tokio::spawn(session.run());
        }
    }
}

pub struct Session {
    conn: UnixStream,
    process: Arc<ProcessCtx>,
}
impl Session {
    pub async fn run(mut self) -> anyhow::Result<()> {
        let mut buf = Vec::with_capacity(64);
        self.handshake(&mut buf).await?;
        loop {
            let req = self.recv_request(&mut buf).await?;
            let resp = match req {
                Request::SetMountNamespace(id) => self.set_mount_namespace(id),
                Request::Open(path, flags, mode) => self.open(path, flags, mode).await,
                Request::Access(path, mode) => self.access(path, mode).await,
                Request::Symlink(src, dst) => self.symlink(src, dst).await,
                Request::Rename(src, dst) => self.rename(src, dst).await,
                Request::Unlink(path) => self.unlink(path).await,
                Request::Mkdir(path, mode) => self.mkdir(path, mode).await,
                Request::Rmdir(path) => self.rmdir(path).await,
                Request::GetSockPath(path, create) => self.get_sock_path(path, create).await,
                Request::VirtualFdRead(vfd, count) => self.vfd_read(vfd, count).await,
                Request::VirtualFdPread(vfd, off, count) => todo!(),
                Request::VirtualFdWrite(vfd, buf) => self.vfd_write(vfd, &buf).await,
                Request::VirtualFdPwrite(vfd, off, count) => todo!(),
                Request::VirtualFdLseek(vfd, whence, off) => self.vfd_lseek(vfd, whence, off).await,
                Request::VirtualFdFcntl(vfd, cmd, data) => self.vfd_fcntl(vfd, cmd, &data).await,
                Request::VirtualFdIoctlQuery(vfd, cmd) => self.vfd_ioctl_query(vfd, cmd),
                Request::VirtualFdIoctl(vfd, cmd, data) => self.vfd_ioctl(vfd, cmd, &data).await,
                Request::VirtualFdGetDents64(vfd) => self.vfd_getdents64(vfd).await,
                Request::VirtualFdStat(vfd) => self.vfd_stat(vfd).await,
                Request::VirtualFdTruncate(vfd, len) => self.vfd_truncate(vfd, len).await,
                Request::VirtualFdChown(vfd, uid, gid) => self.vfd_chown(vfd, uid, gid).await,
                Request::VirtualFdDup(vfd) => self.vfd_dup(vfd).await,
                Request::VirtualFdClose(vfd) => self.vfd_close(vfd),
                Request::VirtualFdOrigPath(vfd) => self.vfd_orig_path(vfd),
                Request::GetNetworkNames => self.get_network_names(),
                Request::SetNetworkNames(nodename, domainname) => {
                    self.set_network_names(nodename, domainname)
                }
                Request::SysInfo => self.sysinfo(),
                Request::WriteSyslog(data) => self.write_syslog(&data),
                Request::BeforeFork => self.before_fork().await,
                Request::AfterFork(pid) => self.after_fork(pid),
                Request::AfterExec => self.after_exec(),
            };
            self.respond(&resp, &mut buf).await?;
        }
    }

    fn set_mount_namespace(&self, id: u64) -> Response {
        match self.process.set_mnt_ns(id) {
            Ok(()) => Response::Nothing,
            Err(err) => Response::Error(err),
        }
    }

    async fn open(&self, path: Vec<u8>, flags: u32, mode: u32) -> Response {
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

    async fn access(&self, path: Vec<u8>, mode: u32) -> Response {
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

    async fn symlink(&self, src: Vec<u8>, dst: Vec<u8>) -> Response {
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

    async fn rename(&self, src: Vec<u8>, dst: Vec<u8>) -> Response {
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

    async fn unlink(&self, path: Vec<u8>) -> Response {
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

    async fn rmdir(&self, path: Vec<u8>) -> Response {
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

    async fn get_sock_path(&self, path: Vec<u8>, create: bool) -> Response {
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

    async fn mkdir(&self, path: Vec<u8>, mode: u32) -> Response {
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

    async fn vfd_read(&self, vfd: u64, count: usize) -> Response {
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

    async fn vfd_write(&self, vfd: u64, buf: &[u8]) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.write(buf).await {
            Ok(count) => Response::Write(count),
            Err(err) => Response::Error(err),
        }
    }

    async fn vfd_lseek(&self, vfd: u64, whence: Whence, off: i64) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.lseek(whence, off).await {
            Ok(count) => Response::Lseek(count),
            Err(err) => Response::Error(err),
        }
    }

    async fn vfd_stat(&self, vfd: u64) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.stat().await {
            Ok(stat) => Response::Stat(stat),
            Err(err) => Response::Error(err),
        }
    }

    async fn vfd_fcntl(&self, vfd: u64, cmd: u32, data: &[u8]) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.fcntl(FcntlCmd(cmd), data).await {
            Ok(resp) => resp,
            Err(err) => Response::Error(err),
        }
    }

    fn vfd_ioctl_query(&self, vfd: u64, cmd: u32) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.ioctl_query(IoctlCmd(cmd)) {
            Ok(ctrl) => Response::VirtualFdAvailCtrl(ctrl),
            Err(err) => Response::Error(err),
        }
    }

    async fn vfd_ioctl(&self, vfd: u64, cmd: u32, data: &[u8]) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.ioctl(IoctlCmd(cmd), data).await {
            Ok(resp) => resp,
            Err(err) => Response::Error(err),
        }
    }

    async fn vfd_truncate(&self, vfd: u64, len: u64) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.truncate(len).await {
            Ok(()) => Response::Nothing,
            Err(err) => Response::Error(err),
        }
    }

    async fn vfd_chown(&self, vfd: u64, uid: u32, gid: u32) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.chown(uid, gid).await {
            Ok(()) => Response::Nothing,
            Err(err) => Response::Error(err),
        }
    }

    async fn vfd_getdents64(&self, vfd: u64) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.getdents64().await {
            Ok(Some(x)) => Response::Dirent64(x),
            Ok(None) => Response::Nothing,
            Err(err) => Response::Error(err),
        }
    }

    fn vfd_close(&self, vfd: u64) -> Response {
        self.process.vfd_close(vfd);
        Response::Nothing
    }

    fn vfd_orig_path(&self, vfd: u64) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        match vfd.orig_path() {
            Ok(path) => Response::OrigPath(path.to_bytes()),
            Err(err) => Response::Error(err),
        }
    }

    async fn vfd_dup(&self, vfd: u64) -> Response {
        let Ok(vfd) = self.process.vfd(vfd) else {
            return Response::Error(LxError::EBADF);
        };
        let new_vfd = vfd.dup().await;
        let new_id = self.process.vfd_register(new_vfd);
        Response::DupVirtualFd(new_id)
    }

    fn set_network_names(&self, nodename: Vec<u8>, domainname: Vec<u8>) -> Response {
        if let Err(err) = self.process.uts_ns().set_domainname(&domainname) {
            return Response::Error(err);
        }
        if let Err(err) = self.process.uts_ns().set_nodename(&nodename) {
            return Response::Error(err);
        }
        Response::Nothing
    }

    fn get_network_names(&self) -> Response {
        Response::NetworkNames(NetworkNames {
            nodename: self.process.uts_ns().nodename(),
            domainname: self.process.uts_ns().domainname(),
        })
    }

    fn sysinfo(&self) -> Response {
        match crate::sysinfo::sysinfo() {
            Ok(sysinfo) => Response::SysInfo(sysinfo),
            Err(err) => Response::Error(err),
        }
    }

    fn write_syslog(&self, data: &[u8]) -> Response {
        tracing::info!("{}", String::from_utf8_lossy(data));
        Response::Nothing
    }

    async fn before_fork(&mut self) -> Response {
        self.process = self.process.fork().await;
        Response::Nothing
    }

    fn after_fork(&self, pid: i32) -> Response {
        self.process.clone().set_native_pid(pid);
        Response::Nothing
    }

    fn after_exec(&self) -> Response {
        self.process.after_exec();
        Response::Nothing
    }

    async fn handshake(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        self.recv_raw(buf).await?;
        let handshake_req: HandshakeRequest =
            bincode::decode_from_slice(buf, bincode::config::standard())?.0;
        if handshake_req != HandshakeRequest::new() {
            return Err(anyhow!("invalid handshake request"));
        }
        buf.clear();
        bincode::encode_into_std_write(&HandshakeResponse::new(), buf, bincode::config::standard())
            .expect("handshake responses should be always serializable");
        self.send_raw(buf).await?;
        Ok(())
    }

    async fn recv_request(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<Request> {
        self.recv_raw(buf).await?;
        Ok(bincode::decode_from_slice(&buf, bincode::config::standard())?.0)
    }

    async fn respond(&mut self, response: &Response, buf: &mut Vec<u8>) -> std::io::Result<()> {
        buf.clear();
        bincode::encode_into_std_write(response, buf, bincode::config::standard())
            .expect("all repsponses should be serializable");
        self.send_raw(buf).await?;
        Ok(())
    }

    async fn send_raw(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.conn.write_u64_le(buf.len() as _).await?;
        self.conn.write_all(buf).await?;
        Ok(())
    }

    async fn recv_raw(&mut self, buf: &mut Vec<u8>) -> std::io::Result<()> {
        buf.clear();
        let len = self.conn.read_u64_le().await?;
        buf.resize(len as _, 0);
        self.conn.read_exact(buf).await?;
        Ok(())
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        crate::process::ctx_close(self.process.native_pid());
    }
}
