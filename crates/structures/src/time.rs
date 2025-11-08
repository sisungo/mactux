use crate::{FromApple, ToApple, error::LxError, unixvariants};
use bincode::{Decode, Encode};
use bitflags::bitflags;
use libc::c_int;
use std::time::Duration;

unixvariants! {
    #[derive(Default)]
    pub struct ClockId: u32 {
        const CLOCK_REALTIME = 0;
        const CLOCK_MONOTONIC = 1;
        const CLOCK_PROCESS_CPUTIME_ID = 2;
        const CLOCK_THREAD_CPUTIME_ID = 3;
        fn from_apple(apple: libc::clockid_t) -> Result<Self, LxError>;
        fn to_apple(self) -> Result<libc::clockid_t, LxError>;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct TimerFlags: u32 {
        const TIMER_ABSTIME = 1;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode)]
#[repr(C)]
pub struct Timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}
impl Timespec {
    pub fn now() -> Timespec {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap();
        Timespec {
            tv_sec: now.as_secs() as _,
            tv_nsec: now.as_nanos() as _,
        }
    }

    pub fn to_duration(self) -> Duration {
        Duration::new(self.tv_sec as _, self.tv_nsec as _)
    }
}
impl FromApple for Timespec {
    type Apple = libc::timespec;

    fn from_apple(apple: Self::Apple) -> Result<Self, LxError> {
        Ok(Self {
            tv_sec: apple.tv_sec,
            tv_nsec: apple.tv_nsec,
        })
    }
}
impl ToApple for Timespec {
    type Apple = libc::timespec;

    fn to_apple(self) -> Result<Self::Apple, LxError> {
        Ok(libc::timespec {
            tv_sec: self.tv_sec,
            tv_nsec: self.tv_nsec,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct Timeval {
    pub tv_sec: i64,
    pub tv_usec: i64,
}
impl Timeval {
    pub const fn to_timespec(self) -> Timespec {
        Timespec {
            tv_sec: self.tv_sec,
            tv_nsec: self.tv_usec * 1000,
        }
    }
}
impl FromApple for Timeval {
    type Apple = libc::timeval;

    fn from_apple(apple: libc::timeval) -> Result<Self, LxError> {
        Ok(Self {
            tv_sec: apple.tv_sec,
            tv_usec: apple.tv_usec as _,
        })
    }
}
impl ToApple for Timeval {
    type Apple = libc::timeval;

    fn to_apple(self) -> Result<libc::timeval, LxError> {
        Ok(libc::timeval {
            tv_sec: self.tv_sec,
            tv_usec: self.tv_usec as _,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct Timezone {
    pub tz_minuteswest: c_int,
    pub tz_dsttime: c_int,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct Tms {
    pub tms_utime: i64,
    pub tms_stime: i64,
    pub tms_cutime: i64,
    pub tms_cstime: i64,
}
impl FromApple for Tms {
    type Apple = libc::tms;

    fn from_apple(apple: libc::tms) -> Result<Self, LxError> {
        Ok(Self {
            tms_utime: apple.tms_utime as _,
            tms_stime: apple.tms_stime as _,
            tms_cutime: apple.tms_cutime as _,
            tms_cstime: apple.tms_cstime as _,
        })
    }
}
