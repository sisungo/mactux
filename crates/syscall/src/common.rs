use super::UcontextExt;
use crate::util::{rust_bytes, with_openat};
use libc::{c_char, c_int, c_uint, c_void};
use macros::syscall;
use rtenv::{error_report::ErrorReport, posix_num};
use std::{io::Write, ptr::NonNull, time::Duration};
use structures::{
    error::LxError,
    fs::{AccessFlags, AtFlags, OpenFlags, Stat, Statx},
    io::{EventFdFlags, FcntlCmd, FdSet, FlockOp, IoctlCmd, PSelectSigMask, PollFd, Whence},
    misc::{GrndFlags, SysInfo, UtsName},
    mm::{Madvice, MmapFlags, MmapProt, MremapFlags, MsyncFlags},
    net::{Domain, Protocol, ShutdownHow, SockAddr, SocketFlags, SocketType},
    process::{PrctlOp, RLimit64, RLimitable, RUsage, RUsageWho, WaitOptions, WaitStatus},
    signal::{KernelSigSet, MaskHowto, SigAction, SigNum},
    sync::{FutexCmd, FutexOp, RSeq},
    time::{ClockId, Timespec, Timeval, Timezone},
};

// -== Filesystem Operations ==-

#[syscall]
pub unsafe fn sys_open(path: *const c_char, flags: OpenFlags, mode: u32) -> Result<c_int, LxError> {
    unsafe { rtenv::fs::open(rust_bytes(path).to_vec(), flags, mode) }
}

#[syscall]
pub unsafe fn sys_openat(
    dfd: c_int,
    filename: *const c_char,
    flags: OpenFlags,
    mode: u32,
) -> Result<c_int, LxError> {
    unsafe {
        rtenv::fs::openat(
            dfd,
            rust_bytes(filename).to_vec(),
            flags,
            AtFlags::empty(),
            mode,
        )
    }
}

#[syscall]
pub unsafe fn sys_access(path: *const c_char, mode: AccessFlags) -> Result<(), LxError> {
    unsafe { rtenv::fs::access(rust_bytes(path).to_vec(), mode) }
}

#[syscall]
pub unsafe fn sys_faccessat2(
    dfd: c_int,
    path: *const c_char,
    mode: AccessFlags,
    flags: AtFlags,
) -> Result<(), LxError> {
    unsafe { rtenv::fs::faccessat2(dfd, rust_bytes(path).to_vec(), mode, flags) }
}

#[syscall]
pub unsafe fn sys_stat(filename: *const c_char, statbuf: *mut Stat) -> Result<(), LxError> {
    unsafe {
        let stat = with_openat(
            -100,
            rust_bytes(filename).to_vec(),
            OpenFlags::O_PATH,
            AtFlags::empty(),
            0,
            |fd| rtenv::fs::stat(fd),
        )?;
        statbuf.write(stat.into());
        Ok(())
    }
}

#[syscall]
pub unsafe fn sys_newfstatat(
    dfd: c_int,
    filename: *const c_char,
    statbuf: *mut Stat,
    flags: AtFlags,
) -> Result<(), LxError> {
    unsafe {
        let stat = with_openat(
            dfd,
            rust_bytes(filename).to_vec(),
            OpenFlags::O_PATH,
            flags,
            0,
            |fd| rtenv::fs::stat(fd),
        )?;
        statbuf.write(stat.into());
        Ok(())
    }
}

#[syscall]
pub unsafe fn sys_fstat(fd: c_int, statbuf: *mut Stat) -> Result<(), LxError> {
    unsafe {
        statbuf.write(rtenv::fs::stat(fd)?.into());
        Ok(())
    }
}

#[syscall]
pub unsafe fn sys_lstat(filename: *const c_char, statbuf: *mut Stat) -> Result<(), LxError> {
    unsafe {
        let stat = with_openat(
            -100,
            rust_bytes(filename).to_vec(),
            OpenFlags::O_NOFOLLOW | OpenFlags::O_PATH,
            AtFlags::empty(),
            0,
            |fd| rtenv::fs::stat(fd),
        )?;
        statbuf.write(stat.into());
        Ok(())
    }
}

#[syscall]
pub unsafe fn sys_statx(
    dfd: c_int,
    pathname: *const c_char,
    flags: AtFlags,
    _mask: u32, // TODO
    buf: *mut Statx,
) -> Result<(), LxError> {
    unsafe {
        let statx = with_openat(
            dfd,
            rust_bytes(pathname).to_vec(),
            OpenFlags::O_PATH,
            flags,
            0,
            |fd| rtenv::fs::stat(fd),
        )?;
        buf.write(statx);
        Ok(())
    }
}

#[syscall]
pub unsafe fn sys_truncate(path: *const c_char, len: u64) -> Result<(), LxError> {
    unsafe {
        let fd = rtenv::fs::open(rust_bytes(path).to_vec(), OpenFlags::O_WRONLY, 0)?;
        let result = rtenv::io::truncate(fd, len);
        _ = rtenv::io::close(fd);
        result
    }
}

