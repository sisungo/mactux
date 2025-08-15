use crate::{emuctx::EmulatedThreadInfo, ipc_client::Client, process};
use rustc_hash::FxHashMap;
use std::{
    cell::{Cell, OnceCell, RefCell, UnsafeCell},
    ffi::c_void,
    ptr::NonNull,
    sync::RwLock,
};
use structures::{error::LxError, process::CloneArgs, signal::SigNum, sync::FutexOpts};

/// Minimal TID that indicates a non-main thread rather than a process (or, the "main thread").
const MINIMUM_TID: i32 = 0x40000000;

static mut THREAD_CTX: libc::pthread_key_t = unsafe { std::mem::zeroed() };

/// Installs the thread context.
pub unsafe fn install() -> std::io::Result<()> {
    unsafe {
        if libc::pthread_key_create(&raw mut THREAD_CTX, Some(ThreadCtx::destructor)) == -1 {
            return Err(std::io::Error::last_os_error());
        }
        enter()?;
    }

    with_context(|ctx| unsafe {
        ctx.tid.set(libc::getpid());
    });

    Ok(())
}

/// Context of a thread.
#[derive(Debug)]
pub struct ThreadCtx {
    pub tid: Cell<i32>,
    pub emulated_gsbase: Cell<*mut u8>,
    pub thread_info_ptr: Cell<*const EmulatedThreadInfo>,
    pub client: OnceCell<RefCell<Client>>,
    pub clear_tid: Cell<Option<NonNull<u32>>>,
}
impl ThreadCtx {
    pub fn new() -> Self {
        Self {
            tid: Cell::new(0),
            emulated_gsbase: Cell::new(std::ptr::null_mut()),
            thread_info_ptr: Cell::new(std::ptr::null()),
            client: OnceCell::new(),
            clear_tid: Cell::new(None),
        }
    }

    unsafe extern "C" fn destructor(data: *mut c_void) {
        unsafe {
            (data as *mut Self).drop_in_place();
            drop(Box::from_raw(data as *mut Self));
        }
    }
}
impl Default for ThreadCtx {
    fn default() -> Self {
        Self::new()
    }
}

/// Executes a closure with context of current thread.
pub fn with_context<T>(f: impl FnOnce(&ThreadCtx) -> T) -> T {
    unsafe { f(&*libc::pthread_getspecific((&raw const THREAD_CTX).read()).cast::<ThreadCtx>()) }
}

/// The thread public context map.
#[derive(Debug)]
pub struct ThreadPubCtxMap(UnsafeCell<RwLock<FxHashMap<libc::pid_t, Box<ThreadPubCtx>>>>);
impl ThreadPubCtxMap {
    /// Creates a new [`ThreadPubCtxMap`] instance.
    pub fn new() -> Self {
        Self(UnsafeCell::new(RwLock::default()))
    }

    /// Registers current process to the map.
    pub fn register(&self, ctx: Box<ThreadPubCtx>) -> *const ThreadPubCtx {
        unsafe {
            let ptr = &raw const *ctx;
            (*self.0.get())
                .write()
                .unwrap()
                .insert(thread_selfid(), ctx);
            ptr
        }
    }

    /// Unregisters current process from the map.
    pub fn unregister(&self) {
        unsafe {
            (*self.0.get()).write().unwrap().remove(&thread_selfid());
        }
    }

    /// Executes a closure with [`ThreadInfo`] for current thread.
    pub fn with_current<T>(&self, f: impl FnOnce(&ThreadPubCtx) -> T) -> T {
        unsafe {
            f((*self.0.get())
                .read()
                .unwrap()
                .get(&thread_selfid())
                .unwrap())
        }
    }

    /// This is called on the new process after `fork()`.
    pub fn after_fork(&self, current: Box<ThreadPubCtx>) {
        unsafe {
            std::mem::forget(self.0.get().replace(RwLock::default()));
            (*self.0.get())
                .write()
                .unwrap()
                .insert(thread_selfid(), current);
        }
    }
}
unsafe impl Send for ThreadPubCtxMap {}
unsafe impl Sync for ThreadPubCtxMap {}

/// Executes a closure `fork` that may run the `fork()` system call, and calls `is_new()` to judge if the return value
/// indicates a new process. Necessary pre- and post-fork work will be done.
pub fn may_fork<T>(fork: impl FnOnce() -> T, is_new: impl FnOnce(&T) -> bool) -> T {
    let ctx = process::context()
        .thread_pubctx_map
        .with_current(|ctx| Box::new(ctx.clone()));
    let thread_info_ptr = &raw const ctx.emulation;
    let result = fork();
    if is_new(&result) {
        process::context().thread_pubctx_map.after_fork(ctx);
        with_context(|ctx| ctx.thread_info_ptr.set(thread_info_ptr));
    }
    result
}

#[derive(Debug, Clone)]
pub struct ThreadPubCtx {
    pub emulation: EmulatedThreadInfo,
}
impl ThreadPubCtx {
    pub fn new() -> Self {
        Self {
            emulation: EmulatedThreadInfo::new(),
        }
    }
}

/// Returns TID of this thread.
pub fn id() -> i32 {
    with_context(|ctx| ctx.tid.get())
}

/// Kills a thread.
pub fn kill(tid: i32, signum: SigNum) -> Result<(), LxError> {
    if tid < MINIMUM_TID {
        return crate::process::kill(tid, signum);
    }

    // TODO
    Err(LxError::ESRCH)
}

/// Sets `clear_child_tid` value for current thread.
#[inline]
pub fn set_clear_tid(value: Option<NonNull<u32>>) {
    with_context(|ctx| ctx.clear_tid.set(value));
}

/// Spawns a thread.
pub fn clone(args: CloneArgs) -> Result<i32, LxError> {
    todo!()
}

/// This is called when entering a MacTux thread.
pub unsafe fn enter() -> std::io::Result<()> {
    unsafe {
        if libc::pthread_setspecific(
            (&raw const THREAD_CTX).read(),
            Box::into_raw(Box::new(ThreadCtx::new())).cast(),
        ) == -1
        {
            return Err(std::io::Error::last_os_error());
        }
        process::context()
            .thread_pubctx_map
            .register(Box::new(ThreadPubCtx::new()));
        crate::emuctx::enter_thread();
    }
    Ok(())
}

/// This is called when exiting a MacTux thread.
pub unsafe fn exit(code: i32) -> ! {
    unsafe {
        if let Some(ptr) = with_context(|ctx| ctx.clear_tid.get()) {
            _ = crate::sync::futex::wake(ptr.as_ptr(), 0, FutexOpts::empty());
        }
        process::context().thread_pubctx_map.unregister();
        libc::pthread_exit(code as usize as _); // TODO: CLS Destruction?
    }
}

/// The macOS raw system call `thread_selfid`.
fn thread_selfid() -> libc::pid_t {
    unsafe { libc::syscall(372) }
}
