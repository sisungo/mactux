mod native_fcntl;
mod native_ioctl;
mod vfd;

use crate::{ipc_client::with_client, posix_bi, posix_num, util::ipc_fail};
use mactux_ipc::{
    request::{InterruptibleRequest, Request},
    response::Response,
};
use rustc_hash::FxHashMap;
use std::{
    ffi::c_int,
    os::fd::{AsRawFd, IntoRawFd},
    time::Duration,
};
use structures::{
    error::LxError, fs::OpenFlags, io::{EventFdFlags, FcntlCmd, FdFlags, FdSet, FlockOp, IoctlCmd, PollEvents, PollFd, Whence}, FromApple, ToApple
};

#[inline]
pub fn read(fd: c_int, buf: &mut [u8]) -> Result<usize, LxError> {
    if let Some(vfd) = crate::vfd::get(fd) {
        vfd::read(vfd, buf)
    } else {
        unsafe { posix_num!(libc::read(fd, buf.as_mut_ptr().cast(), buf.len())) }
    }
}

#[inline]
pub fn write(fd: c_int, buf: &[u8]) -> Result<usize, LxError> {
    if let Some(vfd) = crate::vfd::get(fd) {
        vfd::write(vfd, buf)
    } else {
        unsafe { posix_num!(libc::write(fd, buf.as_ptr().cast(), buf.len())) }
    }
}

#[inline]
pub unsafe fn ioctl(fd: c_int, cmd: IoctlCmd, arg: *mut u8) -> Result<c_int, LxError> {
    if let Some(vfd) = crate::vfd::get(fd) {
        vfd::ioctl(vfd, cmd.0, arg)
    } else {
        native_ioctl::native_ioctl(fd, cmd, arg)
    }
}

#[inline]
pub unsafe fn fcntl(fd: c_int, cmd: FcntlCmd, arg: usize) -> Result<c_int, LxError> {
    if let Some(vfd) = crate::vfd::get(fd) {
        if cmd == FcntlCmd::F_DUPFD {
            let new_fd = unsafe { posix_num!(libc::fcntl(fd, libc::F_DUPFD, arg))? };
            let new_vfd = vfd::dup(vfd);
            crate::vfd::register(new_fd, new_vfd);
            return Ok(new_fd);
        }
        if cmd == FcntlCmd::F_DUPFD_CLOEXEC {
            let new_fd = unsafe { posix_num!(libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, arg))? };
            let new_vfd = vfd::dup(vfd);
            let orig_flags =
                vfd::fcntl(new_vfd, FcntlCmd::F_GETFD.0, 0).inspect_err(|_| vfd::close(new_vfd))?;
            vfd::fcntl(
                new_vfd,
                FcntlCmd::F_SETFD.0,
                (orig_flags as u32 | FdFlags::FD_CLOEXEC.bits()) as usize,
            )
            .inspect_err(|_| vfd::close(new_vfd))?;
            crate::vfd::register(new_fd, new_vfd);
            return Ok(new_fd);
        }

        vfd::fcntl(vfd, cmd.0, arg)
    } else {
        native_fcntl::native_fcntl(fd, cmd, arg)
    }
}

#[inline]
pub unsafe fn flock(fd: c_int, op: FlockOp) -> Result<(), LxError> {
    if let Some(vfd) = crate::vfd::get(fd) {
        todo!()
    } else {
        unsafe { posix_bi!(libc::flock(fd, op.to_apple()?)) }
    }
}

#[inline]
pub fn dup(fd: c_int) -> Result<c_int, LxError> {
    if let Some(vfd) = crate::vfd::get(fd) {
        let fd = std::fs::File::open("/dev/null")?.into_raw_fd();
        let new_vfd = vfd::dup(vfd);
        crate::vfd::register(fd, new_vfd);
        Ok(fd)
    } else {
        unsafe { posix_num!(libc::dup(fd)) }
    }
}

#[inline]
pub fn dup2(old: c_int, new: c_int) -> Result<c_int, LxError> {
    if old == new {
        return Ok(new);
    }

    if let Some(vfd) = crate::vfd::get(old) {
        let new_fd = unsafe { posix_num!(libc::dup2(old, new))? };
        let new_vfd = vfd::dup(vfd);
        crate::vfd::register(new_fd, new_vfd);
        Ok(new)
    } else {
        unsafe { posix_num!(libc::dup2(old, new)) }
    }
}

#[inline]
pub unsafe fn lseek(fd: c_int, off: i64, whence: Whence) -> Result<u64, LxError> {
    if let Some(vfd) = crate::vfd::get(fd) {
        vfd::lseek(vfd, whence, off)
    } else {
        unsafe { posix_num!(libc::lseek(fd, off, whence.to_apple()?)) }
    }
}