#[syscall]
pub unsafe fn sys_readlink(
    path: *const c_char,
    buf: *mut c_char,
    bufsiz: usize,
) -> Result<(), LxError> {
    Err(LxError::EOPNOTSUPP)
}

#[syscall]
pub unsafe fn sys_getdents64(fd: c_int, dp: *mut u8, count: c_int) -> Result<usize, LxError> {
    let Some(dirent) = rtenv::fs::getdents64(fd)? else {
        return Ok(0);
    };
    if dirent.size() > count as _ {
        return Err(LxError::ENOMEM);
    }
    unsafe {
        dirent.write_to(dp);
    }
    Ok(dirent.size())
}

#[syscall]
pub unsafe fn sys_getcwd(buf: *mut u8, bufsz: usize) -> Result<*mut u8, LxError> {
    let cwd = rtenv::fs::getcwd();
    if bufsz < cwd.len() + 1 {
        return Err(LxError::ENOMEM);
    }

    unsafe {
        buf.copy_from(cwd.as_ptr(), cwd.len());
        buf.add(cwd.len()).write(0);
    }

    Ok(buf)
}

#[syscall]
pub unsafe fn sys_chdir(buf: *const c_char) -> Result<(), LxError> {
    unsafe { rtenv::fs::chdir(rust_bytes(buf).to_vec()) }
}

#[syscall]
pub unsafe fn sys_fchdir(fd: c_int) -> Result<(), LxError> {
    rtenv::fs::fchdir(fd)
}

#[syscall]
pub unsafe fn sys_umask(mask: c_int) -> c_int {
    unsafe { libc::umask(mask as _) as _ }
}

#[syscall]
pub unsafe fn sys_sync() {
    unsafe {
        libc::sync();
    }
}

#[syscall]
pub unsafe fn sys_symlink(src: *const c_char, dst: *const c_char) -> Result<(), LxError> {
    unsafe { rtenv::fs::symlink(rust_bytes(src).to_vec(), rust_bytes(dst).to_vec()) }
}

#[syscall]
pub unsafe fn sys_rename(src: *const c_char, dst: *const c_char) -> Result<(), LxError> {
    unsafe { rtenv::fs::rename(rust_bytes(src).to_vec(), rust_bytes(dst).to_vec()) }
}

#[syscall]
pub unsafe fn sys_mkdir(path: *const c_char, mode: u32) -> Result<(), LxError> {
    unsafe { rtenv::fs::mkdir(rust_bytes(path).to_vec(), mode) }
}

#[syscall]
pub unsafe fn sys_unlink(path: *const c_char) -> Result<(), LxError> {
    unsafe { rtenv::fs::unlink(rust_bytes(path).to_vec()) }
}

#[syscall]
pub unsafe fn sys_rmdir(path: *const c_char) -> Result<(), LxError> {
    unsafe { rtenv::fs::rmdir(rust_bytes(path).to_vec()) }
}

#[syscall]
pub unsafe fn sys_chown(path: *const c_char, uid: u32, gid: u32) -> Result<(), LxError> {
    unsafe {
        let fd = rtenv::fs::open(rust_bytes(path).to_vec(), OpenFlags::O_PATH, 0)?;
        let result = rtenv::fs::chown(fd, uid, gid);
        _ = rtenv::io::close(fd);
        result
    }
}

#[syscall]
pub unsafe fn sys_fchown(fd: c_int, uid: u32, gid: u32) -> Result<(), LxError> {
    unsafe { rtenv::fs::chown(fd, uid, gid) }
}

#[syscall]
pub unsafe fn sys_listxattr(
    path: *const c_char,
    list: *mut u8,
    size: usize,
) -> Result<usize, LxError> {
    unsafe {
        with_openat(
            -100,
            rust_bytes(path).to_vec(),
            OpenFlags::O_PATH,
            AtFlags::empty(),
            0,
            |fd| crate::util::ret_buf(&rtenv::fs::listxattr(fd)?, list, size),
        )
    }
}

#[syscall]
pub unsafe fn sys_llistxattr(
    path: *const c_char,
    list: *mut u8,
    size: usize,
) -> Result<usize, LxError> {
    unsafe {
        with_openat(
            -100,
            rust_bytes(path).to_vec(),
            OpenFlags::O_PATH | OpenFlags::O_NOFOLLOW,
            AtFlags::empty(),
            0,
            |fd| crate::util::ret_buf(&rtenv::fs::listxattr(fd)?, list, size),
        )
    }
}

#[syscall]
pub unsafe fn sys_flistxattr(fd: c_int, list: *mut u8, size: usize) -> Result<usize, LxError> {
    unsafe { crate::util::ret_buf(&rtenv::fs::listxattr(fd)?, list, size) }
}

// -== Basic IO Operations ==-

#[syscall]
pub unsafe fn sys_read(fd: c_int, buf: *mut u8, count: usize) -> Result<usize, LxError> {
    unsafe { rtenv::io::read(fd, std::slice::from_raw_parts_mut(buf, count)) }
}

