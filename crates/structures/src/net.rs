use crate::{error::LxError, unixvariants, FromApple, ToApple};
use bitflags::bitflags;
use libc::{c_char, c_int};

unixvariants! {
    pub struct Domain: u32 {
        const PF_LOCAL = 1;
        const PF_INET = 2;
        const PF_INET6 = 10;
        fn from_apple(apple: c_int) -> Result<Self, LxError>;
        fn to_apple(self) -> Result<c_int, LxError>;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SocketType(pub u32);
impl SocketType {
    pub fn kind(self) -> SocketKind {
        SocketKind(self.0 & 255)
    }

    pub fn flags(self) -> SocketFlags {
        SocketFlags::from_bits_retain(self.0 & !255)
    }
}

unixvariants! {
    pub struct SocketKind: u32 {
        const SOCK_STREAM = 1;
        const SOCK_DGRAM = 2;
        const SOCK_RAW = 3;
        fn from_apple(apple: c_int) -> Result<Self, LxError>;
        fn to_apple(self) -> Result<c_int, LxError>;
    }
}

unixvariants! {
    pub struct Protocol: u32 {
        const IPPROTO_IP = 0;
        const IPPROTO_ICMP = 1;
        const IPPROTO_IGMP = 2;
        const IPPROTO_TCP = 6;
        const IPPROTO_UDP = 17;
        const IPPROTO_IPV6 = 41;
        const IPPROTO_ICMPV6 = 58;
        fn from_apple(apple: c_int) -> Result<Self, LxError>;
        fn to_apple(self) -> Result<c_int, LxError>;
    }
}

unixvariants! {
    pub struct SockOpt: u32 {
        const SO_DEBUG = 1;
        const SO_REUSEADDR = 2;
        const SO_TYPE = 3;
        const SO_ERROR = 4;
        const SO_DONTROUTE = 5;
        const SO_BROADCAST = 6;
        const SO_SNDBUF = 7;
        const SO_RCVBUF = 8;
        const SO_KEEPALIVE = 9;
        const SO_OOBINLINE = 10;
        const SO_LINGER = 13;
        const SO_REUSEPORT = 15;
        const SO_RCVLOWAT = 18;
        const SO_SNDLOWAT = 19;
        const SO_RCVTIMEO = 20;
        const SO_SNDTIMEO = 21;
        const SO_TIMESTAMP = 29;
        const SO_ACCEPTCONN = 30;
        #[linux_only] const SO_NO_CHECK = 11;
        #[linux_only] const SO_PRIORITY = 12;
        #[linux_only] const SO_PASSCRED = 16;
        #[linux_only] const SO_PEERSEC = 31;
        #[linux_only] const SO_SNDBUFFORCE = 32;
        #[linux_only] const SO_RCVBUFFORCE = 33;
        #[linux_only] const SO_PROTOCOL = 38;
        #[linux_only] const SO_DOMAIN = 39;
        #[apple = LOCAL_PEERCRED] const SO_PEERCRED = 17;
        #[apple = IP_BOUND_IF] const SO_BINDTODEVICE = 25;
        fn from_apple(apple: c_int) -> Result<Self, LxError>;
        fn to_apple(self) -> Result<c_int, LxError>;
    }
}

unixvariants! {
    pub struct SockOptLevel: u32 {
        const SOL_SOCKET = 1;
        #[apple = IPPROTO_IP] const SOL_IP = 0;
        #[apple = IPPROTO_TCP] const SOL_TCP = 6;
        #[apple = IPPROTO_UDP] const SOL_UDP = 17;
        #[apple = IPPROTO_IPV6] const SOL_IPV6 = 41;
        fn from_apple(apple: c_int) -> Result<Self, LxError>;
        fn to_apple(self) -> Result<c_int, LxError>;
    }
}

unixvariants! {
    pub struct ShutdownHow: u32 {
        const SHUT_RD = 0;
        const SHUT_WR = 1;
        const SHUT_RDWR = 2;
        fn from_apple(apple: c_int) -> Result<Self, LxError>;
        fn to_apple(self) -> Result<c_int, LxError>;
    }
}

bitflags! {
    pub struct SocketFlags: u32 {
        const SOCK_NONBLOCK = 0o4000;
        const SOCK_CLOEXEC = 0o2000000;
    }
}

#[derive(Debug, Clone)]
pub enum SockAddr {
    Un(SockAddrUn, usize),
    In(SockAddrIn),
}
impl SockAddr {
    pub fn from_bytes(buf: &[u8]) -> Result<Self, LxError> {
        unsafe {
            let domain = buf.as_ptr().cast::<SaFamily>().read().to_domain();
            match domain {
                Domain::PF_LOCAL => SockAddrUn::from_bytes(buf).map(|un| Self::Un(un, buf.len())),
                Domain::PF_INET => SockAddrIn::from_bytes(buf).map(Self::In),
                _ => Err(LxError::EAFNOSUPPORT),
            }
        }
    }

    pub fn write_to(&self, buf: &mut [u8]) -> Result<usize, LxError> {
        match self {
            Self::Un(addr, len) => addr.write_to(buf, *len),
            Self::In(addr) => addr.write_to(buf),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SaFamily(pub u16);
impl SaFamily {
    pub fn to_domain(self) -> Domain {
        Domain(self.0 as _)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SockAddrUn {
    pub sun_family: SaFamily,
    pub sun_path: [c_char; 108],
}
impl SockAddrUn {
    pub fn from_bytes(buf: &[u8]) -> Result<Self, LxError> {
        if buf.len() < size_of::<Self>() {
            return Err(LxError::ENOMEM);
        }
        unsafe { Ok(buf.as_ptr().cast::<Self>().read()) }
    }

    pub fn write_to(&self, buf: &mut [u8], size: usize) -> Result<usize, LxError> {
        if buf.len() < size {
            return Err(LxError::ENOMEM);
        }
        unsafe {
            (buf as *mut [u8])
                .cast::<u8>()
                .copy_from_nonoverlapping(self as *const _ as *const u8, size);
        }
        Ok(size_of::<Self>())
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SockAddrIn {
    pub sin_family: SaFamily,
    pub sin_port: u16,
    pub sin_addr: InAddr,
    pub sin_zero: [u8; 8],
}
impl SockAddrIn {
    pub fn from_bytes(buf: &[u8]) -> Result<Self, LxError> {
        if buf.len() < size_of::<Self>() {
            return Err(LxError::ENOMEM);
        }
        unsafe { Ok(buf.as_ptr().cast::<Self>().read()) }
    }

    pub fn write_to(&self, buf: &mut [u8]) -> Result<usize, LxError> {
        if buf.len() < size_of::<Self>() {
            return Err(LxError::ENOMEM);
        }
        unsafe {
            (buf as *mut [u8]).cast::<Self>().write(*self);
        }
        Ok(size_of::<Self>())
    }

    pub fn from_apple(buf: &[u8]) -> Result<Self, LxError> {
        if buf.len() < size_of::<libc::sockaddr_in>() {
            return Err(LxError::ENOMEM);
        }

        unsafe {
            let apple = (buf as *const [u8]).cast::<libc::sockaddr_in>();
            Ok(Self {
                sin_family: SaFamily(Domain::PF_INET.0 as _),
                sin_port: (*apple).sin_port,
                sin_addr: (*apple).sin_addr.into(),
                sin_zero: [0; _],
            })
        }
    }

    pub fn to_apple(&self, buf: &mut [u8]) -> Result<(), LxError> {
        if buf.len() < size_of::<libc::sockaddr_in>() {
            return Err(LxError::ENOMEM);
        }

        unsafe {
            buf.as_mut_ptr()
                .cast::<libc::sockaddr_in>()
                .write(libc::sockaddr_in {
                    sin_len: size_of::<libc::sockaddr_in>() as _,
                    sin_family: libc::AF_INET as _,
                    sin_port: self.sin_port,
                    sin_addr: libc::in_addr {
                        s_addr: self.sin_addr.0,
                    },
                    sin_zero: [0; _],
                });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct InAddr(u32);
impl From<libc::in_addr> for InAddr {
    fn from(value: libc::in_addr) -> Self {
        Self(value.s_addr)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Linger {
    pub l_onoff: c_int,
    pub l_linger: c_int,
}
impl FromApple for Linger {
    type Apple = libc::linger;

    fn from_apple(apple: libc::linger) -> Result<Self, LxError> {
        Ok(Self {
            l_onoff: apple.l_onoff,
            l_linger: apple.l_linger,
        })
    }
}
impl ToApple for Linger {
    type Apple = libc::linger;

    fn to_apple(self) -> Result<libc::linger, LxError> {
        Ok(libc::linger {
            l_onoff: self.l_onoff,
            l_linger: self.l_linger,
        })
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct MsgHdr {
    pub msg_name: *mut u8,
    pub msg_namelen: u32,
    pub msg_iov: *mut libc::iovec,
    pub msg_iovlen: c_int,
    pub _pad1: c_int,
    pub msg_control: *mut u8,
    pub msg_controllen: u32,
    pub _pad2: c_int,
    pub msg_flags: c_int,
}
impl MsgHdr {
    pub unsafe fn name(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.msg_name, self.msg_namelen as _) }
    }

    pub unsafe fn iov(&self) -> &[libc::iovec] {
        unsafe { std::slice::from_raw_parts(self.msg_iov, self.msg_iovlen as _) }
    }

    pub unsafe fn control(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.msg_control, self.msg_controllen as _) }
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct MmsgHdr {
    pub msg_hdr: MsgHdr,
    pub msg_len: u32,
}
