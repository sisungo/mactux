//! Structures and definitions of Linux types, along with utilities converting them from and to the apple ones.

pub mod convention;
pub mod device;
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

/// Converts a value from the Apple platform to the Linux platform.
pub trait FromApple: Sized {
    /// The type of the Apple platform representation.
    type Apple;

    /// Converts a value from the Apple platform to the Linux platform.
    fn from_apple(apple: Self::Apple) -> Result<Self, error::LxError>;
}

/// Converts a value from the Linux platform to the Apple platform.
pub trait ToApple {
    /// The type of the Apple platform representation.
    type Apple;

    /// Converts a value from the Linux platform to the Apple platform.
    fn to_apple(self) -> Result<Self::Apple, error::LxError>;
}

macro_rules! impl_from_to_apple_plain {
    ($t:ty) => {
        impl FromApple for $t {
            type Apple = $t;
            fn from_apple(apple: Self::Apple) -> Result<Self, error::LxError> {
                Ok(apple)
            }
        }
        impl ToApple for $t {
            type Apple = $t;
            fn to_apple(self) -> Result<Self::Apple, error::LxError> {
                Ok(self)
            }
        }
    };
    ($($t:ty),*) => {
        $(impl_from_to_apple_plain!($t);)*
    };
}
impl_from_to_apple_plain!(i8, u8, i32, u32, i64, u64, isize, usize);

#[macro_export]
macro_rules! bitflags_impl_from_to_apple {
    ($lx:ty; type Apple = $ap:ty; values = $($x:ident),*) => {
        impl $crate::FromApple for $lx {
            type Apple = $ap;
            fn from_apple(apple: Self::Apple) -> Result<Self, $crate::error::LxError> {
                let mut linux = Self::empty();
                $(
                    if (apple & libc::$x) != 0 {
                        linux |= Self::$x;
                    }
                )*
                Ok(linux)
            }
        }
        impl $crate::ToApple for $lx {
            type Apple = $ap;
            fn to_apple(self) -> Result<Self::Apple, $crate::error::LxError> {
                let mut apple = 0;
                $(
                    if self.contains(Self::$x) {
                        apple |= libc::$x;
                    }
                )*
                Ok(apple)
            }
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
        }
        impl $crate::FromApple for $n {
            type Apple = $ati;

            #[allow(unreachable_patterns)]
            fn from_apple(apple: $ati) -> Result<Self, $crate::error::LxError> {
                match apple {
                    $(libc::$j => Ok(Self::$j),)*
                    $(libc::$an => Ok(Self::$l),)*
                    _ => Err($crate::error::LxError::EINVAL),
                }
            }
        }
        impl $crate::ToApple for $n {
            type Apple = $ato;

            fn to_apple(self) -> Result<$ato, $crate::error::LxError> {
                match self {
                    $(Self::$j => Ok(libc::$j),)*
                    $(Self::$l => Ok(libc::$an),)*
                    _ => Err($crate::error::LxError::EINVAL),
                }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_bincode_for_bitflags {
    ($b:ty : $o:ty) => {
        impl bincode::Encode for $b {
            fn encode<E: bincode::enc::Encoder>(
                &self,
                encoder: &mut E,
            ) -> Result<(), bincode::error::EncodeError> {
                self.bits().encode(encoder)
            }
        }
        impl<C> bincode::Decode<C> for $b {
            fn decode<D: bincode::de::Decoder<Context = C>>(
                decoder: &mut D,
            ) -> Result<Self, bincode::error::DecodeError> {
                Ok(Self::from_bits_retain(<$o>::decode(decoder)?))
            }
        }
        impl<'de, C> bincode::de::BorrowDecode<'de, C> for $b {
            fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = C>>(
                decoder: &mut D,
            ) -> Result<Self, bincode::error::DecodeError> {
                Ok(Self::from_bits_retain(<$o>::decode(decoder)?))
            }
        }
    };
}
