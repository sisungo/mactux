use crate::posix_bi;
use libc::c_int;
use structures::{
    error::LxError,
    net::{Domain, Protocol, ShutdownHow, SockAddr, Type},
};

pub fn socket(domain: Domain, ty: Type, proto: Protocol) -> Result<c_int, LxError> {
    unsafe {
        match libc::socket(domain.to_apple()?, ty.to_apple()?, proto.to_apple()?) {
            -1 => Err(LxError::last_apple_error()),
            n => Ok(n),
        }
    }
}

pub fn bind(sock: c_int, addr: SockAddr) -> Result<(), LxError> {
    unsafe {
        let mut buf = [0; size_of::<libc::sockaddr_storage>()];
        addr.to_apple(&mut buf)?;
        posix_bi!(libc::bind(sock, buf.as_ptr().cast(), buf.len() as _))
    }
}

pub fn connect(sock: c_int, addr: SockAddr) -> Result<(), LxError> {
    unsafe {
        let mut buf = [0; size_of::<libc::sockaddr_storage>()];
        addr.to_apple(&mut buf)?;
        posix_bi!(libc::connect(sock, buf.as_ptr().cast(), buf.len() as _))
    }
}

pub fn listen(sock: c_int, backlog: c_int) -> Result<(), LxError> {
    unsafe { posix_bi!(libc::listen(sock, backlog)) }
}

pub fn shutdown(sock: c_int, how: ShutdownHow) -> Result<(), LxError> {
    unsafe {
        match libc::shutdown(sock, how.to_apple()?) {
            -1 => Err(LxError::last_apple_error()),
            _ => Ok(()),
        }
    }
}