#[inline]
pub fn pread64(fd: c_int, buf: &mut [u8], off: i64) -> Result<usize, LxError> {
    if let Some(vfd) = crate::vfd::get(fd) {
        vfd::pread(vfd, off, buf)
    } else {
        unsafe { posix_num!(libc::pread(fd, buf.as_mut_ptr().cast(), buf.len(), off)) }
    }
}

#[inline]
pub fn pwrite64(fd: c_int, buf: &[u8], off: i64) -> Result<usize, LxError> {
    if let Some(vfd) = crate::vfd::get(fd) {
        vfd::pwrite(vfd, off, buf)
    } else {
        unsafe { posix_num!(libc::pwrite(fd, buf.as_ptr().cast(), buf.len(), off)) }
    }
}

#[inline]
pub fn readv(fd: c_int, vec: &[libc::iovec]) -> Result<usize, LxError> {
    if let Some(vfd) = crate::vfd::get(fd) {
        let mut count = 0;
        for vec in vec {
            let buf =
                unsafe { std::slice::from_raw_parts_mut(vec.iov_base as *mut u8, vec.iov_len) };
            let n = vfd::read(vfd, buf)?;
            count += n;
            if n != buf.len() {
                break;
            }
        }
        Ok(count)
    } else {
        if vec.len() > i32::MAX as _ {
            return Err(LxError::EINVAL);
        }

        unsafe { posix_num!(libc::readv(fd, vec.as_ptr(), vec.len() as _)) }
    }
}

#[inline]
pub unsafe fn writev(fd: c_int, vec: &[libc::iovec]) -> Result<usize, LxError> {
    if let Some(vfd) = crate::vfd::get(fd) {
        let mut count = 0;
        for vec in vec {
            if vec.iov_base.is_null() || vec.iov_len == 0 {
                continue;
            }
            let buf = unsafe { std::slice::from_raw_parts(vec.iov_base as *const u8, vec.iov_len) };
            let n = vfd::write(vfd, buf)?;
            count += n;
            if n != buf.len() {
                break;
            }
        }
        Ok(count)
    } else {
        if vec.len() > i32::MAX as _ {
            return Err(LxError::EINVAL);
        }

        unsafe { posix_num!(libc::writev(fd, vec.as_ptr(), vec.len() as _)) }
    }
}

#[inline]
pub unsafe fn poll(fds: &mut [PollFd], timeout: Option<Duration>) -> Result<u32, LxError> {
    let mut apple_fds = Vec::with_capacity(fds.len());
    let mut virtual_fds = Vec::new();
    let mut virtual_fd_map = FxHashMap::default();

    let millis = match timeout {
        None => -1,
        Some(dur) => dur.as_millis() as _,
    };

    for (n, poll_fd) in fds.iter_mut().enumerate() {
        if let Some(vfd) = crate::vfd::get(poll_fd.fd) {
            virtual_fds.push((vfd, poll_fd.events.bits()));
            virtual_fd_map.insert(vfd, n);
            continue;
        }
        apple_fds.push(libc::pollfd {
            fd: poll_fd.fd,
            events: poll_fd.events.to_apple()?,
            revents: 0,
        });
    }

    let client = if !virtual_fds.is_empty() {
        let client = crate::ipc_client::begin_interruptible(InterruptibleRequest::VirtualFdPoll(
            virtual_fds,
            timeout,
        ));
        apple_fds.push(libc::pollfd {
            fd: client.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        });
        Some(client)
    } else {
        None
    };

    unsafe {
        match libc::poll(apple_fds.as_mut_ptr(), apple_fds.len() as _, millis) {
            -1 => Err(LxError::last_apple_error()),
            n => {
                if let Some(mut client) = client {
                    if (apple_fds.last().unwrap().revents | libc::POLLIN) != 0 {
                        match client.wait() {
                            Response::Nothing => (),
                            Response::Poll(vfd, revent) => {
                                fds[virtual_fd_map[&vfd]].revents =
                                    PollEvents::from_bits_retain(revent);
                            }
                            Response::Error(err) => {
                                return Err(err);
                            }
                            _ => ipc_fail(),
                        }
                    }
                }
                for (n, apple_fd) in apple_fds.into_iter().enumerate() {
                    fds[n].revents = PollEvents::from_apple(apple_fd.revents)?;
                }
                Ok(n as _)
            }
        }
    }
}