#[syscall]
pub unsafe fn sys_pread64(
    fd: c_int,
    buf: *mut u8,
    count: usize,
    pos: i64,
) -> Result<usize, LxError> {
    unsafe { rtenv::io::pread64(fd, std::slice::from_raw_parts_mut(buf, count), pos) }
}

#[syscall]
pub unsafe fn sys_readv(fd: c_int, vec: *const libc::iovec, vlen: usize) -> Result<usize, LxError> {
    unsafe { rtenv::io::readv(fd, std::slice::from_raw_parts(vec, vlen)) }
}

#[syscall]
pub unsafe fn sys_write(fd: c_int, buf: *mut u8, count: usize) -> Result<usize, LxError> {
    unsafe { rtenv::io::write(fd, std::slice::from_raw_parts(buf, count)) }
}

#[syscall]
pub unsafe fn sys_pwrite64(
    fd: c_int,
    buf: *const u8,
    count: usize,
    pos: i64,
) -> Result<usize, LxError> {
    unsafe { rtenv::io::pwrite64(fd, std::slice::from_raw_parts(buf, count), pos) }
}

#[syscall]
pub unsafe fn sys_writev(
    fd: c_int,
    vec: *const libc::iovec,
    vlen: usize,
) -> Result<usize, LxError> {
    unsafe { rtenv::io::writev(fd, std::slice::from_raw_parts(vec, vlen)) }
}

#[syscall]
pub unsafe fn sys_lseek(fd: c_int, off: i64, whence: Whence) -> Result<u64, LxError> {
    unsafe { rtenv::io::lseek(fd, off, whence) }
}

#[syscall]
pub unsafe fn sys_fcntl(fd: c_int, cmd: FcntlCmd, arg: usize) -> Result<c_int, LxError> {
    unsafe { rtenv::io::fcntl(fd, cmd, arg) }
}

#[syscall]
pub unsafe fn sys_flock(fd: c_int, op: FlockOp) -> Result<(), LxError> {
    unsafe { rtenv::io::flock(fd, op) }
}

#[syscall]
pub unsafe fn sys_ioctl(fd: c_int, cmd: IoctlCmd, arg: *mut u8) -> Result<c_int, LxError> {
    unsafe { rtenv::io::ioctl(fd, cmd, arg) }
}

#[syscall]
pub unsafe fn sys_ftruncate(fd: c_int, len: u64) -> Result<(), LxError> {
    rtenv::io::truncate(fd, len)
}

#[syscall]
pub unsafe fn sys_dup(fd: c_int) -> Result<c_int, LxError> {
    rtenv::io::dup(fd)
}

#[syscall]
pub unsafe fn sys_dup2(old: c_int, new: c_int) -> Result<c_int, LxError> {
    rtenv::io::dup2(old, new)
}

#[syscall]
pub unsafe fn sys_fsync(fd: c_int) -> Result<(), LxError> {
    rtenv::io::fsync(fd)
}

#[syscall]
pub unsafe fn sys_fdatasync(fd: c_int) -> Result<(), LxError> {
    rtenv::io::fdatasync(fd)
}

#[syscall]
pub unsafe fn sys_syncfs(fd: c_int) -> Result<(), LxError> {
    rtenv::io::syncfs(fd)
}

#[syscall]
pub unsafe fn sys_close(fd: c_int) -> Result<(), LxError> {
    rtenv::io::close(fd)
}

#[syscall]
pub unsafe fn sys_pipe(fildes: *mut [c_int; 2]) -> Result<(), LxError> {
    let fds = rtenv::io::pipe(OpenFlags::empty())?;
    unsafe {
        fildes.cast::<[c_int; 2]>().write(fds);
    }
    Ok(())
}

#[syscall]
pub unsafe fn sys_pipe2(fildes: *mut [c_int; 2], flags: OpenFlags) -> Result<(), LxError> {
    let fds = rtenv::io::pipe(flags)?;
    unsafe {
        fildes.cast::<[c_int; 2]>().write(fds);
    }
    Ok(())
}

#[syscall]
pub unsafe fn sys_fadvise64(
    _fd: c_int,
    _off: i64,
    _len: usize,
    _advice: c_int,
) -> Result<(), LxError> {
    Ok(())
}

// -== Zero-copy IO Operations ==-

#[syscall]
pub unsafe fn sys_copy_file_range(
    fd_in: c_int,
    off_in: Option<NonNull<i64>>,
    fd_out: c_int,
    off_out: Option<NonNull<i64>>,
    len: usize,
    flags: u32,
) -> Result<usize, LxError> {
    if flags != 0 {
        // Linux has no supported flags for this yet, so we simply require an empty one.
        return Err(LxError::EINVAL);
    }

    let mut buf = vec![0u8; len.min(4096)];
    let bytes_read = match off_in {
        Some(ptr) => unsafe {
            let n = rtenv::io::pread64(fd_in, &mut buf, ptr.read())?;
            ptr.write(ptr.read() + n as i64);
            n
        },
        None => rtenv::io::read(fd_in, &mut buf)?,
    };
    let bytes_written = match off_out {
        Some(ptr) => unsafe {
            let n = rtenv::io::pwrite64(fd_in, &buf[..bytes_read], ptr.read())?;
            ptr.write(ptr.read() + n as i64);
            n
        },
        None => rtenv::io::write(fd_out, &buf[..bytes_read])?,
    };
    Ok(bytes_written)
}

