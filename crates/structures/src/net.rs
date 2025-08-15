use crate::{error::LxError, unixvariants};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SockOpt(pub u32);
impl SockOpt {
    pub const SO_DEBUG: Self = Self(1);
    pub const SO_REUSEDADDR: Self = Self(2);
    pub const SO_TYPE: Self = Self(3);
    pub const SO_ERROR: Self = Self(4);
    pub const SO_DONTROUTE: Self = Self(5);
    pub const SO_BROADCAST: Self = Self(6);
    pub const SO_SNDBUF: Self = Self(7);
    pub const SO_RCVBUF: Self = Self(8);
    pub const SO_KEEPALIVE: Self = Self(9);
    pub const SO_OOBINLINE: Self = Self(10);
    pub const SO_NO_CHECK: Self = Self(11);
    pub const SO_PRIORITY: Self = Self(12);
    pub const SO_LINGER: Self = Self(13);
    pub const SO_REUSEPORT: Self = Self(15);
    pub const SO_PASSCRED: Self = Self(16);
    pub const SO_PEERCRED: Self = Self(17);
    pub const SO_RCVLOWAT: Self = Self(18);
    pub const SO_SNDLOWAT: Self = Self(19);
    pub const SO_RCVTIMEO: Self = Self(20);
    pub const SO_SNDTIMEO: Self = Self(21);
    pub const SO_BINDTODEVICE: Self = Self(25);
    pub const SO_TIMESTAMP: Self = Self(29);
    pub const SO_ACCEPTCONN: Self = Self(30);
    pub const SO_PEERSEC: Self = Self(31);
    pub const SO_SNDBUFFORCE: Self = Self(32);
    pub const SO_RCVBUFFORCE: Self = Self(33);
    pub const SO_PROTOCOL: Self = Self(38);
    pub const SO_DOMAIN: Self = Self(39);
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
