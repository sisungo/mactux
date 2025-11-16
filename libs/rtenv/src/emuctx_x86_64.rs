use crate::{process, thread};
use std::cell::Cell;

unsafe extern "C" {
    /// Apple Private API. This sets the GSBASE register to the specified value.
    fn _thread_set_tsd_base(gsbase: *mut u8);
}

/// Enters the emulated context. This must be called out of the emulated context.
pub unsafe fn enter_emulated() {
    unsafe {
        let emulated_gsbase = thread::with_context(|ctx| {
            (*ctx.thread_info_ptr.get().cast::<EmulatedThreadInfo>())
                .in_emulated
                .set(true);
            ctx.emulated_gsbase.get()
        });
        _thread_set_tsd_base(emulated_gsbase);
    }
}

/// Leaves the emulated context. This must be called in the emulated context.
pub unsafe fn leave_emulated() {
    unsafe {
        let native_gsbase = process::context().thread_pubctx_map.with_current(|ctx| {
            ctx.emulation.in_emulated.set(false);
            ctx.emulation.native_gsbase
        });
        _thread_set_tsd_base(native_gsbase as _);
    }
}

/// Returns `true` if we are in the emulated context.
pub fn in_emulated() -> bool {
    process::context()
        .thread_pubctx_map
        .with_current(|ctx| ctx.emulation.in_emulated.get())
}

/// Sets value of the GSBASE register when entering the emulated context.
pub fn x86_64_set_emulated_gsbase(new: *mut u8) {
    thread::with_context(|ctx| ctx.emulated_gsbase.set(new));
}

/// Thread information.
#[derive(Debug, Clone)]
pub struct EmulatedThreadInfo {
    native_gsbase: usize,
    in_emulated: Cell<bool>,
}
impl EmulatedThreadInfo {
    /// Creates a [`EmulatedThreadInfo`] instance for current thread.
    pub fn new() -> Self {
        Self {
            native_gsbase: current_gsbase(),
            in_emulated: Cell::new(false),
        }
    }
}

/// Returns current value of the GSBASE register. This may only be called out of the emulated context, or the return value
/// is unspecified.
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
