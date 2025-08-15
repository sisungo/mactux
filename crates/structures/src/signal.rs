use crate::{time::ClockId, unixvariants};
use bitflags::bitflags;
use libc::{c_int, c_long, c_short, c_uint};

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

unixvariants! {
    pub struct SigNum: u32 {
        const SIGHUP = 1;
        const SIGINT = 2;
        const SIGQUIT = 3;
        const SIGILL = 4;
        const SIGTRAP = 5;
        const SIGABRT = 6;
        const SIGBUS = 7;
        const SIGFPE = 8;
        const SIGKILL = 9;
        const SIGUSR1 = 10;
        const SIGSEGV = 11;
        const SIGUSR2 = 12;
        const SIGPIPE = 13;
        const SIGALRM = 14;
        const SIGTERM = 15;
        const SIGCHLD = 17;
        const SIGCONT = 18;
        const SIGSTOP = 19;
        const SIGTSTP = 20;
        const SIGTTIN = 21;
        const SIGTTOU = 22;
        const SIGURG = 23;
        const SIGXCPU = 24;
        const SIGXFSZ = 25;
        const SIGVTALRM = 26;
        const SIGPROF = 27;
        const SIGWINCH = 28;
        const SIGSYS = 31;
        #[linux_only] const SIGSTKFLT = 16;
        #[linux_only] const SIGPWR = 30;
        #[linux_only] const SIGRTMIN = 32;
        #[linux_only] const SIGRTMAX = Self::_NSIG;
        #[apple = SIGIO] const SIGPOLL = 29;
        fn from_apple(apple: c_int) -> Result<Self, LxError>;
        fn to_apple(self) -> Result<c_int, LxError>;
    }
}
impl SigNum {
    pub const _NSIG: u32 = 64;
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

unixvariants! {
    pub struct MaskHowto: u32 {
        const SIG_BLOCK = 0;
        const SIG_UNBLOCK = 1;
        const SIG_SETMASK = 2;
        fn from_apple(apple: c_int) -> Result<Self, LxError>;
        fn to_apple(self) -> Result<c_int, LxError>;
    }
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
