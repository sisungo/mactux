pub mod convention;
pub mod error;
pub mod fs;
pub mod io;
pub mod mapper;
pub mod misc;
pub mod mm;
pub mod net;
pub mod process;
pub mod signal;
pub mod sync;
pub mod terminal;
pub mod thread;
pub mod time;
pub mod ucontext;

#[macro_export]
macro_rules! newtype_impl_to_apple {
    ($self:ident = $($x:ident),*) => {
        #[allow(unreachable_patterns)]
        match $self {
            $(Self::$x => Some(libc::$x),)*
            _ => None,
        }
    };
}

#[macro_export]
macro_rules! newtype_impl_from_apple {
    ($apple:ident = $($x:ident),*) => {
        #[allow(unreachable_patterns)]
        match $apple {
            $(libc::$x => Some(Self::$x),)*
            _ => None,
        }
    };
}

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
