mod local;

use crate::{posix_bi, posix_num};
use libc::c_int;
use std::mem::offset_of;
use structures::{
    error::LxError,
    net::{AcceptFlags, Domain, Protocol, ShutdownHow, SockAddr, SockAddrIn, Type},
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

pub fn accept(sock: c_int, flags: AcceptFlags) -> Result<(SockAddr, c_int), LxError> {
    unsafe {
        let mut buf = [0u8; size_of::<libc::sockaddr_storage>()];
        let mut size = size_of_val(&buf) as libc::socklen_t;
        let fd: c_int = posix_num!(libc::accept(sock, (&raw mut buf).cast(), &mut size))?;
        let sockaddr = prepare_accepted(fd, &buf[..(size as usize)], flags)
            .inspect_err(|_| _ = libc::close(fd))?;
        Ok((sockaddr, fd))
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

fn prepare_accepted(sock: c_int, addr: &[u8], flags: AcceptFlags) -> Result<SockAddr, LxError> {
    unsafe {
        if flags.contains(AcceptFlags::SOCK_NONBLOCK) {
            let flags: c_int = posix_num!(libc::fcntl(sock, libc::F_GETFL))?;
            posix_bi!(libc::fcntl(sock, libc::F_SETFL, flags | libc::O_NONBLOCK))?;
        }
        if flags.contains(AcceptFlags::SOCK_CLOEXEC) {
            let flags: c_int = posix_num!(libc::fcntl(sock, libc::F_GETFD))?;
            posix_bi!(libc::fcntl(sock, libc::F_SETFD, flags | libc::FD_CLOEXEC))?;
        }
        linux_sockaddr(addr)
    }
}

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

fn apple_sockaddr(
    linux: SockAddr,
    create: bool,
) -> Result<([u8; size_of::<libc::sockaddr_storage>()], usize), LxError> {
    let mut buf = [0; _];

    match linux {
        SockAddr::In(inet) => inet.to_apple(&mut buf)?,
        SockAddr::Un(un, len) => unsafe {
            (&raw mut buf)
                .cast::<libc::sockaddr_un>()
                .write(local::apple_sockaddr(un, len, create)?);
        },
    }

    Ok((buf, 0))
}
