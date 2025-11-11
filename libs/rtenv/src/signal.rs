use crate::{emuctx::in_emulated, process};
use libc::c_int;
use std::{
    mem::offset_of,
    sync::{
        Arc,
        atomic::{self, AtomicU8},
    },
};
use structures::{
    FromApple, ToApple,
    error::LxError,
    signal::{
        KernelSigSet, MaskHowto, SigAction, SigActionFlags, SigAltStack, SigHandler, SigInfo,
        SigNum,
    },
    time::ClockId,
    ucontext::UContext,
};

/// macOS signals that can be handled.
const HANDLED_SIGNALS: &[c_int] = &[
    libc::SIGHUP,
    libc::SIGINT,
    libc::SIGQUIT,
    libc::SIGILL,
    libc::SIGTRAP,
    libc::SIGEMT,
    libc::SIGFPE,
    libc::SIGBUS,
    libc::SIGPIPE,
    libc::SIGALRM,
    libc::SIGTERM,
    libc::SIGURG,
    libc::SIGTSTP,
    libc::SIGCHLD,
    libc::SIGTTIN,
    libc::SIGTTOU,
    libc::SIGIO,
    libc::SIGXCPU,
    libc::SIGXFSZ,
    libc::SIGVTALRM,
    libc::SIGPROF,
    libc::SIGWINCH,
    libc::SIGUSR1,
    libc::SIGUSR2,
];

/// Installs signal handlers.
pub fn install() -> std::io::Result<()> {
    install_for(libc::SIGSEGV, handle_sigsegv)?;
    install_for(libc::SIGABRT, handle_sigabrt)?;

    Ok(())
}

/// Raises a signal in the emulated context. This must be called out of the emulated context.
#[cfg(target_arch = "x86_64")]
pub fn raise(
    signum: SigNum,
    info: &libc::siginfo_t,
    ctx: &mut libc::ucontext_t,
    prev_in_emulated: bool,
) {
    let restore_emulation = || {
        if prev_in_emulated {
            unsafe {
                crate::emuctx::enter_emulated();
            }
        }
    };
    let action = sigaction(signum, None).unwrap();
    if action.handler == SigHandler::SIG_DFL {
        let Ok(apple_signum) = signum.to_apple() else {
            crate::error_report::fast_fail();
        };
        unsafe {
            libc::signal(apple_signum, libc::SIG_DFL);
        }
        restore_emulation();
        return;
    } else if action.handler == SigHandler::SIG_IGN || action.handler == SigHandler::SIG_HOLD {
        restore_emulation();
        return;
    }

    let ret_addr = if action.flags.contains(SigActionFlags::SA_RESTORER) {
        action.restorer
    } else {
        linux_restore as usize
    };

    unsafe {
        let sigframe = SignalStackFrame {
            ret_addr,
            info: linux_siginfo(signum, info),
            ucontext: UContext::from_apple(ctx),
            prev_in_emulated,
        };
        let sigaltstack = sigaltstack(None);
        if !sigaltstack.ss_sp.is_null() {
            (*ctx.uc_mcontext).__ss.__rsp =
                sigaltstack.ss_sp.add(sigaltstack.ss_size) as usize as u64;
        }
        (*ctx.uc_mcontext).__ss.__rsp -= size_of::<SignalStackFrame>() as u64;
        ((*ctx.uc_mcontext).__ss.__rsp as usize as *mut SignalStackFrame).write(sigframe);

        (*ctx.uc_mcontext).__ss.__rip = action.handler.0 as u64;

        (*ctx.uc_mcontext).__ss.__rdi = signum.0 as _;
        (*ctx.uc_mcontext).__ss.__rsi =
            (*ctx.uc_mcontext).__ss.__rsp + offset_of!(SignalStackFrame, info) as u64;
        if action.flags.contains(SigActionFlags::SA_SIGINFO) {
            (*ctx.uc_mcontext).__ss.__rdx =
                (*ctx.uc_mcontext).__ss.__rsp + offset_of!(SignalStackFrame, ucontext) as u64;
        }

        crate::emuctx::enter_emulated();
    }
}

