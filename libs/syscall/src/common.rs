//! Implementations of cross-architecture system calls.

use super::UcontextExt;
use crate::util::with_openat;
use libc::{c_char, c_int, c_uint, c_void};
use macros::syscall;
use rtenv::posix_num;
use std::{ffi::CStr, num::NonZero, ptr::NonNull, time::Duration};
use structures::{
    FromApple, ToApple,
    device::DeviceNumber,
    error::LxError,
    fs::{
        AT_FDCWD, AccessFlags, AtFlags, FileMode, OpenFlags, Stat, Statx, StatxMask, UmountFlags,
    },
    io::{
        CloseRangeFlags, EventFdFlags, FcntlCmd, FdSet, FlockOp, IoctlCmd, PSelectSigMask, PollFd,
        Whence,
    },
    mactux_ipc::NetworkNames,
    misc::{GrndFlags, SysInfo, SyslogAction, UtsName},
    mm::{Madvice, MmapFlags, MmapProt, MremapFlags, MsyncFlags},
    net::{Domain, Protocol, ShutdownHow, SockAddr, SockOptLevel, SocketFlags, SocketType},
    process::{PrctlOp, RLimit64, RLimitable, RUsage, RUsageWho, WaitOptions, WaitStatus},
    signal::{KernelSigSet, MaskHowto, SigAction, SigAltStack, SigNum},
    sync::{FutexCmd, FutexOp, RSeq},
    time::{ClockId, TimerFlags, Timespec, Timeval, Timezone, Tms},
};

// -== Filesystem Operations ==-

#[syscall]
pub unsafe fn sys_open(path: &CStr, flags: OpenFlags, mode: u32) -> Result<c_int, LxError> {
    rtenv::fs::openat(
        AT_FDCWD,
        path.to_bytes().to_vec(),
        flags,
        AtFlags::empty(),
        FileMode(mode as _),
    )
}

#[syscall]
pub unsafe fn sys_creat(path: &CStr, mode: u32) -> Result<c_int, LxError> {
    rtenv::fs::openat(
        AT_FDCWD,
        path.to_bytes().to_vec(),
        OpenFlags::O_CREAT | OpenFlags::O_TRUNC | OpenFlags::O_WRONLY,
        AtFlags::empty(),
        FileMode(mode as _),
    )
}

#[syscall]
pub unsafe fn sys_openat(
    dfd: c_int,
    filename: &CStr,
    flags: OpenFlags,
    mode: u32,
) -> Result<c_int, LxError> {
    rtenv::fs::openat(
        dfd,
        filename.to_bytes().to_vec(),
        flags,
        AtFlags::empty(),
        FileMode(mode as _),
    )
}

#[syscall]
pub unsafe fn sys_access(path: &CStr, mode: AccessFlags) -> Result<(), LxError> {
    rtenv::fs::faccessat2(AT_FDCWD, path.to_bytes().to_vec(), mode, AtFlags::empty())
}

#[syscall]
pub unsafe fn sys_faccessat2(
    dfd: c_int,
    path: &CStr,
    mode: AccessFlags,
    flags: AtFlags,
) -> Result<(), LxError> {
    rtenv::fs::faccessat2(dfd, path.to_bytes().to_vec(), mode, flags)
}

#[syscall]
pub unsafe fn sys_stat(filename: &CStr, statbuf: *mut Stat) -> Result<(), LxError> {
    unsafe {
        let stat = with_openat(
            AT_FDCWD,
            filename.to_bytes().to_vec(),
            OpenFlags::O_PATH,
            AtFlags::empty(),
            0,
            |fd| rtenv::fs::fstat(fd, StatxMask::all()),
        )?;
        statbuf.write(stat.into());
        Ok(())
    }
}

