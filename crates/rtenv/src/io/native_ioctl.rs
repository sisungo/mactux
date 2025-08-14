use crate::{posix_bi, posix_num};
use libc::c_int;
use structures::{
    error::LxError,
    io::IoctlCmd,
    terminal::{TcFlowAction, Termios, Termios2, WinSize},
};

pub fn native_ioctl(fd: c_int, cmd: IoctlCmd, arg: *mut u8) -> Result<c_int, LxError> {
    match cmd {
        IoctlCmd::TIOCGPGRP => unsafe {
            let apple_pid: libc::pid_t = posix_num!(libc::tcgetpgrp(fd))?;
            arg.cast::<i32>().write(apple_pid);
            Ok(0)
        },
        IoctlCmd::TIOCSPGRP => unsafe {
            let apple_pid: libc::pid_t = arg.cast::<i32>().read();
            posix_bi!(libc::tcsetpgrp(fd, apple_pid))?;
            Ok(0)
        },
        IoctlCmd::TCGETS => unsafe {
            let mut apple_termios: libc::termios = std::mem::zeroed();
            posix_bi!(libc::tcgetattr(fd, &mut apple_termios))?;
            arg.cast::<Termios>().write(apple_termios.into());
            Ok(0)
        },
        IoctlCmd::TCSETS => unsafe {
            let apple_termios = arg.cast::<Termios>().read().to_apple();
            posix_bi!(libc::tcsetattr(fd, libc::TCSANOW, &apple_termios))?;
            Ok(0)
        },
        IoctlCmd::TCSETSW => unsafe {
            let apple_termios = arg.cast::<Termios>().read().to_apple();
            posix_bi!(libc::tcsetattr(fd, libc::TCSADRAIN, &apple_termios))?;
            Ok(0)
        },
        IoctlCmd::TCSETSF => unsafe {
            let apple_termios = arg.cast::<Termios>().read().to_apple();
            posix_bi!(libc::tcsetattr(fd, libc::TCSAFLUSH, &apple_termios))?;
            Ok(0)
        },
        IoctlCmd::TIOCGWINSZ => unsafe {
            let mut winsize: libc::winsize = std::mem::zeroed();
            posix_bi!(libc::ioctl(fd, libc::TIOCGWINSZ, &mut winsize))?;
            arg.cast::<WinSize>().write(winsize.into());
            Ok(0)
        },
        IoctlCmd::TIOCSWINSZ => unsafe {
            let winsize = arg.cast::<WinSize>().read().to_apple();
            posix_bi!(libc::ioctl(fd, libc::TIOCSWINSZ, &winsize))?;
            Ok(0)
        },
        IoctlCmd::TCGETS2 => unsafe {
            let mut apple_termios: libc::termios = std::mem::zeroed();
            posix_bi!(libc::tcgetattr(fd, &mut apple_termios))?;
            arg.cast::<Termios2>().write(apple_termios.into());
            Ok(0)
        },
        IoctlCmd::TCSETS2 => unsafe {
            let apple_termios = arg.cast::<Termios2>().read().to_apple();
            posix_bi!(libc::tcsetattr(fd, libc::TCSANOW, &apple_termios))?;
            Ok(0)
        },
        IoctlCmd::TCSETSW2 => unsafe {
            let apple_termios = arg.cast::<Termios2>().read().to_apple();
            posix_bi!(libc::tcsetattr(fd, libc::TCSADRAIN, &apple_termios))?;
            Ok(0)
        },
        IoctlCmd::TCSETSF2 => unsafe {
            let apple_termios = arg.cast::<Termios2>().read().to_apple();
            posix_bi!(libc::tcsetattr(fd, libc::TCSAFLUSH, &apple_termios))?;
            Ok(0)
        },
        IoctlCmd::TCXONC => unsafe {
            let action = TcFlowAction(arg as usize as u32);
            posix_bi!(libc::tcflow(fd, action.to_apple()?))?;
            Ok(0)
        },
        _ => Err(LxError::EINVAL),
    }
}
