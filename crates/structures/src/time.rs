use crate::unixvariants;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct Timezone {
    pub tz_minuteswest: c_int,
    pub tz_dsttime: c_int,
}