#[syscall]
pub unsafe fn sys_newfstatat(
    dfd: c_int,
    filename: &CStr,
    statbuf: *mut Stat,
    flags: AtFlags,
) -> Result<(), LxError> {
    unsafe {
        let stat = with_openat(
            dfd,
            filename.to_bytes().to_vec(),
            OpenFlags::O_PATH,
            flags,
            0,
            |fd| rtenv::fs::fstat(fd, StatxMask::all()),
        )?;
        statbuf.write(stat.into());
        Ok(())
    }
}

#[syscall]
pub unsafe fn sys_fstat(fd: c_int, statbuf: *mut Stat) -> Result<(), LxError> {
    unsafe {
        statbuf.write(rtenv::fs::fstat(fd, StatxMask::all())?.into());
        Ok(())
    }
}

#[syscall]
pub unsafe fn sys_lstat(filename: &CStr, statbuf: *mut Stat) -> Result<(), LxError> {
    unsafe {
        let stat = with_openat(
            AT_FDCWD,
            filename.to_bytes().to_vec(),
            OpenFlags::O_PATH,
            AtFlags::AT_SYMLINK_NOFOLLOW,
            0,
            |fd| rtenv::fs::fstat(fd, StatxMask::all()),
        )?;
        statbuf.write(stat.into());
        Ok(())
    }
}

#[syscall]
pub unsafe fn sys_statx(
    dfd: c_int,
    filename: &CStr,
    flags: AtFlags,
    _mask: u32, // TODO
    buf: *mut Statx,
) -> Result<(), LxError> {
    unsafe {
        let statx = with_openat(
            dfd,
            filename.to_bytes().to_vec(),
            OpenFlags::O_PATH,
            flags,
            0,
            |fd| rtenv::fs::fstat(fd, StatxMask::all()),
        )?;
        buf.write(statx);
        Ok(())
    }
}

#[syscall]
pub unsafe fn sys_truncate(path: &CStr, len: u64) -> Result<(), LxError> {
    let fd = rtenv::fs::openat(
        AT_FDCWD,
        path.to_bytes().to_vec(),
        OpenFlags::O_WRONLY,
        AtFlags::empty(),
        FileMode(0),
    )?;
    let result = rtenv::io::truncate(fd, len);
    _ = rtenv::io::close(fd);
    result
}

#[syscall]
pub unsafe fn sys_readlink(path: &CStr, buf: *mut c_char, bufsiz: usize) -> Result<usize, LxError> {
    unsafe {
        let result = with_openat(
            AT_FDCWD,
            path.to_bytes().to_vec(),
            OpenFlags::empty(),
            AtFlags::AT_SYMLINK_NOFOLLOW,
            0,
            rtenv::fs::freadlink,
        )?;
        crate::util::ret_buf(&result, buf.cast(), bufsiz)
    }
}

