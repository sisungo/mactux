use crate::{bitflags_impl_from_to_apple, error::LxError, unixvariants, FromApple, ToApple};
use bitflags::bitflags;

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Termios {
    c_iflag: InputFlags,
    c_oflag: OutputFlags,
    c_cflag: ControlFlags,
    c_lflag: LocalFlags,
    c_line: u8,
    c_cc: ControlCharacters,
}
impl From<Termios2> for Termios {
    #[inline]
    fn from(value: Termios2) -> Self {
        Self {
            c_iflag: value.c_iflag,
            c_oflag: value.c_oflag,
            c_cflag: value.c_cflag,
            c_lflag: value.c_lflag,
            c_line: 0,
            c_cc: value.c_cc,
        }
    }
}
impl FromApple for Termios {
    type Apple = libc::termios;

    fn from_apple(value: libc::termios) -> Result<Self, LxError> {
        Ok(Self::from(Termios2::from_apple(value)?))
    }
}
impl ToApple for Termios {
    type Apple = libc::termios;

    fn to_apple(self) -> Result<libc::termios, LxError> {
        Ok(libc::termios {
            c_iflag: self.c_iflag.to_apple()?,
            c_oflag: self.c_oflag.to_apple()?,
            c_cflag: self.c_cflag.to_apple()?,
            c_lflag: self.c_lflag.to_apple()?,
            c_cc: self.c_cc.to_apple(),
            c_ispeed: 0,
            c_ospeed: 0,
        })
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Termios2 {
    c_iflag: InputFlags,
    c_oflag: OutputFlags,
    c_cflag: ControlFlags,
    c_lflag: LocalFlags,
    c_line: u8,
    c_cc: ControlCharacters,
    c_ispeed: u32,
    c_ospeed: u32,
}
impl ToApple for Termios2 {
    type Apple = libc::termios;

    fn to_apple(self) -> Result<libc::termios, LxError> {
        Ok(libc::termios {
            c_iflag: self.c_iflag.to_apple()?,
            c_oflag: self.c_oflag.to_apple()?,
            c_cflag: self.c_cflag.to_apple()?,
            c_lflag: self.c_lflag.to_apple()?,
            c_cc: self.c_cc.to_apple(),
            c_ispeed: self.c_ispeed as _,
            c_ospeed: self.c_ospeed as _,
        })
    }
}
impl FromApple for Termios2 {
    type Apple = libc::termios;

    fn from_apple(value: libc::termios) -> Result<Self, LxError> {
        Ok(Self {
            c_iflag: InputFlags::from_apple(value.c_iflag)?,
            c_oflag: OutputFlags::from_apple(value.c_oflag)?,
            c_cflag: ControlFlags::from_apple(value.c_cflag)?,
            c_lflag: LocalFlags::from_apple(value.c_lflag)?,
            c_line: 0,
            c_cc: ControlCharacters::from_apple(value.c_cc),
            c_ispeed: value.c_ispeed as _,
            c_ospeed: value.c_ospeed as _,
        })
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct InputFlags: u32 {
        const IGNBRK = 0x1;
        const BRKINT = 0x2;
        const IGNPAR = 0x4;
        const PARMRK = 0x8;
        const INPCK = 0x10;
        const ISTRIP = 0x20;
        const INLCR = 0x40;
        const IGNCR = 0x80;
        const ICRNL = 0x100;
        const IUCLC = 0x200;
        const IXON = 0x400;
        const IXANY = 0x800;
        const IXOFF = 0x1000;
        const IMAXBEL = 0x2000;
        const IUTF8 = 0x4000;
    }
}
bitflags_impl_from_to_apple!(
    InputFlags;
    type Apple = u64;
    values = IGNBRK, BRKINT, IGNPAR, PARMRK, INPCK, ISTRIP, INLCR, IGNCR, ICRNL, IXON, IXANY, IXOFF,
             IMAXBEL, IUTF8
);

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct OutputFlags: u32 {
        const OPOST = 0x1;
        const OLCUC = 0x2;
        const ONLCR = 0x4;
        const OCRNL = 0x8;
        const ONOCR = 0x10;
        const ONLRET = 0x20;
        const OFILL = 0x40;
        const OFDEL = 0x80;
        const NLDLY = 0x100;
        const CRDLY = 0x200;
        const TABDLY = 0x400;
        const BSDLY = 0x800;
        const VTDLY = 0x1000;
        const FFDLY = 0x2000;
    }
}
bitflags_impl_from_to_apple!(
    OutputFlags;
    type Apple = u64;
    values = OPOST, ONLCR, OCRNL, ONOCR, ONLRET, OFILL, OFDEL, NLDLY, CRDLY, TABDLY, BSDLY, VTDLY, FFDLY
);

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct ControlFlags: u32 {
        const CS5 = 0;
        const CS6 = 0o20;
        const CS7 = 0o40;
        const CS8 = 0o60;
        const CBAUD = 0o10017;
        const CBAUDEX = 0o10000;
        const CSTOPB = 0o100;
        const CREAD = 0o200;
        const PARENB = 0o400;
        const PARODD = 0o1000;
        const HUPCL = 0o2000;
        const CLOCAL = 0o4000;
        const CIBAUD = 0o2003600000;
        const CMSPAR = 0o10000000000;
        const CRTSCTS = 0o20000000000;
    }
}
bitflags_impl_from_to_apple!(
    ControlFlags;
    type Apple = u64;
    values = CS5, CS6, CS7, CS8, CSTOPB, CREAD, PARENB, PARODD, HUPCL, CLOCAL, CRTSCTS
);

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct LocalFlags: u32 {
        const ISIG = 0x1;
        const ICANON = 0x2;
        const ECHO = 0x8;
        const ECHOE = 0x10;
        const ECHOK = 0x20;
        const ECHONL = 0x40;
        const NOFLSH = 0x80;
        const TOSTOP = 0x100;
        const ECHOCTL = 0x200;
        const ECHOPRT = 0x400;
        const ECHOKE = 0x800;
        const FLUSHO = 0x1000;
        const PENDIN = 0x2000;
        const IEXTEN = 0x4000;
    }
}
bitflags_impl_from_to_apple!(
    LocalFlags;
    type Apple = u64;
    values = ISIG, ICANON, ECHO, ECHOE, ECHOK, ECHONL, NOFLSH, TOSTOP, ECHOCTL, ECHOPRT, ECHOKE, IEXTEN
);

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct ControlCharacters([ControlCharacter; 19]);
impl ControlCharacters {
    pub const VINTR: usize = 0;
    pub const VQUIT: usize = 1;
    pub const VERASE: usize = 2;
    pub const VKILL: usize = 3;
    pub const VEOF: usize = 4;
    pub const VTIME: usize = 5;
    pub const VMIN: usize = 6;
    pub const VSTART: usize = 8;
    pub const VSTOP: usize = 9;
    pub const VSUSP: usize = 10;
    pub const VEOL: usize = 11;
    pub const VREPRINT: usize = 12;
    pub const VDISCARD: usize = 13;
    pub const VWERASE: usize = 14;
    pub const VLNEXT: usize = 15;
    pub const VEOL2: usize = 16;
}
impl ControlCharacters {
    pub fn from_apple(apple: [u8; 20]) -> Self {
        let mut linux = [ControlCharacter::DISABLED; _];
        linux[Self::VINTR] = ControlCharacter::from_apple(apple[libc::VINTR]);
        linux[Self::VQUIT] = ControlCharacter::from_apple(apple[libc::VQUIT]);
        linux[Self::VERASE] = ControlCharacter::from_apple(apple[libc::VERASE]);
        linux[Self::VKILL] = ControlCharacter::from_apple(apple[libc::VKILL]);
        linux[Self::VEOF] = ControlCharacter::from_apple(apple[libc::VEOF]);
        linux[Self::VTIME] = ControlCharacter::from_apple(apple[libc::VTIME]);
        linux[Self::VMIN] = ControlCharacter::from_apple(apple[libc::VMIN]);
        linux[Self::VSTART] = ControlCharacter::from_apple(apple[libc::VSTART]);
        linux[Self::VSTOP] = ControlCharacter::from_apple(apple[libc::VSTOP]);
        linux[Self::VSUSP] = ControlCharacter::from_apple(apple[libc::VSUSP]);
        linux[Self::VEOL] = ControlCharacter::from_apple(apple[libc::VEOL]);
        linux[Self::VREPRINT] = ControlCharacter::from_apple(apple[libc::VREPRINT]);
        linux[Self::VWERASE] = ControlCharacter::from_apple(apple[libc::VWERASE]);
        linux[Self::VLNEXT] = ControlCharacter::from_apple(apple[libc::VLNEXT]);
        linux[Self::VEOL2] = ControlCharacter::from_apple(apple[libc::VEOL2]);
        Self(linux)
    }

