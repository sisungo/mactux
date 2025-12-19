use crate::{
    emuctx::EmulatedThreadInfo,
    ipc_client::{Client, with_client},
    process,
    util::ipc_fail,
};
use rustc_hash::FxHashMap;
use std::{
    cell::{Cell, OnceCell, RefCell, UnsafeCell},
    ffi::c_void,
    ptr::NonNull,
    sync::{
        RwLock,
        atomic::{self, AtomicPtr, AtomicUsize},
    },
};
use structures::{
    error::LxError,
    internal::mactux_ipc::{Request, Response},
    process::CloneArgs,
    signal::{SigAltStack, SigNum},
    sync::{FutexOpts, RobustListHead},
    thread::TID_MIN,
};

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
    pub ipc_buf: RefCell<Vec<u8>>,
    pub clear_tid: Cell<Option<NonNull<u32>>>,
    pub sigaltstack: Cell<SigAltStack>,
}
impl ThreadCtx {
    /// Creates a new thread context. All fields are initialized to the "empty" values.
    pub fn new() -> Self {
        Self {
            tid: Cell::new(0),
            emulated_gsbase: Cell::new(std::ptr::null_mut()),
            thread_info_ptr: Cell::new(std::ptr::null()),
            client: OnceCell::new(),
            ipc_buf: RefCell::new(Vec::with_capacity(256)),
            clear_tid: Cell::new(None),
            sigaltstack: Cell::new(SigAltStack::default()),
        }
    }

    /// The thread-local storage destructor. Not intended to be used directly in Rust code.
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

/// Public thread context that does not require POSIX thread-local storage and may be accessed from another thread. This can
/// be used for shared thread data, or emulator environment, since we have no access to thread-local storage inside the emulated
/// context.
#[derive(Debug)]
pub struct ThreadPubCtx {
    pub emulation: EmulatedThreadInfo,
    pub robust_list_head: AtomicPtr<RobustListHead>,
    pub robust_list_head_size: AtomicUsize,
}
impl ThreadPubCtx {
    /// Creates a new [`ThreadPubCtx`] instance. All fields are initialized to their proper initial values.
    pub fn new() -> Self {
        Self {
            emulation: EmulatedThreadInfo::new(),
            robust_list_head: AtomicPtr::new(std::ptr::null_mut()),
            robust_list_head_size: AtomicUsize::new(0),
        }
    }
}
impl Clone for ThreadPubCtx {
    fn clone(&self) -> Self {
        Self {
            emulation: self.emulation.clone(),
            robust_list_head: AtomicPtr::new(self.robust_list_head.load(atomic::Ordering::Relaxed)),
            robust_list_head_size: AtomicUsize::new(
                self.robust_list_head_size.load(atomic::Ordering::Relaxed),
            ),
        }
    }
}

/// Returns TID of this thread.
pub fn id() -> i32 {
    with_context(|ctx| ctx.tid.get())
}

/// Kills a thread.
pub fn kill(tid: i32, signum: SigNum) -> Result<(), LxError> {
    if tid < TID_MIN {
        return crate::process::kill(tid, signum);
    }

    // TODO
    Err(LxError::ESRCH)
}

/// Gets `clear_child_tid` value of current thread.
#[inline]
pub fn get_clear_tid() -> Option<NonNull<u32>> {
    with_context(|ctx| ctx.clear_tid.get())
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

/// Sets robust list for current thread.
pub fn set_robust_list(ptr: *mut u8, size: usize) -> Result<(), LxError> {
    if size != size_of::<RobustListHead>() {
        return Err(LxError::EINVAL);
    }
    process::context().thread_pubctx_map.with_current(|ctx| {
        ctx.robust_list_head
            .store(ptr.cast(), atomic::Ordering::Relaxed);
        ctx.robust_list_head_size
            .store(size, atomic::Ordering::Relaxed);
    });
    Ok(())
}

pub fn get_name() -> [u8; 16] {
    let mut result = [0u8; 16];
    let buf = with_client(
        |client| match client.invoke(Request::GetThreadName).unwrap() {
            Response::Bytes(name) => name,
            _ => ipc_fail(),
        },
    );
    result.copy_from_slice(&buf);
    result
}

pub fn set_name(name: [u8; 16]) {
    with_client(|client| {
        client
            .invoke(Request::SetThreadName(name.to_vec()))
            .unwrap();
    });
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
    }

    process::context()
        .thread_pubctx_map
        .register(Box::new(ThreadPubCtx::new()));

    with_context(|ctx| {
        ctx.thread_info_ptr.set(
            process::context()
                .thread_pubctx_map
                .with_current(|x| &raw const x.emulation),
        )
    });

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