#[syscall]
pub unsafe fn sys_sendfile(
    out_fd: c_int,
    in_fd: c_int,
    off_in: Option<NonNull<i64>>,
    count: usize,
) -> Result<usize, LxError> {
    let mut buf = vec![0u8; count.min(4096)];
    let bytes_read = match off_in {
        Some(ptr) => unsafe {
            let n = rtenv::io::pread64(in_fd, &mut buf, ptr.read())?;
            ptr.write(ptr.read() + n as i64);
            n
        },
        None => rtenv::io::read(in_fd, &mut buf)?,
    };
    let bytes_written = rtenv::io::write(out_fd, &buf[..bytes_read])?;
    Ok(bytes_written)
}

// -== POSIX Traditional IO Multiplexers ==-

#[syscall]
pub unsafe fn sys_poll(fds: *mut PollFd, nfds: c_uint, timeout: c_int) -> Result<u32, LxError> {
    let timeout = match timeout {
        -1 => None,
        other => Some(Duration::from_millis(other as _)),
    };
    unsafe { rtenv::io::poll(std::slice::from_raw_parts_mut(fds, nfds as _), timeout) }
}

#[syscall]
pub unsafe fn sys_ppoll(
    fds: *mut PollFd,
    nfds: c_uint,
    timeout: Option<NonNull<Timespec>>,
    sigset: Option<NonNull<KernelSigSet>>,
    sigset_size: usize,
) -> Result<u32, LxError> {
    unsafe {
        if sigset_size != size_of::<KernelSigSet>() {
            return Err(LxError::EINVAL);
        }

        let sigmask = sigset.map(|x| x.read());
        let timeout = timeout.map(|x| x.read().to_duration());

        let orig_mask = rtenv::signal::mask(MaskHowto::SIG_SETMASK, sigmask)?;
        let result = rtenv::io::poll(std::slice::from_raw_parts_mut(fds, nfds as _), timeout);
        rtenv::signal::mask(MaskHowto::SIG_SETMASK, Some(orig_mask))?;
        result
    }
}

#[syscall]
pub unsafe fn sys_select(
    nfds: usize,
    read_fds: Option<NonNull<u64>>,
    write_fds: Option<NonNull<u64>>,
    expect_fds: Option<NonNull<u64>>,
    timeout: Option<NonNull<Timeval>>,
) -> Result<u32, LxError> {
    unsafe {
        let read_fds = read_fds.map(|x| FdSet::new(x, nfds));
        let write_fds = write_fds.map(|x| FdSet::new(x, nfds));
        let expect_fds = expect_fds.map(|x| FdSet::new(x, nfds));
        let timeout = timeout.map(|x| x.read().to_timespec().to_duration());
        rtenv::io::select(read_fds, write_fds, expect_fds, timeout)
    }
}

#[syscall]
pub unsafe fn sys_pselect6(
    nfds: usize,
    read_fds: Option<NonNull<u64>>,
    write_fds: Option<NonNull<u64>>,
    expect_fds: Option<NonNull<u64>>,
    timeout: Option<NonNull<Timespec>>,
    sigmask: Option<NonNull<PSelectSigMask>>,
) -> Result<u32, LxError> {
    unsafe {
        let sigmask = match sigmask {
            Some(x) => Some(x.read().into_sigset()?),
            None => None,
        };
        let timeout = timeout.map(|x| x.read().to_duration());
        let read_fds = read_fds.map(|x| FdSet::new(x, nfds));
        let write_fds = write_fds.map(|x| FdSet::new(x, nfds));
        let expect_fds = expect_fds.map(|x| FdSet::new(x, nfds));

        let orig_mask = rtenv::signal::mask(MaskHowto::SIG_SETMASK, sigmask)?;
        let result = rtenv::io::select(read_fds, write_fds, expect_fds, timeout);
        rtenv::signal::mask(MaskHowto::SIG_SETMASK, Some(orig_mask))?;

        result
    }
}

// -== Linux Special File Descriptors ==-

#[syscall]
pub unsafe fn sys_eventfd(initval: u64) -> Result<c_int, LxError> {
    rtenv::io::eventfd(initval, EventFdFlags::empty())
}

#[syscall]
pub unsafe fn sys_eventfd2(initval: u64, flags: EventFdFlags) -> Result<c_int, LxError> {
    rtenv::io::eventfd(initval, flags)
}

// -== System Information Functions ==-

#[syscall]
pub unsafe fn sys_uname(buf: *mut UtsName) -> Result<(), LxError> {
    unsafe {
        buf.write(rtenv::misc::uname()?);
        Ok(())
    }
}

