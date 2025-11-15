use crate::util::posix_result;
use libc::c_int;
use structures::{
    FromApple, ToApple,
    error::LxError,
    net::{
        IP_TOS, Linger, SO_BROADCAST, SO_DEBUG, SO_DONTROUTE, SO_ERROR, SO_KEEPALIVE, SO_LINGER,
        SO_OOBINLINE, SO_RCVBUF, SO_RCVLOWAT, SO_RCVTIMEO, SO_REUSEADDR, SO_REUSEPORT, SO_SNDBUF,
        SO_SNDLOWAT, SO_SNDTIMEO, SO_TIMESTAMP, SO_TYPE, SockOptLevel, SocketKind,
    },
    time::Timeval,
};

macro_rules! auto {
    ($apple:expr, $l:ty) => {
        (auto!(@get $apple, $l), auto!(@set $apple, $l))
    };
    (@get $apple:expr, $l:ty) => {
        |fd, level, buf: &mut [u8]| unsafe {
            if buf.len() != size_of::<$l>() {
                return Err(LxError::EINVAL);
            }
            let mut apple_buf: <$l as FromApple>::Apple = std::mem::zeroed();
            let mut len = size_of::<<$l as FromApple>::Apple>() as u32;
            posix_result(libc::getsockopt(fd, level, $apple, (&raw mut apple_buf).cast(), &mut len))?;
            let linux = <$l>::from_apple(apple_buf)?;
            (buf as *mut [u8] as *mut u8).copy_from(&linux as *const _ as *const u8, size_of::<$l>());
            Ok(())
        }
    };
    (@set $apple:expr, $l:ty) => {
        |fd, level, buf| unsafe {
            if buf.len() != size_of::<$l>() {
                return Err(LxError::EINVAL);
            }
            let mut linux: $l = std::mem::zeroed();
            (&mut linux as *mut $l as *mut u8).copy_from(&linux as *const _ as *const u8, size_of::<$l>());
            let apple = linux.to_apple()?;
            let len = size_of::<<$l as ToApple>::Apple>() as u32;
            posix_result(libc::setsockopt(fd, level, $apple, (&raw const apple).cast(), len))
        }
    };
}

pub fn get(fd: c_int, lv: SockOptLevel, sockopt: u32, buf: &mut [u8]) -> Result<(), LxError> {
    level(lv)?(sockopt)?.0(fd, lv.to_apple()?, buf)
}

pub fn set(fd: c_int, lv: SockOptLevel, sockopt: u32, buf: &[u8]) -> Result<(), LxError> {
    level(lv)?(sockopt)?.1(fd, lv.to_apple()?, buf)
}

type FnGetSockOpt = fn(fd: c_int, level: c_int, buf: &mut [u8]) -> Result<(), LxError>;
type FnSetSockOpt = fn(fd: c_int, level: c_int, buf: &[u8]) -> Result<(), LxError>;
type FnSockOptLevel = fn(sockopt: u32) -> Result<(FnGetSockOpt, FnSetSockOpt), LxError>;

fn level(level: SockOptLevel) -> Result<FnSockOptLevel, LxError> {
    match level {
        SockOptLevel::SOL_SOCKET => Ok(socket_level),
        SockOptLevel::SOL_IP => Ok(ip_level),
        _ => Err(LxError::EINVAL),
    }
}

fn ip_level(sockopt: u32) -> Result<(FnGetSockOpt, FnSetSockOpt), LxError> {
    match sockopt {
        IP_TOS => Ok(auto!(libc::IP_TOS, c_int)),
        _ => Err(LxError::EINVAL),
    }
}

fn socket_level(sockopt: u32) -> Result<(FnGetSockOpt, FnSetSockOpt), LxError> {
    match sockopt {
        SO_DEBUG => Ok(auto!(libc::SO_DEBUG, c_int)),
        SO_REUSEADDR => Ok(auto!(libc::SO_REUSEADDR, c_int)),
        SO_TYPE => Ok(auto!(libc::SO_TYPE, SocketKind)),
        SO_ERROR => Ok(auto!(libc::SO_ERROR, LxError)),
        SO_DONTROUTE => Ok(auto!(libc::SO_DONTROUTE, c_int)),
        SO_BROADCAST => Ok(auto!(libc::SO_BROADCAST, c_int)),
        SO_SNDBUF => Ok(auto!(libc::SO_SNDBUF, c_int)),
        SO_RCVBUF => Ok(auto!(libc::SO_RCVBUF, c_int)),
        SO_KEEPALIVE => Ok(auto!(libc::SO_KEEPALIVE, c_int)),
        SO_OOBINLINE => Ok(auto!(libc::SO_OOBINLINE, c_int)),
        SO_LINGER => Ok(auto!(libc::SO_LINGER, Linger)),
        SO_REUSEPORT => Ok(auto!(libc::SO_REUSEPORT, c_int)),
        SO_RCVLOWAT => Ok(auto!(libc::SO_RCVLOWAT, c_int)),
        SO_SNDLOWAT => Ok(auto!(libc::SO_SNDLOWAT, c_int)),
        SO_RCVTIMEO => Ok(auto!(libc::SO_RCVTIMEO, Timeval)),
        SO_SNDTIMEO => Ok(auto!(libc::SO_SNDTIMEO, Timeval)),
        SO_TIMESTAMP => Ok(auto!(libc::SO_TIMESTAMP, c_int)),
        _ => Err(LxError::EINVAL),
    }
}
