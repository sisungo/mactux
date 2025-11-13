#[cfg(target_arch = "x86_64")]
#[path = "x86_64.rs"]
mod arch;

mod common;
mod util;

use std::{ffi::CStr, ptr::NonNull};
use structures::{
    FromApple,
    device::DeviceNumber,
    error::LxError,
    fs::{AccessFlags, AtFlags, OpenFlags, UmountFlags},
    io::{CloseRangeFlags, EventFdFlags, FcntlCmd, FlockOp, IoctlCmd, Whence},
    misc::GrndFlags,
    mm::{Madvice, MmapFlags, MmapProt, MremapFlags, MsyncFlags},
    net::{Domain, Protocol, ShutdownHow, SockOpt, SockOptLevel, SocketFlags, SocketType},
    process::{PrctlOp, RLimitable, RUsageWho, WaitOptions},
    signal::{MaskHowto, SigNum},
    sync::FutexOp,
    time::{ClockId, TimerFlags},
};

/// Install the system call emulation signal handlers.
pub fn install() -> std::io::Result<()> {
    let mut old_sigaction = unsafe { std::mem::zeroed() };
    let sigaction = libc::sigaction {
        sa_sigaction: arch::handle_sigsys as _,
        sa_mask: 0,
        sa_flags: libc::SA_SIGINFO | libc::SA_NODEFER,
    };
    let status = unsafe { libc::sigaction(libc::SIGSYS, &sigaction, &mut old_sigaction) };

    match status {
        0 => Ok(()),
        _ => Err(std::io::Error::last_os_error()),
    }
}

/// Type of a system call handler.
type SystemCallHandler = unsafe fn(&mut libc::ucontext_t);

trait UcontextExt {
    fn sysno(&self) -> usize;
    fn arg0(&self) -> usize;
    fn arg1(&self) -> usize;
    fn arg2(&self) -> usize;
    fn arg3(&self) -> usize;
    fn arg4(&self) -> usize;
    fn arg5(&self) -> usize;
    fn ret(&mut self, value: usize);
}

/// Converts a type that can be constructed from a system call argument.
trait FromSyscall {
    /// Converts `value` passed from system call to this type.
    fn from_syscall(value: usize) -> Self;
}

/// Converts a type to a system call return value.
trait ToSysret {
    /// Converts `self` to a value that can be returned by system call.
    fn to_sysret(self) -> usize;
}

macro_rules! impl_from_to_sys_plain {
    ($t:ty) => {
        impl FromSyscall for $t {
            fn from_syscall(value: usize) -> Self {
                value as $t
            }
        }
        impl ToSysret for $t {
            fn to_sysret(self) -> usize {
                self as usize
            }
        }
    };
    ($($t:ty);*) => {
        $(
            impl_from_to_sys_plain!($t);
        )*
    };
}
macro_rules! impl_from_to_sys_bitflags {
    ($t:ty) => {
        impl FromSyscall for $t {
            fn from_syscall(value: usize) -> Self {
                Self::from_bits_retain(value as _)
            }
        }
        impl ToSysret for $t {
            fn to_sysret(self) -> usize {
                self.bits() as _
            }
        }
    };
    ($($t:ty);*) => {
        $(
            impl_from_to_sys_bitflags!($t);
        )*
    };
}
macro_rules! impl_from_to_sys_newtype {
    ($t:ty) => {
        impl FromSyscall for $t {
            fn from_syscall(value: usize) -> Self {
                Self(FromSyscall::from_syscall(value))
            }
        }
        impl ToSysret for $t {
            fn to_sysret(self) -> usize {
                ToSysret::to_sysret(self.0)
            }
        }
    };
    ($($t:ty);*) => {
        $(
            impl_from_to_sys_newtype!($t);
        )*
    };
}
impl_from_to_sys_plain!(i8; u8; i16; u16; i32; u32; i64; u64; isize; usize);
impl_from_to_sys_bitflags!(
    MmapFlags; OpenFlags; AtFlags; MmapProt; GrndFlags; AccessFlags; WaitOptions; MsyncFlags;
    MremapFlags; SocketFlags; EventFdFlags; TimerFlags; UmountFlags; CloseRangeFlags
);
impl_from_to_sys_newtype!(
    Whence; FcntlCmd; IoctlCmd; FutexOp; ClockId; MaskHowto; SigNum; Domain; SocketType; Protocol;
    ShutdownHow; FlockOp; Madvice; RLimitable; RUsageWho; PrctlOp; SockOptLevel; SockOpt; DeviceNumber
);
impl<T> FromSyscall for *const T {
    fn from_syscall(value: usize) -> Self {
        value as _
    }
}
impl<T> ToSysret for *const T {
    fn to_sysret(self) -> usize {
        self as _
    }
}
impl<T> FromSyscall for *mut T {
    fn from_syscall(value: usize) -> Self {
        value as _
    }
}
impl<T> ToSysret for *mut T {
    fn to_sysret(self) -> usize {
        self as _
    }
}
impl FromSyscall for &CStr {
    fn from_syscall(value: usize) -> Self {
        unsafe { CStr::from_ptr(value as _) }
    }
}
impl FromSyscall for Option<&CStr> {
    fn from_syscall(value: usize) -> Self {
        match value {
            0 => None,
            other => unsafe { Some(CStr::from_ptr(other as _)) },
        }
    }
}
impl<T> FromSyscall for Option<NonNull<T>> {
    fn from_syscall(value: usize) -> Self {
        NonNull::new(value as *mut _)
    }
}
impl<T: ToSysret> ToSysret for Result<T, LxError> {
    fn to_sysret(self) -> usize {
        match self {
            Ok(x) => x.to_sysret(),
            Err(err) => (-(err.0 as i32)) as usize,
        }
    }
}
impl<T: ToSysret> ToSysret for std::io::Result<T> {
    fn to_sysret(self) -> usize {
        match self {
            Ok(x) => x.to_sysret(),
            Err(err) => {
                (-(err
                    .raw_os_error()
                    .map(|x| LxError::from_apple(x).unwrap_or(LxError::EIO))
                    .unwrap_or(LxError::EIO)
                    .0 as i32)) as usize
            }
        }
    }
}
impl ToSysret for () {
    fn to_sysret(self) -> usize {
        0
    }
}
