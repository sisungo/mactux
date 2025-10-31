use super::methods::*;
use crate::{ipc::RegChannel, task::process::Process};
use anyhow::anyhow;
use mactux_ipc::request::Request;
use std::os::unix::net::UnixStream;

#[derive(Debug)]
pub struct RegSession(RegChannel);
impl RegSession {
    pub fn new(stream: UnixStream) -> anyhow::Result<Self> {
        Ok(Self(RegChannel::new(stream)?))
    }

    pub fn start(self) -> anyhow::Result<()> {
        let apple_pid = self
            .0
            .peer_pid()
            .ok_or_else(|| anyhow!("failed to get credentials"))?;
        let parent = Process::current();

        std::thread::Builder::new()
            .name(format!("LxThread:{apple_pid}"))
            .spawn(move || {
                crate::task::configure()
                    .parent(parent)
                    .apple_pid(apple_pid)
                    .exec()
                    .unwrap(); // TODO: error report
                _ = self.run();
            })?;

        Ok(())
    }

    pub fn run(self) -> anyhow::Result<()> {
        let mut buf = Vec::with_capacity(512);

        while let Ok(req) = self.0.recv::<Request>(&mut buf) {
            let resp = match req {
                Request::Open(path, how) => open(path, how).into_response(),
                Request::Access(path, flags) => access(path, flags).into_response(),
                Request::Unlink(path) => unlink(path).into_response(),
                Request::Rmdir(path) => rmdir(path).into_response(),
                Request::Mkdir(path, mode) => mkdir(path, mode).into_response(),
                Request::Mknod(path, mode, dev) => mknod(path, mode, dev).into_response(),
                Request::Symlink(src, dst) => symlink(&src, &dst).into_response(),
                Request::Link(src, dst) => link(&src, &dst).into_response(),
                Request::Rename(src, dst) => rename(&src, &dst).into_response(),
                Request::GetSockPath(path, create) => get_sock_path(path, create).into_response(),
                Request::VfdDup(vfd) => vfd_dup(vfd).into_response(),
                Request::VfdStat(vfd) => vfd_stat(vfd, 0xfffffff).into_response(),
                Request::VfdRead(vfd, bufsiz) => vfd_read(vfd, bufsiz).into_response(),
                Request::VfdPread(vfd, off, bufsiz) => vfd_pread(vfd, bufsiz, off).into_response(),
                Request::VfdWrite(vfd, buf) => vfd_write(vfd, &buf).into_response(),
                Request::VfdPwrite(vfd, off, buf) => vfd_pwrite(vfd, &buf, off).into_response(),
                Request::VfdSeek(vfd, whence, off) => vfd_lseek(vfd, whence, off).into_response(),
                Request::VfdGetdent(vfd) => vfd_getdent(vfd).into_response(),
                Request::VfdReadlink(vfd) => vfd_readlink(vfd).into_response(),
                Request::VfdClose(vfd) => vfd_close(vfd).into_response(),
                Request::VfdOrigPath(vfd) => vfd_orig_path(vfd).into_response(),
                Request::VfdIoctlQuery(vfd, cmd) => vfd_ioctl_query(vfd, cmd).into_response(),
                Request::VfdIoctl(vfd, cmd, data) => vfd_ioctl(vfd, cmd, &data).into_response(),
                Request::VfdFcntl(vfd, cmd, data) => vfd_fcntl(vfd, cmd, &data).into_response(),
                Request::GetNetworkNames => get_network_names().into_response(),
                Request::SetNetworkNames(set) => set_network_names(set).into_response(),
                Request::SysInfo => sysinfo().into_response(),
                Request::AfterFork(npid) => after_fork(npid).into_response(),
                Request::AfterExec => after_exec().into_response(),
                Request::WriteSyslog(level, content) => {
                    write_syslog(level, content).into_response()
                }
                Request::SetThreadName(name) => set_thread_name(name).into_response(),
                other => todo!("{other:?}"),
            };
            self.0.send(&resp)?;
        }

        Ok(())
    }
}