#[syscall]
pub unsafe fn sys_readlinkat(
    dfd: c_int,
    filename: &CStr,
    buf: *mut c_char,
    bufsiz: usize,
) -> Result<usize, LxError> {
    unsafe {
        let result = with_openat(
            -dfd,
            filename.to_bytes().to_vec(),
            OpenFlags::empty(),
            AtFlags::AT_SYMLINK_NOFOLLOW,
            0,
            rtenv::fs::freadlink,
        )?;
        crate::util::ret_buf(&result, buf.cast(), bufsiz)
    }
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
pub unsafe fn sys_chdir(path: &CStr) -> Result<(), LxError> {
    with_openat(
        AT_FDCWD,
        path.to_bytes().to_vec(),
        OpenFlags::O_PATH | OpenFlags::O_DIRECTORY,
        AtFlags::empty(),
        0,
        rtenv::fs::fchdir,
    )
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
pub unsafe fn sys_symlink(src: &CStr, dst: &CStr) -> Result<(), LxError> {
    rtenv::fs::symlinkat(src.to_bytes().to_vec(), AT_FDCWD, dst.to_bytes().to_vec())
}

#[syscall]
pub unsafe fn sys_symlinkat(src: &CStr, newdfd: c_int, dst: &CStr) -> Result<(), LxError> {
    rtenv::fs::symlinkat(src.to_bytes().to_vec(), newdfd, dst.to_bytes().to_vec())
}

#[syscall]
pub unsafe fn sys_rename(src: &CStr, dst: &CStr) -> Result<(), LxError> {
    rtenv::fs::renameat2(
        AT_FDCWD,
        src.to_bytes().to_vec(),
        AT_FDCWD,
        dst.to_bytes().to_vec(),
        0,
    )
}

#[syscall]
pub unsafe fn sys_renameat2(
    srcdfd: c_int,
    src: &CStr,
    dstdfd: c_int,
    dst: &CStr,
    flags: u32,
) -> Result<(), LxError> {
    rtenv::fs::renameat2(
        srcdfd,
        src.to_bytes().to_vec(),
        dstdfd,
        dst.to_bytes().to_vec(),
        flags,
    )
}

#[syscall]
pub unsafe fn sys_link(src: &CStr, dst: &CStr) -> Result<(), LxError> {
    rtenv::fs::linkat(
        AT_FDCWD,
        src.to_bytes().to_vec(),
        AT_FDCWD,
        dst.to_bytes().to_vec(),
        AtFlags::empty(),
    )
}

#[syscall]
pub unsafe fn sys_linkat(
    sdfd: c_int,
    src: &CStr,
    ddfd: c_int,
    dst: &CStr,
    flags: AtFlags,
) -> Result<(), LxError> {
    rtenv::fs::linkat(
        sdfd,
        src.to_bytes().to_vec(),
        ddfd,
        dst.to_bytes().to_vec(),
        flags,
    )
}

#[syscall]
pub unsafe fn sys_mkdir(path: &CStr, mode: u32) -> Result<(), LxError> {
    rtenv::fs::mkdirat(AT_FDCWD, path.to_bytes().to_vec(), FileMode(mode as _))
}

#[syscall]
pub unsafe fn sys_mkdirat(dfd: c_int, path: &CStr, mode: u32) -> Result<(), LxError> {
    rtenv::fs::mkdirat(dfd, path.to_bytes().to_vec(), FileMode(mode as _))
}

#[syscall]
pub unsafe fn sys_mknodat(
    dfd: c_int,
    path: &CStr,
    mode: u32,
    dev: DeviceNumber,
) -> Result<(), LxError> {
    rtenv::fs::mknodat(dfd, path.to_bytes().to_vec(), FileMode(mode as _), dev)
}

#[syscall]
pub unsafe fn sys_unlink(path: &CStr) -> Result<(), LxError> {
    rtenv::fs::unlinkat(AT_FDCWD, path.to_bytes().to_vec(), AtFlags::empty())
}

#[syscall]
pub unsafe fn sys_unlinkat(dfd: c_int, path: &CStr, flags: AtFlags) -> Result<(), LxError> {
    rtenv::fs::unlinkat(dfd, path.to_bytes().to_vec(), flags)
}

#[syscall]
pub unsafe fn sys_rmdir(path: &CStr) -> Result<(), LxError> {
    rtenv::fs::unlinkat(AT_FDCWD, path.to_bytes().to_vec(), AtFlags::AT_REMOVEDIR)
}

#[syscall]
pub unsafe fn sys_chown(path: &CStr, uid: u32, gid: u32) -> Result<(), LxError> {
    unsafe {
        with_openat(
            AT_FDCWD,
            path.to_bytes().to_vec(),
            OpenFlags::O_PATH,
            AtFlags::empty(),
            0,
            |fd| rtenv::fs::fchown(fd, uid, gid),
        )
    }
}

#[syscall]
pub unsafe fn sys_lchown(path: &CStr, uid: u32, gid: u32) -> Result<(), LxError> {
    unsafe {
        with_openat(
            AT_FDCWD,
            path.to_bytes().to_vec(),
            OpenFlags::O_PATH,
            AtFlags::AT_SYMLINK_NOFOLLOW,
            0,
            |fd| rtenv::fs::fchown(fd, uid, gid),
        )
    }
}

#[syscall]
pub unsafe fn sys_fchown(fd: c_int, uid: u32, gid: u32) -> Result<(), LxError> {
    unsafe { rtenv::fs::fchown(fd, uid, gid) }
}

#[syscall]
pub unsafe fn sys_chmod(path: &CStr, mode: u16) -> Result<(), LxError> {
    unsafe {
        with_openat(
            AT_FDCWD,
            path.to_bytes().to_vec(),
            OpenFlags::O_PATH,
            AtFlags::empty(),
            0,
            |fd| rtenv::fs::fchmod(fd, mode),
        )
    }
}

#[syscall]
pub unsafe fn sys_fchmod(fd: c_int, mode: u16) -> Result<(), LxError> {
    unsafe { rtenv::fs::fchmod(fd, mode) }
}

#[syscall]
pub unsafe fn sys_utimensat(
    dfd: c_int,
    path: Option<&CStr>,
    times: Option<NonNull<[Timespec; 2]>>,
    flags: AtFlags,
) -> Result<(), LxError> {
    unsafe {
        let mut flags = flags;
        let path = match path {
            Some(val) => val,
            None => {
                flags |= AtFlags::AT_EMPTY_PATH;
                c""
            }
        };
        let times = match times {
            Some(x) => x.read(),
            None => [Timespec::now(), Timespec::now()],
        };
        with_openat(
            dfd,
            path.to_bytes().to_vec(),
            OpenFlags::O_PATH,
            flags | AtFlags::AT_EMPTY_PATH,
            0,
            |fd| rtenv::fs::futimens(fd, times),
        )
    }
}

#[syscall]
pub unsafe fn sys_listxattr(path: &CStr, list: *mut u8, size: usize) -> Result<usize, LxError> {
    unsafe {
        with_openat(
            AT_FDCWD,
            path.to_bytes().to_vec(),
            OpenFlags::O_PATH,
            AtFlags::empty(),
            0,
            |fd| crate::util::ret_buf(&rtenv::fs::flistxattr(fd)?, list, size),
        )
    }
}

#[syscall]
pub unsafe fn sys_llistxattr(path: &CStr, list: *mut u8, size: usize) -> Result<usize, LxError> {
    unsafe {
        with_openat(
            AT_FDCWD,
            path.to_bytes().to_vec(),
            OpenFlags::O_PATH,
            AtFlags::AT_SYMLINK_NOFOLLOW,
            0,
            |fd| crate::util::ret_buf(&rtenv::fs::flistxattr(fd)?, list, size),
        )
    }
}

#[syscall]
pub unsafe fn sys_flistxattr(fd: c_int, list: *mut u8, size: usize) -> Result<usize, LxError> {
    unsafe { crate::util::ret_buf(&rtenv::fs::flistxattr(fd)?, list, size) }
}

#[syscall]
pub unsafe fn sys_umount2(path: &CStr, flags: UmountFlags) -> Result<(), LxError> {
    rtenv::fs::umount(path.to_bytes().to_vec(), flags)
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
pub unsafe fn sys_write(fd: c_int, buf: *const u8, count: usize) -> Result<usize, LxError> {
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
pub unsafe fn sys_lseek(fd: c_int, off: i64, whence: Whence) -> Result<i64, LxError> {
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
pub unsafe fn sys_dup3(old: c_int, new: c_int, flags: OpenFlags) -> Result<c_int, LxError> {
    rtenv::io::dup3(old, new, flags)
}

#[syscall]
pub unsafe fn sys_fsync(fd: c_int) -> Result<(), LxError> {
    rtenv::io::fsync(fd)
}

#[syscall]
pub unsafe fn sys_sync_file_range(
    fd: c_int,
    _off: i64,
    _nbytes: u64,
    _flags: u32,
) -> Result<(), LxError> {
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
pub unsafe fn sys_close_range(
    first: c_int,
    last: c_int,
    flags: CloseRangeFlags,
) -> Result<(), LxError> {
    if first > last {
        return Err(LxError::EINVAL);
    }
    let mut open_fds = Vec::new();
    for entry in std::fs::read_dir("/dev/fd")? {
        let entry = entry?;
        let Ok(filename) = entry.file_name().into_string() else {
            continue;
        };
        let Ok(fd) = filename.parse::<c_int>() else {
            continue;
        };
        open_fds.push(fd);
    }
    for fd in open_fds {
        if (first..=last).contains(&fd) {
            if flags.contains(CloseRangeFlags::CLOSE_RANGE_CLOEXEC) {
                _ = rtenv::io::set_cloexec(fd);
            } else {
                _ = rtenv::io::close(fd);
            }
        }
    }
    Ok(())
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
pub unsafe fn sys_syslog(
    action: SyslogAction,
    buf: *mut u8,
    bufsiz: c_int,
) -> Result<usize, LxError> {
    unsafe {
        let buf = || std::slice::from_raw_parts_mut(buf, bufsiz as _);
        match action {
            SyslogAction::SYSLOG_ACTION_READ_ALL => rtenv::misc::read_syslog_all(buf()),
            _ => Err(LxError::EINVAL),
        }
    }
}

#[syscall]
pub unsafe fn sys_sethostname(name: *const c_char, len: usize) -> Result<(), LxError> {
    let domainname = rtenv::misc::get_network_names()?.domainname;
    unsafe {
        let nodename = std::slice::from_raw_parts(name.cast::<u8>(), len).to_vec();
        rtenv::misc::set_network_names(NetworkNames {
            nodename,
            domainname,
        })?;
        Ok(())
    }
}

#[syscall]
pub unsafe fn sys_setdomainname(name: *const c_char, len: usize) -> Result<(), LxError> {
    let nodename = rtenv::misc::get_network_names()?.nodename;
    unsafe {
        let domainname = std::slice::from_raw_parts(name.cast::<u8>(), len).to_vec();
        rtenv::misc::set_network_names(NetworkNames {
            nodename,
            domainname,
        })?;
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
pub unsafe fn sys_getsockopt(
    sock: c_int,
    level: SockOptLevel,
    opt: u32,
    ptr: *mut u8,
    len: *mut c_int,
) -> Result<(), LxError> {
    unsafe {
        // TODO length
        let buf = std::slice::from_raw_parts_mut(ptr, len.read() as usize);
        rtenv::net::getsockopt(sock, level, opt, buf)
    }
}

#[syscall]
pub unsafe fn sys_setsockopt(
    sock: c_int,
    level: SockOptLevel,
    opt: u32,
    ptr: *const u8,
    len: c_int,
) -> Result<(), LxError> {
    unsafe {
        let buf = std::slice::from_raw_parts(ptr, len as usize);
        rtenv::net::setsockopt(sock, level, opt, buf)
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
) -> Result<*mut u8, LxError> {
    unsafe { rtenv::mm::map(addr, len, prot, flags, fd, offset) }
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
pub unsafe fn sys_mlock(addr: *mut u8, len: usize) -> Result<(), LxError> {
    unsafe {
        match libc::mlock(addr.cast(), len) {
            -1 => Err(LxError::last_apple_error()),
            _ => Ok(()),
        }
    }
}

#[syscall]
pub unsafe fn sys_munlock(addr: *mut u8, len: usize) -> Result<(), LxError> {
    unsafe {
        match libc::munlock(addr.cast(), len) {
            -1 => Err(LxError::last_apple_error()),
            _ => Ok(()),
        }
    }
}

#[syscall]
pub unsafe fn sys_msync(addr: *mut u8, len: usize, flags: MsyncFlags) -> Result<(), LxError> {
    unsafe {
        match libc::msync(addr.cast(), len, flags.to_apple()?) {
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
pub unsafe fn sys_sigaltstack(
    new_ptr: Option<NonNull<SigAltStack>>,
    old_ptr: Option<NonNull<SigAltStack>>,
) {
    unsafe {
        let new = new_ptr.map(|x| x.read());
        let old = rtenv::signal::sigaltstack(new);
        if let Some(old_ptr) = old_ptr {
            old_ptr.write(old);
        }
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
    // TODO: replace with setitimer if `alarm` is deprecated?
    unsafe { libc::alarm(secs) }
}

#[syscall]
pub unsafe fn sys_times(tms: *mut Tms) -> Result<i64, LxError> {
    unsafe {
        let mut apple = std::mem::zeroed();
        match libc::times(&mut apple) as i64 {
            -1 => Err(LxError::last_apple_error()),
            other => {
                tms.write(Tms::from_apple(apple)?);
                Ok(other)
            }
        }
    }
}

#[syscall]
pub unsafe fn sys_nanosleep(rqtp: *const Timespec, rmtp: *mut Timespec) -> Result<(), LxError> {
    unsafe {
        let rqtp = rqtp.read().to_apple()?;
        let mut rmtp_buf = std::mem::zeroed();
        match libc::nanosleep(&rqtp, &mut rmtp_buf) {
            -1 => Err(LxError::last_apple_error()),
            _ => {
                rmtp.write(Timespec::from_apple(rmtp_buf)?);
                Ok(())
            }
        }
    }
}

#[syscall]
pub unsafe fn sys_clock_nanosleep(
    clock: ClockId,
    flags: TimerFlags,
    rqtp: *const Timespec,
    rmtp: Option<NonNull<Timespec>>,
) -> Result<(), LxError> {
    unsafe {
        let mut rqtp = rqtp.read().to_apple()?;
        if flags.contains(TimerFlags::TIMER_ABSTIME) {
            let mut now = std::mem::zeroed();
            if libc::clock_gettime(clock.to_apple()?, &mut now) == -1 {
                return Err(LxError::last_apple_error());
            }
            if rqtp.tv_sec < now.tv_sec
                || (rqtp.tv_sec == now.tv_sec && rqtp.tv_nsec <= now.tv_nsec)
            {
                if let Some(rmtp) = rmtp {
                    rmtp.write(Timespec {
                        tv_sec: 0,
                        tv_nsec: 0,
                    });
                }
                return Ok(());
            }
            rqtp.tv_sec -= now.tv_sec;
            rqtp.tv_nsec -= now.tv_nsec;
        }
        let mut rmtp_buf = std::mem::zeroed();
        match libc::nanosleep(&rqtp, &mut rmtp_buf) {
            -1 => Err(LxError::last_apple_error()),
            _ => {
                if let Some(rmtp) = rmtp {
                    rmtp.write(Timespec::from_apple(rmtp_buf)?);
                }
                Ok(())
            }
        }
    }
}

#[syscall]
pub unsafe fn sys_clock_gettime(clk_id: ClockId, tp: *mut Timespec) -> Result<(), LxError> {
    unsafe {
        let mut apple_tp = tp.read().to_apple()?;
        match libc::clock_gettime(clk_id.to_apple()?, &mut apple_tp) {
            -1 => Err(LxError::last_apple_error()),
            _ => {
                tp.write(Timespec::from_apple(apple_tp)?);
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
                    tv.write(Timeval::from_apple(tvbuf)?);
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
pub unsafe fn sys_getpgid(pid: i32) -> Result<c_int, LxError> {
    rtenv::process::pgid(pid)
}

#[syscall]
pub unsafe fn sys_getpgrp() -> Result<c_int, LxError> {
    rtenv::process::pgid(rtenv::process::pid())
}

#[syscall]
pub unsafe fn sys_setpgid(pid: i32, pgid: i32) -> Result<(), LxError> {
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
    path: &CStr,
    argv: *const *const u8,
    envp: *const *const u8,
) -> Result<(), LxError> {
    unsafe {
        let mut argc = 0;
        let mut envc = 0;

        while !(*argv.add(argc)).is_null() {
            argc += 1;
        }

        while !(*envp.add(envc)).is_null() {
            envc += 1;
        }

        rtenv::process::exec(
            path.to_bytes(),
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
    stat_addr: Option<NonNull<WaitStatus>>,
    options: WaitOptions,
    ru: Option<NonNull<RUsage>>,
) -> Result<i32, LxError> {
    unsafe {
        let mut status = 0;
        let mut apple_ru = std::mem::zeroed();
        let pid = match libc::wait4(pid, &mut status, options.to_apple()?, &mut apple_ru) {
            -1 => Err(LxError::last_apple_error()),
            n => Ok(n),
        }?;
        if let Some(stat_addr) = stat_addr {
            stat_addr.write(WaitStatus::from_apple(status));
        }
        if let Some(ru) = ru {
            ru.write(RUsage::from_apple(apple_ru)?);
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
        rusage.write(RUsage::from_apple(buf)?);
        Ok(())
    }
}

#[syscall]
pub unsafe fn sys_prctl(
    op: PrctlOp,
    arg0: usize,
    _arg1: usize,
    _arg2: usize,
    _arg3: usize,
    _arg4: usize,
) -> Result<(), LxError> {
    match op {
        PrctlOp::PR_SET_NAME => unsafe {
            rtenv::thread::set_name((arg0 as *const [u8; 16]).read());
            Ok(())
        },
        PrctlOp::PR_GET_NAME => unsafe {
            (arg0 as *mut [u8; 16]).write(rtenv::thread::get_name());
            Ok(())
        },
        PrctlOp::PR_GET_TID_ADDRESS => unsafe {
            (arg0 as *mut Option<NonNull<u32>>).write(rtenv::thread::get_clear_tid());
            Ok(())
        },
        _ => Err(LxError::EINVAL),
    }
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
    let cpus = std::thread::available_parallelism()
        .map(NonZero::get)
        .unwrap_or(8);
    let min_size = cpus.div_ceil(size_of::<u8>());
    let last = 0xff << (cpus % size_of::<u8>());
    if cpusetsize < min_size {
        return Err(LxError::EINVAL);
    }
    unsafe {
        cpuset.write_bytes(0, cpusetsize);
        cpuset.write_bytes(0xff, min_size - 1);
        cpuset.add(min_size - 1).write(last);
    }
    Ok(())
}

#[syscall]
pub unsafe fn sys_sched_setaffinity(
    _pid: i32,
    _cpusetsize: usize,
    _cpuset: *const u8,
) -> Result<(), LxError> {
    // TODO: We cannot really implement it on macOS.
    Err(LxError::EPERM)
}

// -== Multi-user Support ==-

#[syscall]
pub unsafe fn sys_setuid(uid: u32) -> Result<(), LxError> {
    rtenv::security::setuid(uid)
}

#[syscall]
pub unsafe fn sys_setgid(gid: u32) -> Result<(), LxError> {
    rtenv::security::setgid(gid)
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

#[syscall]
pub unsafe fn sys_swapon(_dev: *const c_char, _flags: c_int) -> Result<(), LxError> {
    Err(LxError::EPERM)
}

#[syscall]
pub unsafe fn sys_swapoff(_dev: *const c_char) -> Result<(), LxError> {
    Err(LxError::EPERM)
}

pub unsafe fn sys_invalid(uctx: &mut libc::ucontext_t) {
    unsafe {
        rtenv::emuctx::leave_emulated();
        if rtenv::switches::ignore_unsupported_syscalls() {
            log::warn!("ignored unsupported syscall {}", uctx.sysno());
            uctx.ret(-(LxError::ENOSYS.0 as isize) as usize);
            rtenv::emuctx::enter_emulated();
            return;
        }

        eprintln!("Bad system call");
        log::error!(
            "process crashed due to unsupported syscall {}",
            uctx.sysno()
        );
        rtenv::error_report::fast_fail();
    }
}