/// Implementation of the `rt_sigreturn` system call.
#[cfg(target_arch = "x86_64")]
pub unsafe fn sigreturn(ctx: &mut libc::ucontext_t) {
    unsafe {
        (*ctx.uc_mcontext).__ss.__rsp -= offset_of!(SignalStackFrame, info) as u64;
        let frame = ((*ctx.uc_mcontext).__ss.__rsp as *const SignalStackFrame).read();
        frame
            .ucontext
            .uc_mcontext
            .write_to_apple(&mut *ctx.uc_mcontext);
        if !frame.prev_in_emulated {
            crate::emuctx::leave_emulated();
        }
    }
}

/// Returns `true` if the given signal info is asynchronous.
#[inline]
pub const fn is_async(info: &libc::siginfo_t) -> bool {
    const SI_USER: c_int = 65537;

    (info.si_code & SI_USER) != 0
}

/// Executes a closure without asynchoronous signals.
#[inline]
pub fn without_signals<T>(f: impl FnOnce() -> T) -> T {
    let old_set = unsafe {
        let mut old_set = std::mem::zeroed();
        let mut set = std::mem::zeroed();
        libc::sigfillset(&mut set);
        if libc::pthread_sigmask(libc::SIG_SETMASK, &set, &mut old_set) == -1 {
            crate::error_report::fast_fail();
        }
        old_set
    };
    let value = f();
    unsafe {
        if libc::pthread_sigmask(libc::SIG_SETMASK, &old_set, std::ptr::null_mut()) == -1 {
            crate::error_report::fast_fail();
        }
    }
    value
}

pub fn mask(howto: MaskHowto, set: Option<KernelSigSet>) -> Result<KernelSigSet, LxError> {
    unsafe {
        let mut old = std::mem::zeroed();
        let mut set = set.map(|x| x.to_apple());
        let pset = match &mut set {
            Some(x) => x as *mut _,
            None => std::ptr::null_mut(),
        };
        match libc::pthread_sigmask(howto.to_apple()?, pset, &mut old) {
            -1 => Err(LxError::last_apple_error()),
            _ => Ok(KernelSigSet::from_apple(old)),
        }
    }
}

pub fn sigaction(signum: SigNum, new: Option<SigAction>) -> Result<SigAction, LxError> {
    let old = **process::context()
        .sigactions
        .get(signum.0 as usize)
        .ok_or(LxError::EINVAL)?
        .load();
    let Some(new) = new else {
        return Ok(old);
    };

    let apple_signum = signum.to_apple()?;
    if !HANDLED_SIGNALS.contains(&apple_signum) {
        process::context()
            .sigactions
            .get(signum.0 as usize)
            .unwrap()
            .store(Arc::new(new));
        return sigaction(signum, None);
    }

    let mut apple_sigaction: libc::sigaction = unsafe { std::mem::zeroed() };
    apple_sigaction.sa_sigaction = match new.handler {
        SigHandler::SIG_DFL => libc::SIG_DFL,
        SigHandler::SIG_IGN => libc::SIG_IGN,
        SigHandler::SIG_HOLD => {
            mask(
                MaskHowto::SIG_BLOCK,
                Some(KernelSigSet::from_iter([signum].into_iter())),
            )?;
            return Ok(old);
        }
        _ => handle_signal as usize,
    };
    apple_sigaction.sa_flags = new.flags.to_apple();
    apple_sigaction.sa_mask = new.mask.to_apple();

    let old = process::context()
        .sigactions
        .get(signum.0 as usize)
        .unwrap()
        .swap(Arc::new(new));

    unsafe {
        match libc::sigaction(apple_signum, &apple_sigaction, std::ptr::null_mut()) {
            -1 => Err(LxError::last_apple_error()),
            _ => Ok(*old),
        }
    }
}

pub fn sigaltstack(new: Option<SigAltStack>) -> SigAltStack {
    crate::thread::with_context(|ctx| {
        if let Some(new) = new {
            ctx.sigaltstack.set(new);
        }
        ctx.sigaltstack.get()
    })
}

