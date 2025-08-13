use crate::{
    app,
    filesystem::vfs::{MountNamespace, Mountable},
    util::Registry,
    uts::{InitUts, UtsNamespace},
    vfd::VirtualFd,
};
use papaya::Guard;
use std::sync::{
    Arc, RwLock,
    atomic::{self, AtomicI32},
};
use structures::error::LxError;

pub trait PidNamespace: Send + Sync {
    /// Maps an apple native PID to a Linux PID in this PID namespace.
    fn apple_to_linux(&self, apple: libc::pid_t) -> Option<i32>;

    /// Maps a Linux PID in this namespace to an apple native PID.
    fn linux_to_apple(&self, linux: i32) -> Option<libc::pid_t>;

    /// Registers an apple native PID, returning the mapped Linux PID in this namespace.
    fn register(&self, apple: libc::pid_t) -> i32;

    /// Unregisters an apple native PID.
    fn unregister(&self, apple: libc::pid_t);

    /// Returns the parent PID namespace.
    fn parent(&self) -> Option<&Arc<dyn PidNamespace>>;

    /// Returns the associated procfs instance.
    fn procfs(&self) -> Option<Arc<dyn Mountable>>;
}

#[derive(Debug)]
pub struct InitPid;
impl PidNamespace for InitPid {
    fn apple_to_linux(&self, apple: libc::pid_t) -> Option<i32> {
        Some(apple)
    }

    fn linux_to_apple(&self, linux: i32) -> Option<libc::pid_t> {
        Some(linux)
    }

    fn register(&self, apple: libc::pid_t) -> i32 {
        apple
    }

    fn unregister(&self, apple: libc::pid_t) {}

    fn parent(&self) -> Option<&Arc<dyn PidNamespace>> {
        None
    }

    fn procfs(&self) -> Option<Arc<dyn Mountable>> {
        None
    }
}

pub struct ProcessCtx {
    native_pid: AtomicI32,

    mnt_ns: RwLock<Arc<MountNamespace>>,
    uts_ns: RwLock<Arc<dyn UtsNamespace>>,
    pid_ns: RwLock<Arc<dyn PidNamespace>>,

    vfd_table: Registry<Arc<VirtualFd>>,
}
impl ProcessCtx {
    pub fn scratch(pid: i32) -> Arc<Self> {
        Arc::new(Self {
            native_pid: pid.into(),
            mnt_ns: RwLock::new(MountNamespace::initial()),
            uts_ns: RwLock::new(Arc::new(InitUts)),
            pid_ns: RwLock::new(Arc::new(InitPid)),
            vfd_table: Registry::new(),
        })
    }

    pub async fn fork(&self) -> Arc<Self> {
        Arc::new(Self {
            native_pid: AtomicI32::new(0),
            mnt_ns: RwLock::new(self.mnt_ns()),
            uts_ns: RwLock::new(self.uts_ns()),
            pid_ns: RwLock::new(self.pid_ns()),
            vfd_table: crate::vfd::fork_table(&self.vfd_table).await,
        })
    }

    pub fn mnt_ns(&self) -> Arc<MountNamespace> {
        self.mnt_ns.read().unwrap().clone()
    }

    pub fn set_mnt_ns(&self, new: u64) -> Result<(), LxError> {
        let Some(new) = app().mnt_ns_registry.get(new) else {
            return Err(LxError::ENOENT);
        };
        *self.mnt_ns.write().unwrap() = new;
        Ok(())
    }

    pub fn uts_ns(&self) -> Arc<dyn UtsNamespace> {
        self.uts_ns.read().unwrap().clone()
    }

    pub fn set_uts_ns(&self, new: u64) -> Result<(), LxError> {
        todo!()
    }

    pub fn pid_ns(&self) -> Arc<dyn PidNamespace> {
        self.pid_ns.read().unwrap().clone()
    }

    pub fn set_pid_ns(&self, new: u64) -> Result<(), LxError> {
        todo!()
    }

    pub fn native_pid(&self) -> i32 {
        self.native_pid.load(atomic::Ordering::Relaxed)
    }

    pub fn set_native_pid(self: Arc<Self>, new: i32) {
        debug_assert_eq!(self.native_pid(), 0);
        self.native_pid.store(new, atomic::Ordering::Relaxed);
        app().native_procs.pin().insert(new, self);
    }

    pub fn vfd(&self, id: u64) -> Result<Arc<VirtualFd>, LxError> {
        self.vfd_table.get(id).ok_or(LxError::EBADF)
    }

    pub fn vfd_register(&self, object: Arc<VirtualFd>) -> u64 {
        self.vfd_table.register(object)
    }

    pub fn vfd_close(&self, id: u64) {
        _ = self.vfd_table.unregister(id, true);
    }

    pub fn after_exec(&self) {
        crate::vfd::exec_table(&self.vfd_table);
    }
}

pub fn ctx_by_pid(pid: i32) -> Arc<ProcessCtx> {
    let pinned = app().native_procs.pin();
    match pinned.get(&pid).cloned() {
        Some(ctx) => ctx,
        None => {
            let ctx = ProcessCtx::scratch(pid);
            pinned.insert(pid, ctx.clone());
            ctx
        }
    }
}

pub fn ctx_close(pid: i32) {
    let guard = app().native_procs.guard();
    if let Some(ctx) = app().native_procs.get(&pid, &guard) {
        if Arc::strong_count(ctx) <= 2 {
            app().native_procs.remove(&pid, &guard);
            guard.flush();
        }
    }
}
