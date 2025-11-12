use crate::{
    FromApple, ToApple, error::LxError, impl_bincode_for_bitflags, mapper, signal::KernelSigSet,
    terminal::Termios2, unixvariants,
};
use bincode::{Decode, Encode};
use bitflags::bitflags;
use libc::c_int;
use std::ptr::NonNull;

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct FcntlCmd(pub u32);
impl FcntlCmd {
    pub const F_DUPFD: Self = Self(0);
    pub const F_GETFD: Self = Self(1);
    pub const F_SETFD: Self = Self(2);
    pub const F_GETFL: Self = Self(3);
    pub const F_SETFL: Self = Self(4);
    pub const F_GETLK: Self = Self(5);
    pub const F_SETLK: Self = Self(6);
    pub const F_SETLKW: Self = Self(7);
    pub const F_DUPFD_CLOEXEC: Self = Self(1030);

    pub const fn ctrl_query(self) -> VfdAvailCtrl {
        VfdAvailCtrl {
            in_size: self.in_size(),
            out_size: self.out_size(),
        }
    }

    const fn in_size(self) -> isize {
        match self {
            Self::F_DUPFD => -1,
            Self::F_GETFD => -1,
            Self::F_SETFD => -1,
            Self::F_GETFL => -1,
            Self::F_SETFL => -1,
            Self::F_GETLK => -1,
            Self::F_SETLK => size_of::<Flock>() as isize,
            Self::F_SETLKW => size_of::<Flock>() as isize,
            Self::F_DUPFD_CLOEXEC => -1,
            _ => -1,
        }
    }

