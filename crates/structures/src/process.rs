use crate::{
    bitflags_impl_to_apple, error::LxError, signal::SigNum, time::Timeval, unixvariants,
};
use bitflags::bitflags;
use std::ffi::c_int;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct WaitStatus(pub c_int);
impl WaitStatus {
    pub fn from_apple(apple: c_int) -> Self {
        if libc::WTERMSIG(apple) != 0 {
            let signum = SigNum::from_apple(libc::WTERMSIG(apple)).unwrap_or(SigNum::SIGUNUSED);
            Self(signum.0 as _)
        } else {
            Self(libc::WEXITSTATUS(apple) << 8)
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct CloneFlags: u32 {
        const CLONE_VM = 0x100;
        const CLONE_FS = 0x200;
        const CLONE_FILES = 0x400;
        const CLONE_SIGHAND = 0x800;
        const CLONE_PARENT = 0x8000;
        const CLONE_THREAD = 0x10000;
        const CLONE_SETTLS = 0x800000;
        const CLONE_PARENT_SETTID = 0x100000;
        const CLONE_CHILD_CLEARTID = 0x200000;
        const CLONE_CHILD_SETTID = 0x1000000;
        const CLONE_IO = 0x80000000;
    }
}
impl CloneFlags {
    pub fn child_type(self) -> ChildType {
        let thread_mask = Self::CLONE_VM
            | Self::CLONE_FS
            | Self::CLONE_FILES
            | Self::CLONE_SIGHAND
            | Self::CLONE_THREAD;

        if self.contains(thread_mask) {
            ChildType::Thread
        } else if self.intersects(thread_mask) {
            ChildType::Unsupported
        } else {
            ChildType::Process
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChildType {
    Process,
    Thread,
    Unsupported,
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct WaitOptions: u32 {
        const WNOHANG = 1;
    }
}
impl WaitOptions {
    pub fn to_apple(self) -> c_int {
        bitflags_impl_to_apple!(self = WNOHANG)
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct CloneArgs {
    pub flags: u64,
    pub pidfd: u64,
    pub child_tid: u64,
    pub parent_tid: u64,
    pub exit_signal: u64,
    pub stack: u64,
    pub stack_size: u64,
    pub tls: u64,
    pub set_tid: u64,
    pub set_tid_size: u64,
    pub cgroup: u64,
}
impl CloneArgs {
    pub unsafe fn from_ptr_size(ptr: *const u8, size: usize) -> Result<Self, LxError> {
        unsafe {
            let mut result: Self = std::mem::zeroed();
            if ![64, 80, 88].contains(&size) {
                return Err(LxError::EINVAL);
            }
            (&raw mut result)
                .cast::<u8>()
                .copy_from_nonoverlapping(ptr, size);
            Ok(result)
        }
    }

    pub fn flags(&self) -> CloneFlags {
        CloneFlags::from_bits_retain(self.flags as _)
    }

    pub fn pidfd(&self) -> *mut c_int {
        self.pidfd as usize as *mut c_int
    }

    pub fn child_tid(&self) -> *mut i32 {
        self.child_tid as usize as *mut i32
    }

    pub fn parent_tid(&self) -> *mut i32 {
        self.parent_tid as usize as *mut i32
    }

    pub fn exit_signal(&self) -> SigNum {
        SigNum(self.exit_signal as _)
    }

    pub unsafe fn stack(&self) -> *mut u8 {
        unsafe { (self.stack as usize as *mut u8).add(self.stack_size as usize) }
    }

    pub fn tls(&self) -> *mut u8 {
        self.tls as usize as *mut _
    }

    pub unsafe fn set_tid(&self) -> &[i32] {
        unsafe {
            std::slice::from_raw_parts(
                self.set_tid as usize as *const i32,
                self.set_tid_size as usize,
            )
        }
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct RUsage {
    pub ru_utime: Timeval,
    pub ru_stime: Timeval,
    pub ru_maxrss: i64,
    pub ru_ixrss: i64,
    pub ru_idrss: i64,
    pub ru_isrss: i64,
    pub ru_minflt: i64,
    pub ru_majflt: i64,
    pub ru_nswap: i64,
    pub ru_inblock: i64,
    pub ru_oublock: i64,
    pub ru_msgsnd: i64,
    pub ru_msgrcv: i64,
    pub ru_nsignals: i64,
    pub ru_nvcsw: i64,
    pub ru_nivcsw: i64,
    pub __reserved: [i64; 16],
}
impl RUsage {
    pub fn from_apple(apple: libc::rusage) -> Self {
        Self {
            ru_utime: Timeval::from_apple(apple.ru_utime),
            ru_stime: Timeval::from_apple(apple.ru_stime),
            ru_maxrss: apple.ru_maxrss,
            ru_ixrss: apple.ru_ixrss,
            ru_idrss: apple.ru_idrss,
            ru_isrss: apple.ru_isrss,
            ru_minflt: apple.ru_minflt,
            ru_majflt: apple.ru_majflt,
            ru_nswap: apple.ru_nswap,
            ru_inblock: apple.ru_inblock,
            ru_oublock: apple.ru_oublock,
            ru_msgsnd: apple.ru_msgsnd,
            ru_msgrcv: apple.ru_msgrcv,
            ru_nsignals: apple.ru_nsignals,
            ru_nvcsw: apple.ru_nvcsw,
            ru_nivcsw: apple.ru_nivcsw,
            __reserved: [0; _],
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RLimit64 {
    pub rlim_cur: u64,
    pub rlim_max: u64,
}
impl RLimit64 {
    pub const RLIM_INFINITY: u64 = !0;

    pub fn from_apple(apple: libc::rlimit) -> Self {
        let map_value = |apple| match apple {
            libc::RLIM_INFINITY => Self::RLIM_INFINITY,
            other => other,
        };
        Self {
            rlim_cur: map_value(apple.rlim_cur),
            rlim_max: map_value(apple.rlim_max),
        }
    }

    pub fn to_apple(self) -> libc::rlimit {
        let map_value = |linux| match linux {
            Self::RLIM_INFINITY => libc::RLIM_INFINITY,
            other => other,
        };
        libc::rlimit {
            rlim_cur: map_value(self.rlim_cur),
            rlim_max: map_value(self.rlim_max),
        }
    }
}

unixvariants! {
    pub struct RLimitable: u32 {
        const RLIMIT_CPU = 0;
        const RLIMIT_FSIZE = 1;
        const RLIMIT_DATA = 2;
        const RLIMIT_STACK = 3;
        const RLIMIT_CORE = 4;
        const RLIMIT_NOFILE = 7;
        const RLIMIT_MEMLOCK = 8;
        const RLIMIT_AS = 9;
        #[linux_only] const RLIMIT_LOCKS = 10;
        #[linux_only] const RLIMIT_SIGPENDING = 11;
        #[linux_only] const RLIMIT_MSGQUEUE = 12;
        #[linux_only] const RLIMIT_NICE = 13;
        #[linux_only] const RLIMIT_RTPRIO = 14;
        #[linux_only] const RLIMIT_RTTIME = 15;
        #[linux_only] const RLIMIT_RSS = 5;
        #[linux_only] const RLIMIT_NRPOC = 6;
        fn from_apple(apple: c_int) -> Result<Self, LxError>;
        fn to_apple(self) -> Result<c_int, LxError>;
    }
}

unixvariants! {
    pub struct RUsageWho: u32 {
        const RUSAGE_SELF = 0;
        const RUSAGE_CHILDREN = u32::MAX;
        #[apple = RUSAGE_SELF] const RUSAGE_THREAD = 1;
        fn from_apple(apple: c_int) -> Result<Self, LxError>;
        fn to_apple(self) -> Result<c_int, LxError>;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct PrctlOp(pub u32);
