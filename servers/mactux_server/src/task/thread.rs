//! Context and operations for a Linux thread.
//!
//! A Linux thread directly maps to a server thread.

use super::tid_alloc::{alloc as tid_alloc, dealloc as tid_dealloc};
use crate::{app, task::process::Process, util::Shared};
use std::{cell::UnsafeCell, sync::RwLock};
use structures::{error::LxError, thread::TID_MIN};

thread_local! {
    static CURRENT: UnsafeCell<Shared<Thread>> = UnsafeCell::new(Thread::server());
}

pub struct Thread {
    tid: i32,
    pub process: Shared<Process>,
    pub comm: RwLock<Option<Vec<u8>>>,
}
impl Thread {
    pub fn server() -> Shared<Self> {
        app().server_thread.get().unwrap().clone()
    }

    pub fn current() -> Shared<Self> {
        CURRENT.with(|c| unsafe { (*c.get()).clone() })
    }

    pub fn builder() -> Builder {
        Builder::new()
    }

    pub fn set_current(new: Shared<Self>) {
        CURRENT.with(|c| unsafe { c.get().replace(new) });
    }

    pub fn process(&self) -> Shared<Process> {
        self.process.clone()
    }

    pub fn tid(&self) -> i32 {
        self.tid
    }
}
impl Drop for Thread {
    fn drop(&mut self) {
        if self.tid >= TID_MIN {
            tid_dealloc(self.tid);
        }
        _ = self
            .process
            .pid
            .unregister(Shared::id(&self.process) as _, self.tid);
        self.process.threads.remove(&self.tid);
    }
}

pub struct Builder {
    process: Option<Shared<Process>>,
    is_main: bool,
}
impl Builder {
    pub fn new() -> Self {
        Self {
            process: None,
            is_main: false,
        }
    }

    pub fn process(&mut self, process: Shared<Process>) -> &mut Self {
        self.process = Some(process);
        self
    }

    pub fn is_main(&mut self) -> &mut Self {
        self.is_main = true;
        self
    }

    pub fn build(&mut self) -> Result<Shared<Thread>, LxError> {
        let process = self.process.take().ok_or(LxError::EPERM)?;
        let tid = match self.is_main {
            true => Shared::id(&process) as i32,
            false => tid_alloc()?,
        };
        process.threads.insert(tid);
        Ok(app().threads.intervene(
            tid as _,
            Thread {
                tid,
                process,
                comm: None.into(),
            },
        ))
    }
}