#[syscall]
pub unsafe fn sys_sysinfo(buf: *mut SysInfo) -> Result<(), LxError> {
    unsafe {
        buf.write(rtenv::misc::sysinfo()?);
        Ok(())
    }
}

#[syscall]
pub unsafe fn sys_sethostname(name: *const c_char, len: usize) -> Result<(), LxError> {
    let (_, domainname) = rtenv::misc::get_network_names()?;
    unsafe {
        let nodename = std::slice::from_raw_parts(name.cast::<u8>(), len).to_vec();
        rtenv::misc::set_network_names(nodename, domainname)?;
        Ok(())
    }
}

#[syscall]
pub unsafe fn sys_setdomainname(name: *const c_char, len: usize) -> Result<(), LxError> {
    let (nodename, _) = rtenv::misc::get_network_names()?;
    unsafe {
        let domainname = std::slice::from_raw_parts(name.cast::<u8>(), len).to_vec();
        rtenv::misc::set_network_names(nodename, domainname)?;
        Ok(())
    }
}

// -== Cryptography Functions ==-

#[syscall]
pub unsafe fn sys_getrandom(buf: *mut u8, len: usize, flags: GrndFlags) -> Result<usize, LxError> {
    if flags.contains(GrndFlags::GRND_NONBLOCK) {
        // TODO
    }

    if flags.contains(GrndFlags::GRND_RANDOM) {
        unsafe {
            let fd = libc::open(
                (&raw const *b"/dev/random\0").cast(),
                libc::O_RDONLY | libc::O_CLOEXEC,
            );
            if fd == -1 {
                return Err(LxError::last_apple_error());
            }
            let read_result = libc::read(fd, buf.cast(), len);
            libc::close(fd);
            posix_num!(read_result)
        }
    } else {
        unsafe {
            libc::arc4random_buf(buf.cast(), len);
        }
        Ok(len)
    }
}

// -== Network Communication ==-

#[syscall]
pub unsafe fn sys_socket(
    domain: Domain,
    ty: SocketType,
    proto: Protocol,
) -> Result<c_int, LxError> {
    rtenv::net::socket(domain, ty, proto)
}

#[syscall]
pub unsafe fn sys_listen(sock: c_int, backlog: c_int) -> Result<(), LxError> {
    rtenv::net::listen(sock, backlog)
}

#[syscall]
pub unsafe fn sys_accept(
    sock: c_int,
    buf: Option<NonNull<u8>>,
    len: Option<NonNull<u32>>,
) -> Result<c_int, LxError> {
    let (addr, fd) = rtenv::net::accept(sock, SocketFlags::empty())?;
    if let Some(buf) = buf
        && let Some(len) = len
    {
        unsafe {
            let size = addr.write_to(std::slice::from_raw_parts_mut(
                buf.as_ptr(),
                len.as_ptr().read() as usize,
            ))?;
            len.write(size as _);
        }
    }
    Ok(fd)
}

#[syscall]
pub unsafe fn sys_accept4(
    sock: c_int,
    buf: *mut u8,
    len: *mut u32,
    flags: SocketFlags,
) -> Result<c_int, LxError> {
    let (addr, fd) = rtenv::net::accept(sock, flags)?;
    unsafe {
        crate::util::ret_sockaddr(addr, buf, len)?;
    }
    Ok(fd)
}

#[syscall]
pub unsafe fn sys_getsockname(sock: c_int, buf: *mut u8, len: *mut u32) -> Result<(), LxError> {
    let addr = rtenv::net::getsockname(sock)?;
    unsafe {
        crate::util::ret_sockaddr(addr, buf, len)?;
    }
    Ok(())
}

#[syscall]
pub unsafe fn sys_getpeername(sock: c_int, buf: *mut u8, len: *mut u32) -> Result<(), LxError> {
    let addr = rtenv::net::getpeername(sock)?;
    unsafe {
        crate::util::ret_sockaddr(addr, buf, len)?;
    }
    Ok(())
}

#[syscall]
pub unsafe fn sys_bind(sock: c_int, addr: *const u8, len: c_int) -> Result<(), LxError> {
    unsafe {
        rtenv::net::bind(
            sock,
            SockAddr::from_bytes(std::slice::from_raw_parts(addr, len as usize))?,
        )
    }
}

#[syscall]
pub unsafe fn sys_connect(sock: c_int, addr: *const u8, len: c_int) -> Result<(), LxError> {
    unsafe {
        rtenv::net::connect(
            sock,
            SockAddr::from_bytes(std::slice::from_raw_parts(addr, len as usize))?,
        )
    }
}

#[syscall]
pub unsafe fn sys_shutdown(sock: c_int, how: ShutdownHow) -> Result<(), LxError> {
    rtenv::net::shutdown(sock, how)
}

// -== Memory Management ==-

#[syscall]
pub unsafe fn sys_brk(_addr: usize) -> Result<(), LxError> {
    Err(LxError::ENOSYS)
}

