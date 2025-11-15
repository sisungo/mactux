mod local;
mod sockopt;

use crate::{posix_num, util::posix_result};
use libc::c_int;
use std::mem::offset_of;
use structures::{
    ToApple,
    error::LxError,
    net::{
        Domain, Protocol, ShutdownHow, SockAddr, SockAddrIn, SockOptLevel, SocketFlags, SocketType,
    },
};

pub fn socket(domain: Domain, ty: SocketType, proto: Protocol) -> Result<c_int, LxError> {
    unsafe {
        let fd = match libc::socket(domain.to_apple()?, ty.kind().to_apple()?, proto.to_apple()?) {
            -1 => Err(LxError::last_apple_error()),
            n => Ok(n),
        }?;
        prepare_new(fd, ty.flags()).inspect_err(|_| _ = libc::close(fd))?;
        Ok(fd)
    }
}

pub fn bind(sock: c_int, addr: SockAddr) -> Result<(), LxError> {
    unsafe {
        let (buf, len) = apple_sockaddr(addr, true)?;
        posix_result(libc::bind(sock, (&raw const buf).cast(), len as _))
    }
}

pub fn connect(sock: c_int, addr: SockAddr) -> Result<(), LxError> {
    unsafe {
        let (buf, len) = apple_sockaddr(addr, false)?;
        posix_result(libc::connect(sock, (&raw const buf).cast(), len as _))
    }
}

pub fn listen(sock: c_int, backlog: c_int) -> Result<(), LxError> {
    unsafe { posix_result(libc::listen(sock, backlog)) }
}

pub fn accept(sock: c_int, flags: SocketFlags) -> Result<(SockAddr, c_int), LxError> {
    unsafe {
        let mut buf = [0u8; size_of::<libc::sockaddr_storage>()];
        let mut size = size_of_val(&buf) as libc::socklen_t;
        let fd: c_int = posix_num!(libc::accept(sock, (&raw mut buf).cast(), &mut size))?;
        prepare_new(fd, flags).inspect_err(|_| _ = libc::close(fd))?;
        let sockaddr =
            linux_sockaddr(&buf[..(size as usize)]).inspect_err(|_| _ = libc::close(fd))?;
        Ok((sockaddr, fd))
    }
}

pub fn getsockname(sock: c_int) -> Result<SockAddr, LxError> {
    unsafe {
        let mut buf = [0u8; size_of::<libc::sockaddr_storage>()];
        let mut size = size_of_val(&buf) as libc::socklen_t;
        posix_result(libc::getsockname(sock, (&raw mut buf).cast(), &mut size))?;
        linux_sockaddr(&buf[..(size as usize)])
    }
}

pub fn getpeername(sock: c_int) -> Result<SockAddr, LxError> {
    unsafe {
        let mut buf = [0u8; size_of::<libc::sockaddr_storage>()];
        let mut size = size_of_val(&buf) as libc::socklen_t;
        posix_result(libc::getpeername(sock, (&raw mut buf).cast(), &mut size))?;
        linux_sockaddr(&buf[..(size as usize)])
    }
}

pub fn shutdown(sock: c_int, how: ShutdownHow) -> Result<(), LxError> {
    unsafe {
        match libc::shutdown(sock, how.to_apple()?) {
            -1 => Err(LxError::last_apple_error()),
            _ => Ok(()),
        }
    }
}

pub fn getsockopt(
    sock: c_int,
    level: SockOptLevel,
    opt: u32,
    buf: &mut [u8],
) -> Result<(), LxError> {
    sockopt::get(sock, level, opt, buf)
}

pub fn setsockopt(sock: c_int, level: SockOptLevel, opt: u32, buf: &[u8]) -> Result<(), LxError> {
    sockopt::set(sock, level, opt, buf)
}

/// Prepares a socket with given Linux-specific socket flags.
fn prepare_new(sock: c_int, flags: SocketFlags) -> Result<(), LxError> {
    unsafe {
        if flags.contains(SocketFlags::SOCK_NONBLOCK) {
            let flags: c_int = posix_num!(libc::fcntl(sock, libc::F_GETFL))?;
            posix_result(libc::fcntl(sock, libc::F_SETFL, flags | libc::O_NONBLOCK))?;
        }
        if flags.contains(SocketFlags::SOCK_CLOEXEC) {
            crate::io::set_cloexec(sock)?;
        }
        Ok(())
    }
}

/// Converts from an Apple socket address to a Linux one.
fn linux_sockaddr(apple: &[u8]) -> Result<SockAddr, LxError> {
    if apple.len() < offset_of!(libc::sockaddr, sa_data) {
        return Err(LxError::ENOMEM);
    }
    unsafe {
        let header = (apple as *const [u8]).cast::<libc::sockaddr>();
        let (_len, family) = ((*header).sa_len, (*header).sa_family as c_int);
        match family {
            libc::AF_LOCAL => {
                let (lx_addr, lx_len) = local::linux_sockaddr(apple)?;
                Ok(SockAddr::Un(lx_addr, lx_len))
            }
            libc::AF_INET => Ok(SockAddr::In(SockAddrIn::from_apple(apple)?)),
            _ => Err(LxError::EAFNOSUPPORT),
        }
    }
}

/// Converts from a Linux socket address to an Apple one.
fn apple_sockaddr(
    linux: SockAddr,
    create: bool,
) -> Result<(libc::sockaddr_storage, usize), LxError> {
    let mut buf: libc::sockaddr_storage = unsafe { std::mem::zeroed() };

    let size = match linux {
        SockAddr::In(inet) => unsafe {
            (&mut buf as *mut _ as *mut libc::sockaddr_in).write(inet.to_apple().unwrap());
            size_of::<libc::sockaddr_in>()
        },
        SockAddr::Un(un, len) => unsafe {
            (&raw mut buf)
                .cast::<libc::sockaddr_un>()
                .write(local::apple_sockaddr(un, len, create)?);
            size_of::<libc::sockaddr_un>()
        },
    };

    Ok((buf, size))
}