    pub fn to_apple(self) -> [u8; libc::NCCS] {
        let linux = self.0;
        let mut apple = [libc::_POSIX_VDISABLE; _];
        apple[libc::VINTR] = linux[Self::VINTR].to_apple();
        apple[libc::VQUIT] = linux[Self::VQUIT].to_apple();
        apple[libc::VERASE] = linux[Self::VERASE].to_apple();
        apple[libc::VKILL] = linux[Self::VKILL].to_apple();
        apple[libc::VEOF] = linux[Self::VEOF].to_apple();
        apple[libc::VTIME] = linux[Self::VTIME].to_apple();
        apple[libc::VMIN] = linux[Self::VMIN].to_apple();
        apple[libc::VSTART] = linux[Self::VSTART].to_apple();
        apple[libc::VSTOP] = linux[Self::VSTOP].to_apple();
        apple[libc::VSUSP] = linux[Self::VSUSP].to_apple();
        apple[libc::VEOL] = linux[Self::VEOL].to_apple();
        apple[libc::VREPRINT] = linux[Self::VREPRINT].to_apple();
        apple[libc::VWERASE] = linux[Self::VWERASE].to_apple();
        apple[libc::VLNEXT] = linux[Self::VLNEXT].to_apple();
        apple[libc::VEOL2] = linux[Self::VEOL2].to_apple();
        apple[libc::VSTATUS] = b'T' - 0x40;
        apple
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ControlCharacter(u8);
impl ControlCharacter {
    pub const DISABLED: Self = Self(0);

    pub fn from_apple(apple: u8) -> Self {
        match apple {
            libc::_POSIX_VDISABLE => Self::DISABLED,
            other => Self(other),
        }
    }

    pub fn to_apple(self) -> u8 {
        match self {
            Self::DISABLED => libc::_POSIX_VDISABLE,
            other => other.0,
        }
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct WinSize {
    pub ws_row: u16,
    pub ws_col: u16,
    pub ws_xpixel: u16,
    pub ws_ypixel: u16,
}
impl WinSize {
    pub fn to_apple(self) -> libc::winsize {
        libc::winsize {
            ws_row: self.ws_row,
            ws_col: self.ws_col,
            ws_xpixel: self.ws_xpixel,
            ws_ypixel: self.ws_ypixel,
        }
    }
}
impl From<libc::winsize> for WinSize {
    #[inline]
    fn from(value: libc::winsize) -> Self {
        Self {
            ws_row: value.ws_row,
            ws_col: value.ws_col,
            ws_xpixel: value.ws_xpixel,
            ws_ypixel: value.ws_ypixel,
        }
    }
}

unixvariants! {
    pub struct TcFlowAction: u32 {
        const TCOOFF = 0;
        const TCOON = 1;
        const TCIOFF = 2;
        const TCION = 3;
        fn from_apple(apple: libc::c_int) -> Result<Self, LxError>;
        fn to_apple(self) -> Result<libc::c_int, LxError>;
    }
}