#[syscall]
pub unsafe fn sys_mmap(
    addr: *mut u8,
    len: usize,
    prot: MmapProt,
    flags: MmapFlags,
    fd: c_int,
    offset: i64,
) -> std::io::Result<*mut c_void> {
    unsafe {
        match libc::mmap(
            addr.cast(),
            len,
            prot.to_apple(),
            flags.to_apple(),
            fd,
            offset,
        ) {
            libc::MAP_FAILED => Err(std::io::Error::last_os_error()),
            addr => Ok(addr),
        }
    }
}

#[syscall]
pub unsafe fn sys_mprotect(addr: *mut u8, len: usize, prot: i32) -> Result<(), LxError> {
    unsafe {
        match libc::mprotect(addr.cast(), len, prot) {
            -1 => Err(LxError::last_apple_error()),
            _ => Ok(()),
        }
    }
}

#[syscall]
pub unsafe fn sys_msync(addr: *mut u8, len: usize, flags: MsyncFlags) -> Result<(), LxError> {
    unsafe {
        match libc::msync(addr.cast(), len, flags.to_apple()) {
            -1 => Err(LxError::last_apple_error()),
            _ => Ok(()),
        }
    }
}

#[syscall]
pub unsafe fn sys_mincore(addr: *mut u8, len: usize, vec: *mut u8) -> Result<(), LxError> {
    unsafe {
        // TODO
        match libc::mincore(addr.cast(), len, vec.cast()) {
            -1 => Err(LxError::last_apple_error()),
            _ => Ok(()),
        }
    }
}

#[syscall]
pub unsafe fn sys_madvise(start: *mut u8, len: usize, advice: Madvice) -> Result<(), LxError> {
    unsafe { rtenv::mm::advise(start, len, advice) }
}

#[syscall]
pub unsafe fn sys_mremap(
    addr: *mut u8,
    old_len: usize,
    new_len: usize,
    flags: MremapFlags,
    new_addr: *mut u8,
) -> Result<*mut u8, LxError> {
    unsafe { rtenv::mm::remap(addr, old_len, new_addr, new_len, flags) }
}

#[syscall]
pub unsafe fn sys_munmap(addr: *mut c_void, len: usize) -> std::io::Result<()> {
    unsafe {
        match libc::munmap(addr, len) {
            -1 => Err(std::io::Error::last_os_error()),
            _ => Ok(()),
        }
    }
}

// -== Signal Handling ==-

#[syscall]
pub unsafe fn sys_rt_sigaction(
    signum: SigNum,
    sigaction: Option<NonNull<SigAction>>,
    old_sigaction: Option<NonNull<SigAction>>,
    size: usize,
) -> Result<(), LxError> {
    if size != size_of::<KernelSigSet>() {
        return Err(LxError::EINVAL);
    }

    unsafe {
        let old = rtenv::signal::sigaction(signum, sigaction.map(|x| x.read()))?;
        if let Some(old_sigaction) = old_sigaction {
            old_sigaction.write(old);
        }
        Ok(())
    }
}

#[syscall]
pub unsafe fn sys_rt_sigprocmask(
    howto: MaskHowto,
    set: Option<NonNull<KernelSigSet>>,
    oset: Option<NonNull<KernelSigSet>>,
    size: usize,
) -> Result<(), LxError> {
    if size != size_of::<KernelSigSet>() {
        return Err(LxError::EINVAL);
    }

    unsafe {
        let old_set = rtenv::signal::mask(howto, set.map(|x| x.read()))?;
        if let Some(oset) = oset {
            oset.write(old_set);
        }
        Ok(())
    }
}

#[syscall]
pub unsafe fn sys_pause() -> Result<(), LxError> {
    unsafe {
        libc::pause();
    }
    Err(LxError::EINTR)
}

// -== Timing ==-

#[syscall]
pub unsafe fn sys_alarm(secs: c_uint) -> c_uint {
    unsafe { libc::alarm(secs) }
}

#[syscall]
pub unsafe fn sys_nanosleep(rqtp: *const Timespec, rmtp: *mut Timespec) -> std::io::Result<()> {
    unsafe {
        let rqtp = rqtp.read().to_apple();
        let mut rmtp_buf = std::mem::zeroed();
        match libc::nanosleep(&rqtp, &mut rmtp_buf) {
            -1 => Err(std::io::Error::last_os_error()),
            _ => {
                rmtp.write(Timespec::from_apple(rmtp_buf));
                Ok(())
            }
        }
    }
}

#[syscall]
pub unsafe fn sys_clock_gettime(clk_id: ClockId, tp: *mut Timespec) -> Result<(), LxError> {
    unsafe {
        let mut apple_tp = tp.read().to_apple();
        match libc::clock_gettime(clk_id.to_apple()?, &mut apple_tp) {
            -1 => Err(LxError::last_apple_error()),
            _ => {
                tp.write(Timespec::from_apple(apple_tp));
                Ok(())
            }
        }
    }
}

