//! Process context management.

use crate::{
    app,
    filesystem::{
        kernfs::KernFs,
        vfs::{MountNamespace, Mountable},
    },
    util::Registry,
    uts::{InitUts, UtsNamespace},
    vfd::VirtualFd,
};
use papaya::Guard;
use std::sync::{
    Arc, OnceLock, RwLock,
    atomic::{self, AtomicI32},
};
use structures::error::LxError;

/// A PID namespace.
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

/// The initial PID namespace, mapping directly to the host system.
pub struct InitPid {
    procfs: Arc<KernFs>,
}
impl InitPid {
    /// Returns the singleton instance of the initial PID namespace.
    pub fn instance() -> Arc<Self> {
        static INSTANCE: OnceLock<Arc<InitPid>> = OnceLock::new();

        INSTANCE
            .get_or_init(|| {
                let procfs = crate::filesystem::procfs::empty();
                crate::filesystem::procfs::add_process(&procfs, std::process::id() as _, 1);
                Arc::new(Self { procfs })
            })
            .clone()
    }
}
impl PidNamespace for InitPid {
    fn apple_to_linux(&self, apple: libc::pid_t) -> Option<i32> {
        Some(apple)
    }

    fn linux_to_apple(&self, linux: i32) -> Option<libc::pid_t> {
        Some(linux)
    }

    fn register(&self, apple: libc::pid_t) -> i32 {
        crate::filesystem::procfs::add_process(&self.procfs, apple, apple);
        apple
    }

    fn unregister(&self, apple: libc::pid_t) {
        crate::filesystem::procfs::del_process(&self.procfs, apple);
    }

    fn parent(&self) -> Option<&Arc<dyn PidNamespace>> {
        None
    }

    fn procfs(&self) -> Option<Arc<dyn Mountable>> {
        Some(self.procfs.clone())
    }
}

/// The context of a process.
pub struct ProcessCtx {
    native_pid: AtomicI32,

    mnt_ns: RwLock<Arc<MountNamespace>>,
    uts_ns: RwLock<Arc<dyn UtsNamespace>>,
    pid_ns: RwLock<Arc<dyn PidNamespace>>,

    vfd_table: Registry<Arc<VirtualFd>>,
}
impl ProcessCtx {
    /// Creates a scratch process context for the given native PID.
    pub fn scratch(pid: i32) -> Arc<Self> {
        InitPid::instance().register(pid);
        Arc::new(Self {
            native_pid: pid.into(),
            mnt_ns: RwLock::new(MountNamespace::initial()),
            uts_ns: RwLock::new(Arc::new(InitUts)),
            pid_ns: RwLock::new(InitPid::instance()),
            vfd_table: Registry::new(),
        })
    }

    /// Forks this process context.
    pub async fn fork(&self) -> Arc<Self> {
        Arc::new(Self {
            native_pid: AtomicI32::new(0),
            mnt_ns: RwLock::new(self.mnt_ns()),
            uts_ns: RwLock::new(self.uts_ns()),
            pid_ns: RwLock::new(self.pid_ns()),
            vfd_table: crate::vfd::fork_table(&self.vfd_table).await,
        })
    }

    /// Returns the current mount namespace.
    pub fn mnt_ns(&self) -> Arc<MountNamespace> {
        self.mnt_ns.read().unwrap().clone()
    }

    /// Sets the current mount namespace.
    pub fn set_mnt_ns(&self, new: u64) -> Result<(), LxError> {
        let Some(new) = app().mnt_ns_registry.get(new) else {
            return Err(LxError::ENOENT);
        };
        let Some(new) = new.upgrade() else {
            app().mnt_ns_registry.gc();
            return Err(LxError::ENOENT);
        };
        *self.mnt_ns.write().unwrap() = new;
        Ok(())
    }

    /// Returns the current UTS namespace.
    pub fn uts_ns(&self) -> Arc<dyn UtsNamespace> {
        self.uts_ns.read().unwrap().clone()
    }

    /// Sets the current UTS namespace.
    pub fn set_uts_ns(&self, new: u64) -> Result<(), LxError> {
        todo!()
    }

    /// Returns the current PID namespace.
    pub fn pid_ns(&self) -> Arc<dyn PidNamespace> {
        self.pid_ns.read().unwrap().clone()
    }

    /// Sets the current PID namespace.
    pub fn set_pid_ns(&self, new: u64) -> Result<(), LxError> {
        let Some(new) = app().pid_ns_registry.get(new) else {
            return Err(LxError::ENOENT);
        };
        let Some(new) = new.upgrade() else {
            app().pid_ns_registry.gc();
            return Err(LxError::ENOENT);
        };
        // TODO: Should we unregister from the old PID namespace here?
        new.register(self.native_pid());
        *self.pid_ns.write().unwrap() = new;
        Ok(())
    }

    /// Returns the native PID of this process.
    pub fn native_pid(&self) -> i32 {
        self.native_pid.load(atomic::Ordering::Relaxed)
    }

    /// Sets the native PID of this process.
    pub fn set_native_pid(self: Arc<Self>, new: i32) {
        debug_assert_eq!(self.native_pid(), 0);
        self.native_pid.store(new, atomic::Ordering::Relaxed);
        self.pid_ns.read().unwrap().register(new);
        app().native_procs.pin().insert(new, self);
    }

    /// Retrieves a virtual file descriptor by its ID.
    pub fn vfd(&self, id: u64) -> Result<Arc<VirtualFd>, LxError> {
        self.vfd_table.get(id).ok_or(LxError::EBADF)
    }

    /// Registers a new virtual file descriptor, returning its ID.
    pub fn vfd_register(&self, object: Arc<VirtualFd>) -> u64 {
        self.vfd_table.register(object)
    }

    /// Closes a virtual file descriptor by its ID.
    pub fn vfd_close(&self, id: u64) {
        _ = self.vfd_table.unregister(id, true);
    }

    /// A function that should be called after an `exec` syscall.
    pub fn after_exec(&self) {
        crate::vfd::exec_table(&self.vfd_table);
    }
}
impl Drop for ProcessCtx {
    fn drop(&mut self) {
        self.pid_ns.read().unwrap().unregister(self.native_pid());
    }
}

/// Retrieves (or creates) a process context for the given native PID.
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

/// Closes the process context for the given native PID, if it is no longer referenced.
pub fn ctx_close(pid: i32) {
    let guard = app().native_procs.guard();
    if let Some(ctx) = app().native_procs.get(&pid, &guard) {
        if Arc::strong_count(ctx) <= 2 {
            app().native_procs.remove(&pid, &guard);
            guard.flush();
        }
    }
}
