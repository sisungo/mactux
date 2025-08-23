use libc::c_int;
use structures::{error::LxError, net::SockOpt};

macro_rules! sockopt_impl {
    ($linux:ty, $apple:ty, $from:ident, $to:ident) => {
        unsafe {
            (
                |fd, level, sockopt, buf| {
                    if buf.len() < size_of::<$linux>() {
                        return Err(LxError::EINVAL);
                    }
                    let mut apple: $apple = std::mem::zeroed();
                    let mut apple_size = size_of::<$apple>() as _;
                    let result = libc::getsockopt(
                        fd,
                        level,
                        sockopt,
                        (&raw mut apple).cast(),
                        &mut apple_size,
                    );
                    buf.as_mut_ptr()
                        .cast::<$linux>()
                        .write(<$linux>::$from(apple));
                    $crate::posix_bi!(result)
                },
                |fd, level, sockopt, buf| {
                    if buf.len() < size_of::<$linux>() {
                        return Err(LxError::EINVAL);
                    }
                    let linux = buf.as_ptr().cast::<$linux>().read();
                    let apple: $apple = linux.$to();
                    $crate::posix_bi!(libc::setsockopt(
                        fd,
                        level,
                        sockopt,
                        (&raw const apple).cast(),
                        size_of::<$apple>() as _
                    ))
                },
            )
        }
    };
    (invalid) => {
        (
            |_, _, _, _| Err(LxError::EINVAL),
            |_, _, _, _| Err(LxError::EINVAL),
        )
    };
}

type SockOptGetFn = unsafe fn(c_int, c_int, c_int, &mut [u8]) -> Result<(), LxError>;
type SockOptSetFn = unsafe fn(c_int, c_int, c_int, &[u8]) -> Result<(), LxError>;

pub trait SockOptLevel {
    fn impls() -> &'static [(SockOptGetFn, SockOptSetFn)];
    fn apple() -> c_int;
}

#[derive(Debug, Clone, Copy)]
pub struct SocketLevel;
impl SockOptLevel for SocketLevel {
    fn impls() -> &'static [(SockOptGetFn, SockOptSetFn)] {
        &[
            sockopt_impl!(invalid),                  // 0
            sockopt_impl!(c_int, c_int, from, into), // 1
        ]
    }

    fn apple() -> c_int {
        libc::SOL_SOCKET
    }
}

pub fn get<L: SockOptLevel>(fd: c_int, sockopt: SockOpt, buf: &mut [u8]) -> Result<(), LxError> {
    unsafe {
        L::impls().get(sockopt.0 as usize).ok_or(LxError::EINVAL)?.0(
            fd,
            L::apple(),
            sockopt.to_apple()?,
            buf,
        )
    }
}

pub fn set<L: SockOptLevel>(fd: c_int, sockopt: SockOpt, buf: &[u8]) -> Result<(), LxError> {
    unsafe {
        L::impls().get(sockopt.0 as usize).ok_or(LxError::EINVAL)?.1(
            fd,
            L::apple(),
            sockopt.to_apple()?,
            buf,
        )
    }
}