#[syscall]
pub unsafe fn sys_gettimeofday(
    tv: Option<NonNull<Timeval>>,
    tz: Option<NonNull<Timezone>>,
) -> Result<(), LxError> {
    unsafe {
        let mut tvbuf = std::mem::zeroed();
        let mut tzbuf: Timezone = std::mem::zeroed();
        match libc::gettimeofday(&mut tvbuf, (&raw mut tzbuf).cast()) {
            -1 => Err(LxError::last_apple_error()),
            _ => {
                if let Some(tv) = tv {
                    tv.write(Timeval::from_apple(tvbuf));
                }
                if let Some(tz) = tz {
                    tz.write(tzbuf);
                }
                Ok(())
            }
        }
    }
}

#[syscall]
pub unsafe fn sys_time(time: *mut i64) -> Result<(), LxError> {
    unsafe {
        match libc::time(time) {
            -1 => Err(LxError::last_apple_error()),
            _ => Ok(()),
        }
    }
}

// -== Synchronous ==-

#[syscall]
pub unsafe fn sys_futex(
    uaddr: *mut u32,
    op: FutexOp,
    val: u32,
    utime: *mut libc::timespec,
    uaddr2: *mut u32,
    val3: u32,
) -> Result<usize, LxError> {
    match op.cmd() {
        FutexCmd::FUTEX_WAIT => unsafe {
            rtenv::sync::futex::wait(uaddr, val, utime, op.opts()).map(|()| 0)
        },
        FutexCmd::FUTEX_WAKE => unsafe { rtenv::sync::futex::wake(uaddr, val, op.opts()) },
        FutexCmd::FUTEX_WAKE_OP => unsafe {
            rtenv::sync::futex::wake_op(uaddr, val, utime as usize as u32, uaddr2, val3, op.opts())
        },
        FutexCmd::FUTEX_LOCK_PI => unsafe {
            rtenv::sync::pi_futex::lock();
            Ok(0)
        },
        _ => Err(LxError::EINVAL),
    }
}

#[syscall]
pub unsafe fn sys_set_robust_list(head: *mut u8, size: usize) -> Result<(), LxError> {
    rtenv::thread::set_robust_list(head, size)
}

#[syscall]
pub unsafe fn sys_rseq(
    _rseq: *mut RSeq,
    _rseq_len: u32,
    _flags: u32,
    _sig: u32,
) -> Result<(), LxError> {
    Err(LxError::ENOSYS)
}

// -== Thread Management ==-

#[syscall]
pub unsafe fn sys_gettid() -> i32 {
    rtenv::thread::id()
}

#[syscall]
pub unsafe fn sys_set_tid_address(addr: Option<NonNull<u32>>) -> i32 {
    rtenv::thread::set_clear_tid(addr);
    rtenv::thread::id()
}

#[syscall]
pub unsafe fn sys_tkill(tid: i32, signum: SigNum) -> Result<(), LxError> {
    rtenv::thread::kill(tid, signum)
}

#[syscall]
pub unsafe fn sys_exit(code: c_int) {
    unsafe {
        rtenv::thread::exit(code);
    }
}

// -== Process Management ==-

#[syscall]
pub unsafe fn sys_getpid() -> c_int {
    rtenv::process::pid()
}

#[syscall]
pub unsafe fn sys_getpgid(pid: i32) -> c_int {
    rtenv::process::pgid(pid)
}

#[syscall]
pub unsafe fn sys_getpgrp() -> c_int {
    rtenv::process::pgid(rtenv::process::pid())
}

#[syscall]
pub unsafe fn sys_setpgid(pid: i32, pgid: i32) -> Result<i32, LxError> {
    rtenv::process::setpgid(pid, pgid)
}

#[syscall]
pub unsafe fn sys_getppid() -> c_int {
    rtenv::process::ppid()
}

#[syscall]
pub unsafe fn sys_kill(pid: i32, signum: SigNum) -> Result<(), LxError> {
    rtenv::process::kill(pid, signum)
}

#[syscall]
pub unsafe fn sys_execve(
    path: *const c_char,
    argv: *const *const u8,
    envp: *const *const u8,
) -> Result<(), LxError> {
    unsafe {
        let path = rust_bytes(path);

        let mut argc = 0;
        while *argv.add(argc) != std::ptr::null() {
            argc += 1;
        }

        let mut envc = 0;
        while *envp.add(envc) != std::ptr::null() {
            envc += 1;
        }

        rtenv::process::exec(
            path,
            std::slice::from_raw_parts(argv, argc),
            std::slice::from_raw_parts(envp, envc),
        )?;
        Ok(())
    }
}

