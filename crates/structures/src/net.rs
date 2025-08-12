use crate::{error::LxError, newtype_impl_to_apple};
use libc::{c_char, c_int};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Domain(pub u32);
impl Domain {
    pub const PF_LOCAL: Self = Self(1);
    pub const PF_INET: Self = Self(2);
    pub const PF_INET6: Self = Self(10);

    pub fn to_apple(self) -> Result<c_int, LxError> {
        newtype_impl_to_apple!(self = PF_LOCAL, PF_INET, PF_INET6).ok_or(LxError::EINVAL)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Type(pub u32);
impl Type {
    pub const SOCK_STREAM: Self = Self(1);
    pub const SOCK_DGRAM: Self = Self(2);
    pub const SOCK_RAW: Self = Self(3);

    pub fn to_apple(self) -> Result<c_int, LxError> {
        newtype_impl_to_apple!(self = SOCK_STREAM, SOCK_DGRAM, SOCK_RAW).ok_or(LxError::EINVAL)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Protocol(pub u32);
impl Protocol {
    pub const IPPROTO_IP: Self = Self(0);
    pub const IPPROTO_ICMP: Self = Self(1);
    pub const IPPROTO_IGMP: Self = Self(2);
    pub const IPPROTO_TCP: Self = Self(6);
    pub const IPPROTO_UDP: Self = Self(17);
    pub const IPPROTO_IPV6: Self = Self(41);
    pub const IPPROTO_ICMPV6: Self = Self(58);

    pub fn to_apple(self) -> Result<c_int, LxError> {
        newtype_impl_to_apple!(
            self = IPPROTO_IP,
            IPPROTO_ICMP,
            IPPROTO_IGMP,
            IPPROTO_TCP,
            IPPROTO_UDP,
            IPPROTO_IPV6,
            IPPROTO_ICMPV6
        )
        .ok_or(LxError::EINVAL)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ShutdownHow(pub u32);
impl ShutdownHow {
    pub const SHUT_RD: Self = Self(0);
    pub const SHUT_WR: Self = Self(1);
    pub const SHUT_RDWR: Self = Self(2);

    pub const fn to_apple(self) -> Result<c_int, LxError> {
        match self {
            Self::SHUT_RD => Ok(libc::SHUT_RD),
            Self::SHUT_WR => Ok(libc::SHUT_WR),
            Self::SHUT_RDWR => Ok(libc::SHUT_RDWR),
            _ => Err(LxError::EINVAL),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SockAddr {
    Un(SockAddrUn),
    In(SockAddrIn),
}
impl SockAddr {
    pub fn from_bytes(buf: &[u8]) -> Result<Self, LxError> {
        unsafe {
            let domain = buf.as_ptr().cast::<SaFamily>().read().to_domain();
            match domain {
                Domain::PF_LOCAL => SockAddrUn::from_bytes(buf).map(Self::Un),
                Domain::PF_INET => SockAddrIn::from_bytes(buf).map(Self::In),
                _ => Err(LxError::EAFNOSUPPORT),
            }
        }
    }

    pub fn to_apple(&self, buf: &mut [u8]) -> Result<(), LxError> {
        match self {
            Self::Un(un) => un.to_apple(buf),
            Self::In(inet) => inet.to_apple(buf),
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

    pub fn to_apple(&self, buf: &mut [u8]) -> Result<(), LxError> {
        if buf.len() < size_of::<Self>() {
            return Err(LxError::ENOMEM);
        }
        let mut sun_path = [0; _];
        let path_size = size_of_val(&sun_path);
        sun_path.copy_from_slice(&self.sun_path[..path_size]);
        unsafe {
            buf.as_mut_ptr()
                .cast::<libc::sockaddr_un>()
                .write(libc::sockaddr_un {
                    sun_len: size_of::<libc::sockaddr_un>() as _,
                    sun_family: libc::AF_LOCAL as _,
                    sun_path,
                });
        }
        Ok(())
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
