use libc::c_int;
use structures::{
    error::LxError, net::{Linger, SockOpt, SocketKind}, time::Timeval, ToApple
};

macro_rules! sockopt_impl {
    (readonly $linux:ty, $apple:ty) => {
        (
            sockopt_impl!(@getter :: $linux, $apple),
            |_, _, _, _| Err(LxError::EINVAL),
        )
    };
    ($linux:ty, $apple:ty) => {
        (
            sockopt_impl!(@getter :: $linux, $apple),
            sockopt_impl!(@setter :: $linux, $apple),
        )
    };
    (invalid) => {
        (
            |_, _, _, _| Err(LxError::EINVAL),
            |_, _, _, _| Err(LxError::EINVAL),
        )
    };
    (@getter :: $linux:ty, $apple:ty) => {
        |fd, level, sockopt, buf| unsafe {
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
                .write(::structures::FromApple::from_apple(apple)?);
            $crate::posix_bi!(result)
        }
    };
    (@setter :: $linux:ty, $apple:ty) => {
        |fd, level, sockopt, buf| unsafe {
            if buf.len() < size_of::<$linux>() {
                return Err(LxError::EINVAL);
            }
            let linux = buf.as_ptr().cast::<$linux>().read();
            let apple: $apple = ::structures::ToApple::to_apple(linux)?;
            $crate::posix_bi!(libc::setsockopt(
                fd,
                level,
                sockopt,
                (&raw const apple).cast(),
                size_of::<$apple>() as _
            ))
        }
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
            sockopt_impl!(invalid),                    // 0
            sockopt_impl!(c_int, c_int),               // 1
            sockopt_impl!(c_int, c_int),               // 2
            sockopt_impl!(readonly SocketKind, c_int), // 3
            sockopt_impl!(readonly LxError, c_int),    // 4
            sockopt_impl!(c_int, c_int),               // 5
            sockopt_impl!(c_int, c_int),               // 6
            sockopt_impl!(c_int, c_int),               // 7
            sockopt_impl!(c_int, c_int),               // 8
            sockopt_impl!(c_int, c_int),               // 9
            sockopt_impl!(c_int, c_int),               // 10
            sockopt_impl!(invalid),                    // 11
            sockopt_impl!(invalid),                    // 12
            sockopt_impl!(Linger, libc::linger),       // 13
            sockopt_impl!(invalid),                    // 14
            sockopt_impl!(c_int, c_int),               // 15
            sockopt_impl!(invalid),                    // 16
            sockopt_impl!(invalid),                    // 17
            sockopt_impl!(c_int, c_int),               // 18
            sockopt_impl!(c_int, c_int),               // 19
            sockopt_impl!(Timeval, libc::timeval),     // 20
            sockopt_impl!(Timeval, libc::timeval),     // 21
            sockopt_impl!(invalid),                    // 22
            sockopt_impl!(invalid),                    // 23
            sockopt_impl!(invalid),                    // 24
            sockopt_impl!(invalid),                    // 25
            sockopt_impl!(invalid),                    // 26
            sockopt_impl!(invalid),                    // 27
            sockopt_impl!(invalid),                    // 28
            sockopt_impl!(c_int, c_int),               // 29
            sockopt_impl!(readonly c_int, c_int),      // 30
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
