use crate::ipc_client::Client;
use std::{
    cell::{Cell, OnceCell, RefCell},
    ffi::c_void,
    ptr::NonNull,
};
use structures::{error::LxError, signal::SigNum, sync::FutexOpts};

const MINIMUM_TID: i32 = 0x40000000;

static mut THREAD_CTX: libc::pthread_key_t = unsafe { std::mem::zeroed() };

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

#[derive(Debug)]
pub struct ThreadCtx {
    pub tid: Cell<i32>,
    pub emulated_gsbase: Cell<*mut u8>,
    pub thread_info_ptr: Cell<*const u8>,
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

pub fn with_context<T>(f: impl FnOnce(&ThreadCtx) -> T) -> T {
    unsafe { f(&*libc::pthread_getspecific((&raw const THREAD_CTX).read()).cast::<ThreadCtx>()) }
}

pub fn id() -> i32 {
    with_context(|ctx| ctx.tid.get())
}

pub fn kill(tid: i32, signum: SigNum) -> Result<(), LxError> {
    if tid < MINIMUM_TID {
        return crate::process::kill(tid, signum);
    }

    // TODO
    Err(LxError::ESRCH)
}

#[inline]
pub fn set_clear_tid(value: Option<NonNull<u32>>) {
    with_context(|ctx| ctx.clear_tid.set(value));
}

pub unsafe fn enter() -> std::io::Result<()> {
    unsafe {
        if libc::pthread_setspecific(
            (&raw const THREAD_CTX).read(),
            Box::into_raw(Box::new(ThreadCtx::new())).cast(),
        ) == -1
        {
            return Err(std::io::Error::last_os_error());
        }
        crate::emuctx::enter_thread();
    }
    Ok(())
}

pub unsafe fn exit(code: i32) -> ! {
    unsafe {
        if let Some(ptr) = with_context(|ctx| ctx.clear_tid.get()) {
            _ = crate::sync::futex::wake(ptr.as_ptr(), 0, FutexOpts::empty());
        }

        crate::emuctx::exit_thread();
        libc::pthread_exit(code as usize as _); // TODO: CLS Destruction?
    }
}
