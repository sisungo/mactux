use crate::{error::LxError, time::ClockId};
use bitflags::bitflags;
use libc::{c_int, c_long, c_short, c_uint};

macro_rules! impl_from_to_apple {
    ($($x:ident),*) => {
        pub fn to_apple(self) -> Result<libc::c_int, LxError> {
            crate::newtype_impl_to_apple!(self = $($x),*).ok_or(LxError::EINVAL)
        }

        pub fn from_apple(apple: libc::c_int) -> Result<Self, LxError> {
            crate::newtype_impl_from_apple!(apple = $($x),*).ok_or(LxError::EINVAL)
        }
    };
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SigAction {
    pub handler: SigHandler,
    pub flags: SigActionFlags,
    pub restorer: usize,
    pub mask: KernelSigSet,
}
impl SigAction {
    pub const fn new() -> Self {
        Self {
            handler: SigHandler::SIG_DFL,
            flags: SigActionFlags::empty(),
            restorer: 0,
            mask: KernelSigSet::empty(),
        }
    }
}
impl Default for SigAction {
    fn default() -> Self {
        Self::new()
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct SigActionFlags: u64 {
        const SA_NOCLDSTOP = 0x1;
        const SA_NOCLDWAIT = 0x2;
        const SA_SIGINFO = 0x4;
        const SA_UNSUPPORTED = 0x400;
        const SA_EXPOSE_TAGBITS = 0x800;
        const SA_RESTORER = 0x4000000;
        const SA_ONSTACK = 0x8000000;
        const SA_RESTART = 0x10000000;
        const SA_NODEFER = 0x40000000;
        const SA_RESETHAND = 0x80000000;
    }
}
impl SigActionFlags {
    pub fn to_apple(self) -> c_int {
        let mut apple = 0;
        if self.contains(Self::SA_NOCLDSTOP) {
            apple |= libc::SA_NOCLDSTOP;
        }
        if self.contains(Self::SA_NOCLDWAIT) {
            apple |= libc::SA_NOCLDWAIT;
        }
        apple |= libc::SA_SIGINFO;
        if self.contains(Self::SA_ONSTACK) {
            apple |= libc::SA_ONSTACK;
        }
        if self.contains(Self::SA_RESTART) {
            apple |= libc::SA_RESTART;
        }
        if self.contains(Self::SA_NODEFER) {
            apple |= libc::SA_NODEFER;
        }
        if self.contains(Self::SA_RESETHAND) {
            apple |= libc::SA_RESETHAND;
        }
        apple
    }
}

#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct SigInfo {
    pub si_signo: c_int,
    pub si_errno: c_int,
    pub si_code: c_int,
    pub si_trapno: c_int,
    pub si_pid: i32,
    pub si_uid: u32,
    pub si_status: c_int,
    pub si_utime: ClockId,
    pub si_value: usize,
    pub si_int: c_int,
    pub si_ptr: *mut u8,
    pub si_overrun: c_int,
    pub si_timerid: c_int,
    pub si_addr: *mut u8,
    pub si_band: c_long,
    pub si_fd: c_int,
    pub si_addr_lsb: c_short,
    pub si_lower: *mut u8,
    pub si_upper: *mut u8,
    pub si_pkey: c_int,
    pub si_call_addr: *mut u8,
    pub si_syscall: c_int,
    pub si_arch: c_uint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SigNum(pub u32);
impl SigNum {
    pub const SIGHUP: Self = Self(1);
    pub const SIGINT: Self = Self(2);
    pub const SIGQUIT: Self = Self(3);
    pub const SIGILL: Self = Self(4);
    pub const SIGTRAP: Self = Self(5);
    pub const SIGABRT: Self = Self(6);
    pub const SIGIOT: Self = Self::SIGABRT;
    pub const SIGBUS: Self = Self(7);
    pub const SIGFPE: Self = Self(8);
    pub const SIGKILL: Self = Self(9);
    pub const SIGUSR1: Self = Self(10);
    pub const SIGSEGV: Self = Self(11);
    pub const SIGUSR2: Self = Self(12);
    pub const SIGPIPE: Self = Self(13);
    pub const SIGALRM: Self = Self(14);
    pub const SIGTERM: Self = Self(15);
    pub const SIGSTKFLT: Self = Self(16);
    pub const SIGCHLD: Self = Self(17);
    pub const SIGCONT: Self = Self(18);
    pub const SIGSTOP: Self = Self(19);
    pub const SIGTSTP: Self = Self(20);
    pub const SIGTTIN: Self = Self(21);
    pub const SIGTTOU: Self = Self(22);
    pub const SIGURG: Self = Self(23);
    pub const SIGXCPU: Self = Self(24);
    pub const SIGXFSZ: Self = Self(25);
    pub const SIGVTALRM: Self = Self(26);
    pub const SIGPROF: Self = Self(27);
    pub const SIGWINCH: Self = Self(28);
    pub const SIGIO: Self = Self(29);
    pub const SIGPOLL: Self = Self(29);
    pub const SIGPWR: Self = Self(30);
    pub const SIGSYS: Self = Self(31);
    pub const SIGUNUSED: Self = Self::SIGSYS;

    pub const SIGRTMIN: Self = Self(32);
    pub const SIGRTMAX: Self = Self(Self::_NSIG);

    pub const _NSIG: u32 = 64;

    impl_from_to_apple!(
        SIGHUP, SIGINT, SIGQUIT, SIGILL, SIGTRAP, SIGABRT, SIGIOT, SIGBUS, SIGFPE, SIGKILL,
        SIGUSR1, SIGSEGV, SIGUSR2, SIGPIPE, SIGALRM, SIGTERM, SIGCHLD, SIGCONT, SIGSTOP, SIGTSTP,
        SIGTTIN, SIGTTOU, SIGXCPU, SIGVTALRM, SIGPROF, SIGWINCH, SIGIO, SIGSYS
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct SigHandler(pub usize);
impl SigHandler {
    pub const SIG_DFL: Self = Self(0);
    pub const SIG_IGN: Self = Self(1);
    pub const SIG_HOLD: Self = Self(2);
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct KernelSigSet(u64);
impl KernelSigSet {
    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn add(&mut self, sig: SigNum) {
        let sig = sig.0 - 1;
        self.0 |= 1 << sig;
    }

    pub const fn del(&mut self, sig: SigNum) {
        let sig = sig.0 - 1;
        self.0 &= !(1 << sig);
    }

    pub const fn get(&self, sig: SigNum) -> bool {
        let sig = sig.0 - 1;
        (self.0 & (1 << sig)) != 0
    }

    pub fn from_iter(iter: impl Iterator<Item = SigNum>) -> Self {
        let mut obj = Self::empty();
        for signum in iter {
            obj.add(signum);
        }
        obj
    }

    pub const fn iter(self) -> KernelSigSetIter {
        KernelSigSetIter {
            sigset: self,
            pos: 0,
        }
    }

    pub fn from_apple(sigset: libc::sigset_t) -> Self {
        let mut obj = Self::empty();
        unsafe {
            for n in 1..32 {
                if libc::sigismember(&sigset, n) != 0 {
                    obj.add(SigNum(n as _));
                }
            }
        }
        obj
    }

    pub fn to_apple(self) -> libc::sigset_t {
        let mut apple = 0;
        unsafe {
            libc::sigemptyset(&mut apple);
            for signum in self.iter() {
                let Ok(apple_signum) = signum.to_apple() else {
                    continue;
                };
                if [libc::SIGSEGV, libc::SIGSYS].contains(&apple_signum) {
                    continue;
                }
                libc::sigaddset(&mut apple, apple_signum);
            }
        }
        apple
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct MaskHowto(pub u32);
impl MaskHowto {
    pub const SIG_BLOCK: Self = Self(0);
    pub const SIG_UNBLOCK: Self = Self(1);
    pub const SIG_SETMASK: Self = Self(2);

    impl_from_to_apple!(SIG_BLOCK, SIG_SETMASK, SIG_UNBLOCK);
}

#[derive(Debug, Clone)]
pub struct KernelSigSetIter {
    sigset: KernelSigSet,
    pos: u32,
}
impl Iterator for KernelSigSetIter {
    type Item = SigNum;

    fn next(&mut self) -> Option<Self::Item> {
        while self.pos <= SigNum::_NSIG - 1 {
            if self.sigset.get(SigNum(self.pos + 1)) {
                self.pos += 1;
                return Some(SigNum(self.pos));
            }
            self.pos += 1;
        }
        None
    }
}
