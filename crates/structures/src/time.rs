use crate::{error::LxError, newtype_impl_to_apple};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ClockId(pub u32);
impl ClockId {
    pub const CLOCK_REALTIME: Self = Self(0);
    pub const CLOCK_MONOTONIC: Self = Self(1);
    pub const CLOCK_PROCESS_CPUTIME_ID: Self = Self(2);
    pub const CLOCK_THREAD_CPUTIME_ID: Self = Self(3);

    pub fn to_apple(self) -> Result<libc::clockid_t, LxError> {
        newtype_impl_to_apple!(
            self = CLOCK_REALTIME,
            CLOCK_MONOTONIC,
            CLOCK_PROCESS_CPUTIME_ID,
            CLOCK_THREAD_CPUTIME_ID
        )
        .ok_or(LxError::EINVAL)
    }
}
impl Default for ClockId {
    fn default() -> Self {
        Self::CLOCK_REALTIME
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct Timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}
impl Timespec {
    pub fn from_apple(apple: libc::timespec) -> Self {
        Self {
            tv_sec: apple.tv_sec,
            tv_nsec: apple.tv_nsec,
        }
    }

    pub fn to_apple(self) -> libc::timespec {
        libc::timespec {
            tv_sec: self.tv_sec,
            tv_nsec: self.tv_nsec,
        }
    }

    pub fn to_duration(self) -> Duration {
        Duration::new(self.tv_sec as _, self.tv_nsec as _)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct Timeval {
    pub tv_sec: i64,
    pub tv_usec: i64,
}
impl Timeval {
    pub fn from_apple(apple: libc::timeval) -> Self {
        Self {
            tv_sec: apple.tv_sec,
            tv_usec: apple.tv_usec as _,
        }
    }

    pub fn to_timespec(self) -> Timespec {
        Timespec {
            tv_sec: self.tv_sec,
            tv_nsec: self.tv_usec * 1000,
        }
    }
}
