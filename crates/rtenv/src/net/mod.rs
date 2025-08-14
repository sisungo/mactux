mod local;

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
        let (buf, len) = apple_sockaddr(addr, true)?;
        posix_bi!(libc::bind(sock, buf.as_ptr().cast(), len as _))
    }
}

pub fn connect(sock: c_int, addr: SockAddr) -> Result<(), LxError> {
    unsafe {
        let (buf, len) = apple_sockaddr(addr, false)?;
        posix_bi!(libc::connect(sock, buf.as_ptr().cast(), len as _))
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

fn apple_sockaddr(linux: SockAddr, create: bool) -> Result<([u8; size_of::<libc::sockaddr_storage>()], usize), LxError> {
    let mut buf = [0; _];

    match linux {
        SockAddr::In(inet) => inet.to_apple(&mut buf)?,
        SockAddr::Un(un, len) => unsafe {
            (&raw mut buf).cast::<libc::sockaddr_un>().write(local::apple_sockaddr(un, len, create)?);
        },
    }

    Ok((buf, 0))
}
