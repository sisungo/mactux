mod interruptible;
mod operations;

use crate::process::ProcessCtx;
use anyhow::anyhow;
use interruptible::InterruptibleSession;
use mactux_ipc::{
    handshake::{HandshakeRequest, HandshakeResponse},
    request::Request,
    response::Response,
};
use std::{path::Path, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
};

/// A server.
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

/// A session.
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
                Request::Umount(path, flags) => self.umount(path, flags),
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
                Request::VirtualFdReadlink(vfd) => self.vfd_readlink(vfd).await,
                Request::VirtualFdClose(vfd) => self.vfd_close(vfd),
                Request::VirtualFdSync(vfd) => self.vfd_sync(vfd).await,
                Request::VirtualFdOrigPath(vfd) => self.vfd_orig_path(vfd),
                Request::EventFd(initval, flags) => self.eventfd(initval, flags),
                Request::GetNetworkNames => self.get_network_names(),
                Request::SetNetworkNames(nodename, domainname) => {
                    self.set_network_names(nodename, domainname)
                }
                Request::SysInfo => self.sysinfo(),
                Request::WriteSyslog(data) => self.write_syslog(&data),
                Request::BeforeFork => self.before_fork().await,
                Request::AfterFork(pid) => self.after_fork(pid),
                Request::AfterExec => self.after_exec(),
                Request::CallInterruptible(ireq) => {
                    break InterruptibleSession::from_session(self).run(ireq).await;
                }
            };
            self.respond(&resp, &mut buf).await?;
        }
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
