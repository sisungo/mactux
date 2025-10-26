use crate::{
    app,
    filesystem::vfs::MountNamespace,
    sysinfo::UtsNamespace,
    task::{PidNamespace, thread::Thread},
    util::Shared,
    vfd::VfdTable,
};
use std::cell::RefCell;
use structures::error::LxError;

thread_local! {
    static SAVED_PROCESS: RefCell<Option<Process>> = RefCell::new(None);
}

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

pub fn before_fork() {
    SAVED_PROCESS.set(Some(Process::current()._child()));
}

pub fn after_fork(native_pid: libc::pid_t) -> Result<(), LxError> {
    let process = app()
        .processes
        .intervene(native_pid as _, SAVED_PROCESS.take().unwrap());
    let thread = Thread::builder().process(process).is_main().build()?;
    Thread::set_current(thread);
    Ok(())
}
