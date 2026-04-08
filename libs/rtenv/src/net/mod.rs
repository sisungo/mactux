mod local;
mod sockopt;

use crate::{posix_num, util::posix_result};
use libc::c_int;
use std::mem::offset_of;
use structures::{
    ToApple,
    error::LxError,
    net::{
        Domain, MmsgHdr, MsgFlags, MsgHdr, Protocol, ShutdownHow, SockAddr, SockAddrIn,
        SockOptLevel, SocketFlags, SocketType,
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

pub fn socketpair(domain: Domain, ty: SocketType, proto: Protocol) -> Result<[c_int; 2], LxError> {
    unsafe {
        let mut fds = [0; 2];
        match libc::socketpair(
            domain.to_apple()?,
            ty.kind().to_apple()?,
            proto.to_apple()?,
            fds.as_mut_ptr(),
        ) {
            -1 => Err(LxError::last_apple_error()),
            n => Ok(n),
        }?;
        let close_fds = |_: &LxError| {
            _ = libc::close(fds[0]);
            _ = libc::close(fds[1]);
        };
        prepare_new(fds[0], ty.flags()).inspect_err(close_fds)?;
        prepare_new(fds[1], ty.flags()).inspect_err(close_fds)?;
        Ok(fds)
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

pub fn sendto(
    sock: c_int,
    buf: &[u8],
    flags: MsgFlags,
    dest: Option<SockAddr>,
) -> Result<usize, LxError> {
    unsafe {
        let has_dest = dest.is_some();
        let (addr_buf, addr_len) = match dest {
            Some(dest) => apple_sockaddr(dest, false)?,
            None => (std::mem::zeroed(), 0),
        };
        let addr_buf_ptr = if has_dest {
            (&raw const addr_buf).cast()
        } else {
            std::ptr::null()
        };
        let ret = libc::sendto(
            sock,
            buf.as_ptr().cast(),
            buf.len(),
            flags.to_apple()?,
            addr_buf_ptr,
            addr_len as _,
        );
        match ret {
            -1 => Err(LxError::last_apple_error()),
            n => Ok(n as usize),
        }
    }
}

pub unsafe fn sendmsg(sock: c_int, message: MsgHdr, flags: MsgFlags) -> Result<usize, LxError> {
    unsafe {
        let message = message.applize(apple_sockaddr)?;
        posix_num!(libc::sendmsg(sock, &message.msghdr(), flags.to_apple()?))
    }
}

pub fn recvmsg(sock: c_int, msghdr: &mut MsgHdr, flags: MsgFlags) -> Result<usize, LxError> {
    unsafe {
        let apple_msghdr_full = msghdr.clone().applize(apple_sockaddr)?;
        let mut apple_msghdr = apple_msghdr_full.msghdr();
        let n = posix_num!(libc::recvmsg(sock, &mut apple_msghdr, flags.to_apple()?))?;
        if apple_msghdr.msg_name.is_null() {
            msghdr.msg_name = None;
            msghdr.msg_namelen = 0;
        } else {
            if let Some(buf) = msghdr.msg_name.as_mut() {
                let buf = std::slice::from_raw_parts_mut(buf.as_ptr(), msghdr.msg_namelen as _);
                let apple = std::slice::from_raw_parts_mut(
                    apple_msghdr.msg_name.cast(),
                    apple_msghdr.msg_namelen as _,
                );
                msghdr.msg_namelen = linux_sockaddr(apple)?.write_to(buf)? as _;
            }
        }
        Ok(n)
    }
}

pub unsafe fn sendmmsg(
    sock: c_int,
    messages: &mut [MmsgHdr],
    flags: MsgFlags,
) -> Result<usize, LxError> {
    unsafe {
        let mut ret = 0;
        for mmsg in messages {
            let n = sendmsg(sock, mmsg.msg_hdr.clone(), flags)?;
            mmsg.msg_len = n as _;
            ret += 1;
        }
        Ok(ret)
    }
}

pub fn recvfrom(
    sock: c_int,
    buf: &mut [u8],
    flags: MsgFlags,
) -> Result<(usize, Option<SockAddr>), LxError> {
    unsafe {
        let mut addr = [0u8; size_of::<libc::sockaddr_storage>()];
        let mut addrlen = size_of::<libc::sockaddr_storage>() as libc::socklen_t;
        let ret = libc::recvfrom(
            sock,
            buf.as_mut_ptr().cast(),
            buf.len(),
            flags.to_apple()?,
            (&raw mut addr).cast(),
            &mut addrlen,
        );
        let len = match ret {
            -1 => return Err(LxError::last_apple_error()),
            n => n as usize,
        };
        Ok((len, linux_sockaddr(&addr[..(addrlen as usize)]).ok()))
    }
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
        SockAddr::Unspec => {
            buf.ss_len = 16;
            16
        }
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
