use super::methods::*;
use crate::{ipc::RegChannel, task::process::Process};
use anyhow::anyhow;
use mactux_ipc::request::Request;
use std::os::unix::net::UnixStream;
use structures::fs::{AccessFlags, OpenFlags};

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
        let mut buf = Vec::new();

        while let Ok(val) = self.0.recv::<Request>(&mut buf) {
            let resp = match val {
                Request::Open(path, flags, mode) => {
                    open(path, OpenFlags::from_bits_retain(flags), mode).into_response()
                }
                Request::Access(path, flags) => access(path, flags).into_response(),
                Request::Unlink(path) => unlink(path).into_response(),
                Request::Rmdir(path) => rmdir(path).into_response(),
                Request::Mkdir(path, mode) => mkdir(path, mode).into_response(),
                Request::Symlink(src, dst) => symlink(&src, &dst).into_response(),
                Request::GetSockPath(path, create) => get_sock_path(path, create).into_response(),
                Request::VfdStat(vfd) => vfd_stat(vfd, 0xfffffff).into_response(),
                Request::VfdRead(vfd, bufsiz) => vfd_read(vfd, bufsiz).into_response(),
                Request::VfdWrite(vfd, buf) => vfd_write(vfd, &buf).into_response(),
                Request::VfdSeek(vfd, whence, off) => vfd_lseek(vfd, whence, off).into_response(),
                Request::VfdGetdent(vfd) => vfd_getdent(vfd).into_response(),
                Request::VfdClose(vfd) => vfd_close(vfd).into_response(),
                Request::GetNetworkNames => get_network_names().into_response(),
                Request::BeforeFork => before_fork().into_response(),
                Request::AfterFork(npid) => after_fork(npid).into_response(),
                Request::AfterExec => after_exec().into_response(),
                other => todo!("{other:?}"),
            };
            self.0.send(&resp)?;
        }

        Ok(())
    }
}
