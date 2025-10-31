use crate::{
    filesystem::vfs::MountNamespace,
    sysinfo::UtsNamespace,
    task::{PidNamespace, thread::Thread},
    util::Shared,
    vfd::VfdTable,
};
use std::sync::RwLock;
use structures::error::LxError;

pub struct Process {
    pub mnt: Shared<MountNamespace>,
    pub uts: Shared<Box<dyn UtsNamespace>>,
    pub vfd: VfdTable,
    pub pid: Shared<Box<dyn PidNamespace>>,
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
            vfd: self.vfd.fork(),
            pid: self.pid.clone(),
        }
    }

    pub fn exec(&self) {
        self.vfd.exec();
    }
}

pub fn after_fork(apple_pid: libc::pid_t) -> Result<(), LxError> {
    crate::task::configure()
        .parent(Process::current())
        .apple_pid(apple_pid)
        .exec()
}