/// Converts from apple `siginfo` to the Linux one.
fn linux_siginfo(signum: SigNum, apple: &libc::siginfo_t) -> SigInfo {
    SigInfo {
        si_signo: signum.0 as _,
        si_errno: apple.si_errno,
        si_code: apple.si_code,
        si_trapno: 0,
        si_pid: apple.si_pid, // TODO
        si_uid: apple.si_uid,
        si_status: apple.si_status,
        si_utime: ClockId(0),
        si_value: 0, // TODO
        si_int: 0,
        si_ptr: std::ptr::null_mut(),
        si_overrun: 0,
        si_timerid: 0,
        si_addr: std::ptr::null_mut(),
        si_band: 0,
        si_fd: 0,
        si_addr_lsb: 0,
        si_lower: std::ptr::null_mut(),
        si_upper: std::ptr::null_mut(),
        si_pkey: 0,
        si_call_addr: std::ptr::null_mut(),
        si_syscall: 0,
        si_arch: 0,
    }
}

/// Installs a signal handler.
fn install_for(
    signum: c_int,
    handler: unsafe extern "C" fn(c_int, &libc::siginfo_t, &mut libc::ucontext_t),
) -> std::io::Result<()> {
    let mut old_sigaction = unsafe { std::mem::zeroed() };
    let sigaction = libc::sigaction {
        sa_sigaction: handler as _,
        sa_mask: 0,
        sa_flags: libc::SA_SIGINFO | libc::SA_RESTART,
    };
    let status = unsafe { libc::sigaction(signum, &sigaction, &mut old_sigaction) };

    match status {
        0 => Ok(()),
        _ => Err(std::io::Error::last_os_error()),
    }
}

/// Handles a signal.
unsafe extern "C" fn handle_signal(
    signum: c_int,
    info: &libc::siginfo_t,
    ctx: &mut libc::ucontext_t,
) {
    let in_emulated = reentrant_in_emulated(info);
    let Ok(signum) = SigNum::from_apple(signum) else {
        crate::error_report::fast_fail();
    };

    unsafe {
        if in_emulated {
            crate::emuctx::leave_emulated();
        }
    }
    raise(signum, info, ctx, in_emulated);
}

/// Handles SIGSEGV.
#[cfg(target_arch = "x86_64")]
unsafe extern "C" fn handle_sigsegv(_: c_int, info: &libc::siginfo_t, ctx: &mut libc::ucontext_t) {
    // This special handler may process all `fs` accesses to `gs` ones.
    if !reentrant_in_emulated(info) {
        return raise(SigNum::SIGSEGV, info, ctx, false);
    }

    unsafe {
        let insc_byte = (*ctx.uc_mcontext).__ss.__rip as usize as *const AtomicU8;
        match (*insc_byte).compare_exchange(
            0x64,
            0x65,
            atomic::Ordering::Relaxed,
            atomic::Ordering::Relaxed,
        ) {
            Ok(_) => (),
            Err(_) => {
                crate::emuctx::leave_emulated();
                raise(SigNum::SIGSEGV, info, ctx, true);
            }
        }
    }
}

/// Handles SIGABRT.
unsafe extern "C" fn handle_sigabrt(_: c_int, info: &libc::siginfo_t, ctx: &mut libc::ucontext_t) {
    let prev_in_emulated = reentrant_in_emulated(info);
    if prev_in_emulated {
        unsafe {
            crate::emuctx::leave_emulated();
        }
    }

    if prev_in_emulated || is_async(info) {
        raise(SigNum::SIGABRT, info, ctx, prev_in_emulated);
    } else {
        crate::error_report::fast_fail();
    }
}

/// Reentrantly judges if we are in the emulated context.
fn reentrant_in_emulated(info: &libc::siginfo_t) -> bool {
    if is_async(info) {
        without_signals(in_emulated)
    } else {
        in_emulated()
    }
}

/// Stack frame of a signal handler.
#[derive(Debug, Clone)]
#[repr(C)]
struct SignalStackFrame {
    ret_addr: usize,
    info: SigInfo,
    ucontext: UContext,
    prev_in_emulated: bool,
}

/// Default implementation of the signal restorer.
#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
unsafe extern "sysv64" fn linux_restore() -> ! {
    std::arch::naked_asm! {
        "mov rax, 15",
        "syscall",
    }
}
