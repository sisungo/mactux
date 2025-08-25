use crate::signal::{KernelSigSet, SigAltStack};
use bitflags::bitflags;

#[derive(Debug, Clone)]
#[repr(C)]
pub struct UContext {
    pub uc_flags: UContextFlags,
    pub uc_link: *mut UContext,
    pub uc_stack: SigAltStack,
    pub uc_mcontext: MContext,
    pub uc_sigmask: KernelSigSet,
}
impl UContext {
    pub unsafe fn from_apple(apple: &libc::ucontext_t) -> Self {
        unsafe {
            Self {
                uc_flags: UContextFlags::empty(),
                uc_link: std::ptr::null_mut(),
                uc_stack: SigAltStack::from_apple(apple.uc_stack),
                uc_mcontext: MContext::from_apple(&*apple.uc_mcontext),
                uc_sigmask: KernelSigSet::from_apple(apple.uc_sigmask),
            }
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct UContextFlags: u64 {}
}

#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone)]
#[repr(C)]
pub struct MContext {
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub rdx: u64,
    pub rax: u64,
    pub rcx: u64,
    pub rsp: u64,
    pub rip: u64,
    pub rflags: u64,
    pub cs: u16,
    pub gs: u16,
    pub fs: u16,
    pub ss: u16,
    pub err: u64,
    pub trapno: u64,
    pub oldmask: u64,
    pub cr2: u64,
    pub fpstate: u64,
    pub reserved: [u64; 8],
}
#[cfg(target_arch = "x86_64")]
impl MContext {
    pub fn from_apple(apple: &libc::__darwin_mcontext64) -> Self {
        Self {
            r8: apple.__ss.__r8,
            r9: apple.__ss.__r9,
            r10: apple.__ss.__r10,
            r11: apple.__ss.__r11,
            r12: apple.__ss.__r12,
            r13: apple.__ss.__r13,
            r14: apple.__ss.__r14,
            r15: apple.__ss.__r15,
            rdi: apple.__ss.__rdi,
            rsi: apple.__ss.__rsi,
            rbp: apple.__ss.__rbp,
            rbx: apple.__ss.__rbx,
            rdx: apple.__ss.__rdx,
            rax: apple.__ss.__rax,
            rcx: apple.__ss.__rcx,
            rsp: apple.__ss.__rsp,
            rip: apple.__ss.__rip,
            rflags: apple.__ss.__rflags,
            cs: apple.__ss.__cs as _,
            gs: apple.__ss.__gs as _,
            fs: apple.__ss.__fs as _,
            ss: 0,
            err: apple.__es.__err as _,
            trapno: apple.__es.__trapno as _,
            fpstate: 0,
            oldmask: 0,
            cr2: 0,
            reserved: [0; _],
        }
    }

    pub fn write_to_apple(&self, apple: &mut libc::__darwin_mcontext64) {
        apple.__ss.__r8 = self.r8;
        apple.__ss.__r9 = self.r9;
        apple.__ss.__r10 = self.r10;
        apple.__ss.__r11 = self.r11;
        apple.__ss.__r12 = self.r12;
        apple.__ss.__r13 = self.r13;
        apple.__ss.__r14 = self.r14;
        apple.__ss.__r15 = self.r15;
        apple.__ss.__rdi = self.rdi;
        apple.__ss.__rsi = self.rsi;
        apple.__ss.__rbp = self.rbp;
        apple.__ss.__rbx = self.rbx;
        apple.__ss.__rdx = self.rdx;
        apple.__ss.__rax = self.rax;
        apple.__ss.__rcx = self.rcx;
        apple.__ss.__rsp = self.rsp;
        apple.__ss.__rip = self.rip;
        apple.__ss.__rflags = self.rflags;
        apple.__ss.__cs = self.cs as _;
        apple.__ss.__gs = self.gs as _;
        apple.__ss.__fs = self.fs as _;
    }
}

#[cfg(target_arch = "aarch64")]
#[derive(Debug, Clone)]
#[repr(C)]
pub struct MContext {}
#[cfg(target_arch = "aarch64")]
impl MContext {
    pub fn from_apple(apple: &libc::__darwin_mcontext64) -> Self {
        Self {}
    }
}
