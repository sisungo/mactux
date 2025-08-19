//! Structures and definitions of Linux types, along with utilities converting them from and to the apple ones.

pub mod convention;
pub mod error;
pub mod fs;
pub mod io;
pub mod mapper;
pub mod misc;
pub mod mm;
pub mod net;
pub mod process;
pub mod security;
pub mod signal;
pub mod sync;
pub mod terminal;
pub mod thread;
pub mod time;
pub mod ucontext;

#[macro_export]
macro_rules! bitflags_impl_to_apple {
    ($self:ident = $($x:ident),*) => {{
        let mut apple = 0;
        $(
            if $self.contains(Self::$x) {
                apple |= libc::$x;
            }
        )*
        apple
    }};
}

#[macro_export]
macro_rules! bitflags_impl_from_apple {
    ($apple:ident = $($x:ident),*) => {{
        let mut linux = Self::empty();
        $(
            if ($apple & libc::$x) != 0 {
                linux |= Self::$x;
            }
        )*
        linux
    }};
}

#[macro_export]
macro_rules! bitflags_impl_from_to_apple_permissive {
    (type Apple = $ty:ty; values = $($x:ident),*) => {
        pub fn from_apple(apple: $ty) -> Self {
            $crate::bitflags_impl_from_apple!(apple = $($x),*)
        }

        pub fn to_apple(self) -> $ty {
            $crate::bitflags_impl_to_apple!(self = $($x),*)
        }
    };
}

#[macro_export]
macro_rules! unixvariants {
    {
        $(#[$outer:meta])*
        $v:vis struct $n:ident: $t:ty {
            $(const $j:ident = $k:expr;)*
            $(#[linux_only] const $h:ident = $i:expr;)*
            $(#[apple = $an:ident] const $l:ident = $m:expr;)*

            fn from_apple($_:ident: $ati:ty) -> Result<Self, LxError>;
            fn to_apple(self) -> Result<$ato:ty, LxError>;
        }
    } => {
        $(#[$outer])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        $v struct $n(pub $t);
        impl $n {
            $(
                pub const $h: Self = Self($i);
            )*
            $(
                pub const $j: Self = Self($k);
            )*
            $(
                pub const $l: Self = Self($m);
            )*

            #[allow(unreachable_patterns)]
            pub const fn from_apple(apple: $ati) -> Result<Self, $crate::error::LxError> {
                match apple {
                    $(libc::$j => Ok(Self::$j),)*
                    $(libc::$an => Ok(Self::$l),)*
                    _ => Err($crate::error::LxError::EINVAL),
                }
            }

            pub const fn to_apple(self) -> Result<$ato, $crate::error::LxError> {
                match self {
                    $(Self::$j => Ok(libc::$j),)*
                    $(Self::$l => Ok(libc::$an),)*
                    _ => Err($crate::error::LxError::EINVAL),
                }
            }
        }
    };
}
