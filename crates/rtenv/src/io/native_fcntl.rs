use crate::posix_num;
use libc::c_int;
use structures::{
    error::LxError,
    fs::OpenFlags,
    io::{FcntlCmd, FdFlags, Flock}, FromApple, ToApple,
};

pub fn native_fcntl(fd: c_int, cmd: FcntlCmd, arg: usize) -> Result<c_int, LxError> {
    match cmd {
        FcntlCmd::F_DUPFD => unsafe { posix_num!(libc::fcntl(fd, libc::F_DUPFD, arg)) },
        FcntlCmd::F_GETFD => unsafe {
            posix_num!(libc::fcntl(fd, libc::F_GETFD)).and_then(FdFlags::from_apple).map(|x| x.bits() as _)
        },
        FcntlCmd::F_SETFD => unsafe {
            posix_num!(libc::fcntl(
                fd,
                libc::F_SETFD,
                FdFlags::from_bits_retain(arg as u32).to_apple()?
            ))
        },
        FcntlCmd::F_GETFL => unsafe {
            posix_num!(libc::fcntl(fd, libc::F_GETFL)).and_then(OpenFlags::from_apple).map(|x| x.bits() as _)
        },
        FcntlCmd::F_SETFL => unsafe {
            posix_num!(libc::fcntl(
                fd,
                libc::F_SETFL,
                OpenFlags::from_bits_retain(arg as u32).to_apple()
            ))
        },
        FcntlCmd::F_GETLK => unsafe {
            let mut flock_apple: libc::flock = std::mem::zeroed();
            let n = posix_num!(libc::fcntl(fd, libc::F_SETLK, &mut flock_apple))?;
            (arg as *mut Flock).write(Flock::from_apple(flock_apple)?);
            Ok(n)
        },
        FcntlCmd::F_SETLK => unsafe {
            let flock = (arg as *mut Flock).read();
            let mut flock_apple = flock.to_apple()?;
            posix_num!(libc::fcntl(fd, libc::F_SETLK, &mut flock_apple))
        },
        FcntlCmd::F_SETLKW => unsafe {
            let flock = (arg as *mut Flock).read();
            let mut flock_apple = flock.to_apple()?;
            posix_num!(libc::fcntl(fd, libc::F_SETLKW, &mut flock_apple))
        },
        FcntlCmd::F_DUPFD_CLOEXEC => unsafe {
            posix_num!(libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, arg))
        },
        _ => Err(LxError::EINVAL),
    }
}