#[syscall]
pub unsafe fn sys_prlimit64(
    pid: i32,
    res: RLimitable,
    new: Option<NonNull<RLimit64>>,
    old: Option<NonNull<RLimit64>>,
) -> Result<(), LxError> {
    if ![0, -1, rtenv::process::pid()].contains(&pid) {
        return Err(LxError::EPERM);
    }
    let Ok(res) = res.to_apple() else {
        return Ok(());
    };
    unsafe {
        if let Some(old) = old {
            let mut buf = std::mem::zeroed();
            if libc::getrlimit(res, &mut buf) == -1 {
                return Err(LxError::last_apple_error());
            }
            old.write(RLimit64::from_apple(buf));
        }
        if let Some(new) = new {
            let buf = new.read().to_apple();
            if libc::setrlimit(res, &buf) == -1 {
                return Err(LxError::last_apple_error());
            }
        }
    }
    Ok(())
}

#[syscall]
pub unsafe fn sys_wait4(
    pid: i32,
    stat_addr: *mut WaitStatus,
    options: WaitOptions,
    ru: Option<NonNull<RUsage>>,
) -> Result<i32, LxError> {
    unsafe {
        let mut status = 0;
        let mut apple_ru = std::mem::zeroed();
        let pid = match libc::wait4(pid, &mut status, options.to_apple(), &mut apple_ru) {
            -1 => Err(LxError::last_apple_error()),
            n => Ok(n),
        }?;
        stat_addr.write(WaitStatus::from_apple(status));
        if let Some(ru) = ru {
            ru.write(RUsage::from_apple(apple_ru));
        }
        Ok(pid)
    }
}

#[syscall]
pub unsafe fn sys_getrusage(who: RUsageWho, rusage: *mut RUsage) -> Result<(), LxError> {
    unsafe {
        let mut buf = std::mem::zeroed();
        if libc::getrusage(who.to_apple()?, &mut buf) == -1 {
            return Err(LxError::last_apple_error());
        }
        rusage.write(RUsage::from_apple(buf));
        Ok(())
    }
}

#[syscall]
pub unsafe fn sys_prctl(op: PrctlOp) -> Result<(), LxError> {
    Err(LxError::EINVAL)
}

#[syscall]
pub unsafe fn sys_exit_group(code: c_int) {
    std::process::exit(code);
}

// -== Scheduling ==-

#[syscall]
pub unsafe fn sys_sched_yield() {
    std::thread::yield_now();
}

#[syscall]
pub unsafe fn sys_sched_getaffinity(
    _pid: i32,
    cpusetsize: usize,
    cpuset: *mut u8,
) -> Result<(), LxError> {
    unsafe {
        cpuset.write_bytes(0xff, cpusetsize);
    }
    Ok(())
}

#[syscall]
pub unsafe fn sys_sched_setaffinity(
    _pid: i32,
    _cpusetsize: usize,
    _cpuset: *const u8,
) -> Result<(), LxError> {
    Err(LxError::EPERM)
}

// -== Multi-user Support ==-

#[syscall]
pub unsafe fn sys_setuid(uid: u32) -> Result<(), LxError> {
    Err(LxError::EPERM)
}

#[syscall]
pub unsafe fn sys_setgid(gid: u32) -> Result<(), LxError> {
    Err(LxError::EPERM)
}

#[syscall]
pub unsafe fn sys_getuid() -> u32 {
    rtenv::security::uid()
}

#[syscall]
pub unsafe fn sys_getgid() -> u32 {
    rtenv::security::gid()
}

#[syscall]
pub unsafe fn sys_geteuid() -> u32 {
    rtenv::security::euid()
}

#[syscall]
pub unsafe fn sys_getegid() -> u32 {
    rtenv::security::egid()
}

#[syscall]
pub unsafe fn sys_getgroups(len: c_int, list: *mut u32) -> Result<u32, LxError> {
    let groups = rtenv::security::groups();
    if len == 0 {
        return Ok(groups.len() as _);
    }
    if (len as usize) < groups.len() {
        return Err(LxError::EINVAL);
    }
    unsafe {
        std::slice::from_raw_parts_mut(list, groups.len()).copy_from_slice(&groups);
    }
    Ok(groups.len() as _)
}

// -== Program Debugging ==-

#[syscall]
pub unsafe fn sys_acct(_name: *const c_char) -> Result<(), LxError> {
    Err(LxError::ENOSYS)
}

// -== Misc ==-

#[syscall]
pub unsafe fn sys_uselib(_lib: *const c_char) -> Result<(), LxError> {
    Err(LxError::ENOSYS)
}

#[syscall]
pub unsafe fn sys_sysfs() -> Result<(), LxError> {
    Err(LxError::ENOSYS)
}

pub unsafe fn sys_invalid(uctx: &mut libc::ucontext_t) {
    unsafe {
        rtenv::emuctx::leave_emulated();
        if rtenv::switches::ignore_unsupported_syscalls() {
            _ = writeln!(
                ErrorReport,
                " [ ! ] MacTux: Unsupported syscall: {}, ignoring!",
                uctx.sysno()
            );
            uctx.ret(-(LxError::ENOSYS.0 as isize) as usize);
            rtenv::emuctx::enter_emulated();
            return;
        }

        _ = writeln!(
            ErrorReport,
            " [ ! ] MacTux: Unsupported syscall: {}, exiting!",
            uctx.sysno()
        );
        rtenv::error_report::fast_fail();
    }
}