    const fn out_size(self) -> usize {
        match self {
            Self::F_DUPFD => 0,
            Self::F_GETFD => 0,
            Self::F_SETFD => 0,
            Self::F_GETFL => 0,
            Self::F_SETFL => 0,
            Self::F_GETLK => size_of::<Flock>(),
            Self::F_SETLK => 0,
            Self::F_SETLKW => 0,
            Self::F_DUPFD_CLOEXEC => 0,
            _ => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct IoctlCmd(pub u32);
impl IoctlCmd {
    pub const TCGETS: Self = Self(0x5401);
    pub const TCSETS: Self = Self(0x5402);
    pub const TCSETSW: Self = Self(0x5403);
    pub const TCSETSF: Self = Self(0x5404);
    pub const TCXONC: Self = Self(0x540A);
    pub const TIOCGPGRP: Self = Self(0x540F);
    pub const TIOCSPGRP: Self = Self(0x5410);
    pub const TIOCGWINSZ: Self = Self(0x5413);
    pub const TIOCSWINSZ: Self = Self(0x5414);

    pub const TCGETS2: Self = Self::_ior::<Termios2>(b'T' as _, 42);
    pub const TCSETS2: Self = Self::_iow::<Termios2>(b'T' as _, 43);
    pub const TCSETSW2: Self = Self::_iow::<Termios2>(b'T' as _, 44);
    pub const TCSETSF2: Self = Self::_iow::<Termios2>(b'T' as _, 45);

    pub const SNDCTL_DSP_CHANNELS: Self = Self::_iowr::<c_int>(b'P' as _, 6);
    pub const SNDCTL_DSP_SPEED: Self = Self::_iowr::<c_int>(b'P' as _, 2);
    pub const SNDCTL_DSP_SETFMT: Self = Self::_iowr::<c_int>(b'P' as _, 5);
    pub const SNDCTL_DSP_SETFRAGMENT: Self = Self::_iowr::<c_int>(b'P' as _, 10);
    pub const SNDCTL_DSP_STEREO: Self = Self::_iowr::<c_int>(b'P' as _, 3);

    pub const _IOC_READ: u32 = 2;
    pub const _IOC_WRITE: u32 = 1;

    pub const fn _ioc(a: u32, b: u32, c: u32, d: u32) -> Self {
        Self(a << 30 | b << 8 | c | d << 16)
    }

    pub const fn _ior<T>(a: u32, b: u32) -> Self {
        Self::_ioc(Self::_IOC_READ, a, b, size_of::<T>() as u32)
    }

    pub const fn _iow<T>(a: u32, b: u32) -> Self {
        Self::_ioc(Self::_IOC_WRITE, a, b, size_of::<T>() as u32)
    }

    pub const fn _iowr<T>(a: u32, b: u32) -> Self {
        Self::_ioc(
            Self::_IOC_WRITE | Self::_IOC_READ,
            a,
            b,
            size_of::<T>() as u32,
        )
    }
}

unixvariants! {
    #[derive(Encode, Decode)]
    pub struct Whence: u32 {
        const SEEK_SET = 0;
        const SEEK_CUR = 1;
        const SEEK_END = 2;
        const SEEK_DATA = 3;
        const SEEK_HOLE = 4;
        fn from_apple(apple: c_int) -> Result<Self, LxError>;
        fn to_apple(self) -> Result<c_int, LxError>;
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Flock {
    pub l_type: FlockTy,
    pub l_whence: i16,
    pub l_start: i64,
    pub l_len: i64,
    pub l_pid: i32,
}
impl Flock {
    #[inline]
    pub fn to_apple(&self) -> Result<libc::flock, LxError> {
        Ok(libc::flock {
            l_start: self.l_start,
            l_len: self.l_len,
            l_pid: mapper::with_pid_mapper(|x| x.linux_to_apple(self.l_pid))?,
            l_type: self.l_type.to_apple()?,
            l_whence: Whence(self.l_whence as _).to_apple()? as _,
        })
    }

    #[inline]
    pub fn from_apple(apple: libc::flock) -> Result<Flock, LxError> {
        Ok(Self {
            l_start: apple.l_start,
            l_len: apple.l_len,
            l_pid: mapper::with_pid_mapper(|x| x.apple_to_linux(apple.l_pid))?,
            l_type: FlockTy::from_apple(apple.l_type)?,
            l_whence: Whence::from_apple(apple.l_whence as _)?.0 as _,
        })
    }
}

unixvariants! {
    pub struct FlockTy: u16 {
        const F_RDLCK = 0;
        const F_WRLCK = 1;
        const F_UNLCK = 2;
        fn from_apple(apple: i16) -> Result<Self, LxError>;
        fn to_apple(self) -> Result<i16, LxError>;
    }
}

unixvariants! {
    pub struct FlockOp: u32 {
        const LOCK_SH = 1;
        const LOCK_EX = 2;
        const LOCK_UN = 8;
        fn from_apple(apple: c_int) -> Result<Self, LxError>;
        fn to_apple(self) -> Result<c_int, LxError>;
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct PollFd {
    pub fd: c_int,
    pub events: PollEvents,
    pub revents: PollEvents,
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct PollEvents: u16 {
        const POLLIN = 1;
        const POLLPRI = 2;
        const POLLOUT = 4;
        const POLLERR = 8;
        const POLLHUP = 16;
        const POLLNVAL = 32;
        const POLLRDNORM = 64;
        const POLLRDBAND = 128;
        const POLLWRNORM = 256;
        const POLLWRBAND = 512;
        const POLLRDHUP = 0x2000;
    }
}
crate::bitflags_impl_from_to_apple!(
    PollEvents;
    type Apple = i16;
    values = POLLIN, POLLPRI, POLLOUT, POLLERR, POLLHUP, POLLNVAL, POLLRDNORM, POLLRDBAND, POLLWRNORM, POLLWRBAND
);

#[derive(Debug)]
pub struct FdSet {
    ptr: NonNull<u64>,
    nfd: usize,
}
impl FdSet {
    pub fn new(ptr: NonNull<u64>, nfd: usize) -> Self {
        Self { ptr, nfd }
    }

    pub unsafe fn contains(&self, fd: c_int) -> bool {
        if fd >= self.nfd as _ {
            return false;
        }

        unsafe {
            (*self.ptr.add(Self::element_major(fd)).as_ptr()) & (1 << Self::element_minor(fd)) != 0
        }
    }

    pub unsafe fn insert(&self, fd: c_int) {
        if fd >= self.nfd as _ {
            panic!("cannot overflow file descriptor set");
        }

        unsafe {
            *self.ptr.add(Self::element_major(fd)).as_ptr() |= 1 << Self::element_minor(fd);
        }
    }

    pub unsafe fn remove(&self, fd: c_int) {
        if fd >= self.nfd as _ {
            panic!("cannot overflow file descriptor set");
        }

        unsafe {
            *self.ptr.add(Self::element_major(fd)).as_ptr() &= !(1 << Self::element_minor(fd));
        }
    }

    pub unsafe fn clear(&self) {
        if self.nfd == 0 {
            return;
        }

        unsafe {
            self.ptr
                .write_bytes(0, self.nfd.div_ceil(u64::BITS as usize));
        }
    }

    pub unsafe fn iter(&self) -> FdSetIter<'_> {
        FdSetIter { set: self, pos: 0 }
    }

    fn element_major(fd: c_int) -> usize {
        fd as usize / (u64::BITS as usize)
    }

    fn element_minor(fd: c_int) -> usize {
        fd as usize % (u64::BITS as usize)
    }
}

#[derive(Debug)]
pub struct FdSetIter<'a> {
    set: &'a FdSet,
    pos: usize,
}
impl Iterator for FdSetIter<'_> {
    type Item = c_int;

    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.set.nfd {
            unsafe {
                if self.set.contains(self.pos as _) {
                    self.pos += 1;
                    return Some(self.pos as c_int - 1);
                }
            }
            self.pos += 1;
        }
        None
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct FdFlags: u32 {
        const FD_CLOEXEC = 1;
    }
}
crate::bitflags_impl_from_to_apple!(FdFlags; type Apple = c_int; values = FD_CLOEXEC);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PSelectSigMask {
    sigset: KernelSigSet,
    size: usize,
}
impl PSelectSigMask {
    pub fn into_sigset(self) -> Result<KernelSigSet, LxError> {
        if self.size != size_of::<KernelSigSet>() {
            return Err(LxError::EINVAL);
        }
        Ok(self.sigset)
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct EventFdFlags: u32 {
        const EFD_SEMAPHORE = 1;
        const EFD_NONBLOCK = 0o4000;
        const EFD_CLOEXEC = 0o2000000;
    }
}
impl EventFdFlags {
    pub fn open_flags(self) -> crate::fs::OpenFlags {
        let mut result = crate::fs::OpenFlags::O_RDWR;
        if self.contains(Self::EFD_NONBLOCK) {
            result |= crate::fs::OpenFlags::O_NONBLOCK;
        }
        if self.contains(Self::EFD_CLOEXEC) {
            result |= crate::fs::OpenFlags::O_CLOEXEC;
        }
        result
    }
}
impl_bincode_for_bitflags!(EventFdFlags: u32);

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct CloseRangeFlags: u32 {
        const CLOSE_RANGE_UNSHARE = 2;
        const CLOSE_RANGE_CLOEXEC = 4;
    }
}

/// Information about a virtual file descriptor's specific "ioctl" availability.
#[derive(Debug, Clone, Encode, Decode)]
pub struct VfdAvailCtrl {
    pub in_size: isize,
    pub out_size: usize,
}