pub unsafe fn select(
    read_fds: Option<FdSet>,
    write_fds: Option<FdSet>,
    expect_fds: Option<FdSet>,
    timeout: Option<Duration>,
) -> Result<u32, LxError> {
    unsafe {
        let mut poll_fds = Vec::new();
        let mut fd_mapping = FxHashMap::default();

        for fd in read_fds.iter().flat_map(|x| x.iter()) {
            poll_fds.push(PollFd {
                fd,
                events: PollEvents::POLLIN,
                revents: PollEvents::empty(),
            });
            fd_mapping.insert(poll_fds.len() - 1, (0, fd));
        }
        for fd in write_fds.iter().flat_map(|x| x.iter()) {
            poll_fds.push(PollFd {
                fd,
                events: PollEvents::POLLOUT,
                revents: PollEvents::empty(),
            });
            fd_mapping.insert(poll_fds.len() - 1, (1, fd));
        }
        for fd in expect_fds.iter().flat_map(|x| x.iter()) {
            poll_fds.push(PollFd {
                fd,
                events: PollEvents::POLLPRI | PollEvents::POLLERR,
                revents: PollEvents::empty(),
            });
            fd_mapping.insert(poll_fds.len() - 1, (2, fd));
        }

        let result = poll(&mut poll_fds, timeout);

        if let Some(x) = &read_fds {
            x.clear();
        }
        if let Some(x) = &write_fds {
            x.clear();
        }
        if let Some(x) = &expect_fds {
            x.clear();
        }

        for (n, poll_fd) in poll_fds.into_iter().enumerate() {
            if !poll_fd.revents.is_empty() {
                match fd_mapping[&n] {
                    (0, fd) => {
                        read_fds.as_ref().unwrap().insert(fd);
                    }
                    (1, fd) => {
                        write_fds.as_ref().unwrap().insert(fd);
                    }
                    (2, fd) => {
                        expect_fds.as_ref().unwrap().insert(fd);
                    }
                    _ => unreachable!(),
                }
            }
        }
        result
    }
}

#[inline]
pub fn truncate(fd: c_int, len: u64) -> Result<(), LxError> {
    if let Some(vfd) = crate::vfd::get(fd) {
        vfd::truncate(vfd, len)
    } else {
        unsafe { posix_bi!(libc::ftruncate(fd, len as _)) }
    }
}

#[inline]
pub fn fsync(fd: c_int) -> Result<(), LxError> {
    if let Some(vfd) = crate::vfd::get(fd) {
        todo!()
    } else {
        unsafe { posix_bi!(libc::fsync(fd)) }
    }
}

#[inline]
pub fn fdatasync(fd: c_int) -> Result<(), LxError> {
    fsync(fd)
}

#[inline]
pub fn syncfs(fd: c_int) -> Result<(), LxError> {
    fsync(fd)
}

#[inline]
pub fn close(fd: c_int) -> Result<(), LxError> {
    if crate::process::context().important_fds.pin().contains(&fd) {
        return Err(LxError::EPERM);
    }

    if let Some(vfd) = crate::vfd::take(fd) {
        vfd::close(vfd);
    }
    unsafe { posix_bi!(libc::close(fd)) }
}

#[inline]
pub fn pipe(flags: OpenFlags) -> Result<[c_int; 2], LxError> {
    if flags.contains(OpenFlags::O_DIRECT) {
        todo!()
    }

    unsafe {
        let mut buf = [0; 2];
        posix_bi!(libc::pipe((&raw mut buf).cast()))?;
        let close_fds = |_: &LxError| {
            libc::close(buf[0]);
            libc::close(buf[1]);
        };
        for &fd in &buf {
            if flags.contains(OpenFlags::O_CLOEXEC) {
                let original: i32 =
                    posix_num!(libc::fcntl(fd, libc::F_GETFD)).inspect_err(close_fds)?;
                posix_bi!(libc::fcntl(
                    fd,
                    libc::F_SETFD,
                    (original | libc::FD_CLOEXEC) as c_int
                ))
                .inspect_err(close_fds)?;
            }
            if flags.contains(OpenFlags::O_NONBLOCK) {
                let original: i32 =
                    posix_num!(libc::fcntl(fd, libc::F_GETFL)).inspect_err(close_fds)?;
                posix_bi!(libc::fcntl(
                    fd,
                    libc::F_SETFL,
                    (original | libc::O_NONBLOCK) as c_int
                ))
                .inspect_err(close_fds)?;
            }
        }
        Ok(buf)
    }
}

#[inline]
pub fn eventfd(initval: u64, flags: EventFdFlags) -> Result<c_int, LxError> {
    with_client(|client| {
        match client
            .invoke(Request::EventFd(initval, flags.bits()))
            .unwrap()
        {
            Response::EventFd(vfd) => crate::vfd::create(vfd, flags.open_flags()),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

#[inline]
pub fn set_mount_namespace(new: u64) {
    with_client(|client| {
        client.invoke(Request::SetMountNamespace(new)).unwrap();
    });
}
