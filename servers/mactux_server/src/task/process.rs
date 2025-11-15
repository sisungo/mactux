use crate::{
    filesystem::vfs::MountNamespace,
    network::NetNamespace,
    sysinfo::UtsNamespace,
    task::{PidNamespace, thread::Thread},
    util::Shared,
    vfd::VfdTable,
};
use structures::error::LxError;

pub struct Process {
    pub mnt: Shared<MountNamespace>,
    pub uts: Shared<Box<dyn UtsNamespace>>,
    pub pid: Shared<Box<dyn PidNamespace>>,
    pub net: Shared<NetNamespace>,
    pub vfd: VfdTable,
}
impl Process {
    pub fn server() -> Shared<Self> {
        Thread::server().process()
    }

    pub fn current() -> Shared<Self> {
        Thread::current().process()
    }

    pub(super) fn _child(&self) -> Self {
        Self {
            mnt: self.mnt.clone(),
            uts: self.uts.clone(),
            pid: self.pid.clone(),
            net: self.net.clone(),
            vfd: self.vfd.fork(),
        }
    }

    pub fn on_exec(&self) {
        self.vfd.on_exec();
    }
}

pub fn after_fork(apple_pid: libc::pid_t) -> Result<(), LxError> {
    crate::task::configure()
        .parent(Process::current())
        .apple_pid(apple_pid)
        .exec()
}
