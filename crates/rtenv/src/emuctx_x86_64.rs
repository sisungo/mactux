use crate::{process, thread};
use rustc_hash::FxHashMap;
use std::{
    cell::{Cell, UnsafeCell},
    sync::RwLock,
};

unsafe extern "C" {
    fn _thread_set_tsd_base(gsbase: *mut u8);
}

pub fn enter_thread() {
    thread::with_context(|ctx| {
        ctx.thread_info_ptr
            .set(crate::process::context().thread_info_map.register().cast())
    });
}

pub fn exit_thread() {
    process::context().thread_info_map.unregister();
}

pub unsafe fn enter_emulated() {
    unsafe {
        thread::with_context(|ctx| {
            (*ctx.thread_info_ptr.get().cast::<ThreadInfo>())
                .in_emulated
                .set(true)
        });
        let emulated_gsbase = thread::with_context(|ctx| ctx.emulated_gsbase.get());
        _thread_set_tsd_base(emulated_gsbase);
    }
}

pub unsafe fn leave_emulated() {
    unsafe {
        let native_gsbase = process::context()
            .thread_info_map
            .with_thread_info(|thread_info| {
                thread_info.in_emulated.set(false);
                thread_info.native_gsbase
            });
        _thread_set_tsd_base(native_gsbase as _);
    }
}

pub fn in_emulated() -> bool {
    process::context()
        .thread_info_map
        .with_thread_info(|info| info.in_emulated.get())
}

pub fn x86_64_set_emulated_gsbase(new: *mut u8) {
    thread::with_context(|ctx| ctx.emulated_gsbase.set(new));
}

pub fn may_fork<T>(fork: impl FnOnce() -> T, is_new: impl FnOnce(&T) -> bool) -> T {
    let thread_info = process::context()
        .thread_info_map
        .with_thread_info(|info| Box::new(info.clone()));
    let thread_info_ptr = &raw const *thread_info;
    let result = fork();
    if is_new(&result) {
        process::context().thread_info_map.after_fork(thread_info);
        thread::with_context(|ctx| ctx.thread_info_ptr.set(thread_info_ptr.cast()));
    }
    result
}

#[derive(Debug)]
pub struct ThreadInfoMap(UnsafeCell<RwLock<FxHashMap<libc::pid_t, Box<ThreadInfo>>>>);
impl ThreadInfoMap {
    pub fn new() -> Self {
        Self(UnsafeCell::new(RwLock::default()))
    }

    fn register(&self) -> *const ThreadInfo {
        unsafe {
            let thread_info = Box::new(ThreadInfo::new());
            let ptr = &raw const *thread_info;
            (*self.0.get())
                .write()
                .unwrap()
                .insert(thread_selfid(), thread_info);
            ptr
        }
    }

    fn unregister(&self) {
        unsafe {
            (*self.0.get()).write().unwrap().remove(&thread_selfid());
        }
    }

    fn with_thread_info<T>(&self, f: impl FnOnce(&ThreadInfo) -> T) -> T {
        unsafe {
            f((*self.0.get())
                .read()
                .unwrap()
                .get(&thread_selfid())
                .unwrap())
        }
    }

    fn after_fork(&self, current: Box<ThreadInfo>) {
        unsafe {
            std::mem::forget(self.0.get().replace(RwLock::default()));
            (*self.0.get())
                .write()
                .unwrap()
                .insert(thread_selfid(), current);
        }
    }
}
unsafe impl Send for ThreadInfoMap {}
unsafe impl Sync for ThreadInfoMap {}

#[derive(Debug, Clone)]
struct ThreadInfo {
    native_gsbase: usize,
    in_emulated: Cell<bool>,
}
impl ThreadInfo {
    fn new() -> Self {
        Self {
            native_gsbase: current_gsbase(),
            in_emulated: Cell::new(false),
        }
    }
}

fn thread_selfid() -> libc::pid_t {
    unsafe { libc::syscall(372) }
}

fn current_gsbase() -> usize {
    let mut tiinfo: libc::thread_identifier_info = unsafe { std::mem::zeroed() };
    let mut info_count = libc::THREAD_IDENTIFIER_INFO_COUNT;
    let thread_self = unsafe { mach2::mach_init::mach_thread_self() };
    let kr = unsafe {
        libc::thread_info(
            thread_self,
            libc::THREAD_IDENTIFIER_INFO as _,
            (&raw mut tiinfo).cast(),
            (&raw mut info_count).cast(),
        )
    };
    unsafe {
        mach2::mach_port::mach_port_deallocate(mach2::traps::mach_task_self(), thread_self);
    }
    if kr == libc::KERN_SUCCESS {
        tiinfo.thread_handle as usize
    } else {
        0
    }
}
