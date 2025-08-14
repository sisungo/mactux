use crate::{error::LxError, unixvariants};
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

unixvariants! {
    pub struct Type: u32 {
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
    pub struct ShutdownHow: u32 {
        const SHUT_RD = 0;
        const SHUT_WR = 1;
        const SHUT_RDWR = 2;
        fn from_apple(apple: c_int) -> Result<Self, LxError>;
        fn to_apple(self) -> Result<c_int, LxError>;
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
