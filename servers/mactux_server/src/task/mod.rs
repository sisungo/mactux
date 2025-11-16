pub mod process;
pub mod thread;
pub mod tid_alloc;

use crate::{
    app,
    filesystem::{procfs, tmpfs::Tmpfs, vfs::Filesystem},
    task::thread::Thread,
    util::Shared,
};
use process::Process;
use std::sync::Arc;
use structures::{error::LxError, thread::TID_MIN};

/// A pid namespace.
///
/// Note that "native pid" represents to both macOS PIDs and MacTux thread IDs.
pub trait PidNamespace: Send + Sync {
    /// Converts from native PID to Linux PID in this namespace.
    ///
    /// This would return an error [`LxError::ENOENT`] if the process is not registered in this namespace.
    fn ntol(&self, native: i32) -> Result<i32, LxError>;

    /// Converts from Linux PID in this namespace to its native PID.
    ///
    /// This would return an error [`LxError::ENOENT`] if the process is not registered in this namespace.
    fn lton(&self, linux: i32) -> Result<i32, LxError>;

    /// Registers a native PID to this namespace, returning the allocated Linux PID in this namespace on success.
    ///
    /// This would return an error [`LxError::EEXIST`] if the native PID is already registered in this namespace.
    fn register(&self, native: i32) -> Result<i32, LxError>;

    /// Unregisters a process from this namespace by its native PID.
    ///
    /// This would return an error [`LxError::ENOENT`] if the native PID is not registered in this namespace.
    fn unregister(&self, native_pid: i32, native_tid: i32) -> Result<(), LxError>;

    /// Returns parent of this PID namespace.
    ///
    /// This would return `None` if this is the initial PID namespace.
    fn parent(&self) -> Option<Shared<Box<dyn PidNamespace>>>;

    /// Creates a child namespace of this.
    fn child(&self) -> Shared<Box<dyn PidNamespace>>;

    /// Returns `procfs` instance associated with this pid namespace.
    fn procfs(&self) -> Result<Arc<dyn Filesystem>, LxError>;
}

pub struct InitPid {
    procfs: Arc<Tmpfs>,
}
impl InitPid {
    pub fn new() -> Self {
        let procfs = procfs::new().expect("it should never fail to create procfs for init_pid");
        Self { procfs }
    }
}
impl PidNamespace for InitPid {
    fn ntol(&self, native: i32) -> Result<i32, LxError> {
        match app().threads.get(native as _) {
            Some(_) => Ok(native),
            None => Err(LxError::ENOENT),
        }
    }

    fn lton(&self, linux: i32) -> Result<i32, LxError> {
        match app().threads.get(linux as _) {
            Some(_) => Ok(linux),
            None => Err(LxError::ENOENT),
        }
    }

    fn register(&self, native: i32) -> Result<i32, LxError> {
        if native < TID_MIN {
            procfs::add_proc(&self.procfs, native, native)?;
        }
        procfs::add_thread(self, &self.procfs, native)?;
        Ok(native)
    }

    fn unregister(&self, native_pid: i32, native_tid: i32) -> Result<(), LxError> {
        if native_tid < TID_MIN {
            procfs::del_proc(&self.procfs, native_tid)?;
        } else {
            procfs::del_thread(&self.procfs, native_pid, native_tid)?;
        }
        Ok(())
    }

    fn parent(&self) -> Option<Shared<Box<dyn PidNamespace>>> {
        None
    }

    fn child(&self) -> Shared<Box<dyn PidNamespace>> {
        todo!()
    }

    fn procfs(&self) -> Result<Arc<dyn Filesystem>, LxError> {
        Ok(self.procfs.clone())
    }
}

pub fn configure() -> Configuration {
    Configuration::new()
}

pub struct Configuration {
    parent: Shared<Process>,
    apple_pid: i32,
}
impl Configuration {
    pub fn new() -> Self {
        Self {
            parent: Process::server(),
            apple_pid: std::process::id() as _,
        }
    }

    pub fn parent(&mut self, parent: Shared<Process>) -> &mut Self {
        self.parent = parent;
        self
    }

    pub fn apple_pid(&mut self, apple_pid: i32) -> &mut Self {
        self.apple_pid = apple_pid;
        self
    }

    pub fn exec(&mut self) -> Result<(), LxError> {
        // First, try to acquire/create process from global registry.
        if self.apple_pid != -1 {
            let (proc, created) = app()
                .processes
                .tempt::<_, ()>(self.apple_pid as _, || Ok(self.parent._child()))
                .unwrap();
            let mut thread_builder = Thread::builder();
            thread_builder.process(proc);
            if created {
                thread_builder.is_main();
            }
            let thread = thread_builder.build()?;
            self.parent.pid.register(thread.tid())?;
            Thread::set_current(thread);
        }
        Ok(())
    }
}
