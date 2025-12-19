use crate::error::LxError;
use bitflags::bitflags;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
#[repr(C)]
pub struct UtsName {
    pub sysname: [u8; 65],
    pub nodename: [u8; 65],
    pub release: [u8; 65],
    pub version: [u8; 65],
    pub machine: [u8; 65],
    pub domainname: [u8; 65],
}

/// Converts a byte string to the format that fits [`UtsName`].
#[inline]
pub fn uname_str(s: &[u8]) -> Result<[u8; 65], LxError> {
    if s.len() >= 65 {
        return Err(LxError::ENOMEM);
    }

    let mut data = [0; 65];
    data[..s.len()].copy_from_slice(s);
    Ok(data)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(C)]
pub struct SysInfo {
    pub uptime: i64,
    pub loads: [u64; 3],
    pub totalram: u64,
    pub freeram: u64,
    pub sharedram: u64,
    pub bufferram: u64,
    pub totalswap: u64,
    pub freeswap: u64,
    pub procs: u16,
    pub totalhigh: u64,
    pub freehigh: u64,
    pub mem_unit: u32,
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct GrndFlags: u32 {
        const GRND_NONBLOCK = 1;
        const GRND_RANDOM = 2;
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct SyslogAction(pub u32);
impl SyslogAction {
    pub const SYSLOG_ACTION_READ_ALL: Self = Self(3);
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct LogLevel(pub u32);
impl LogLevel {
    pub const KERN_EMERG: Self = Self(0);
    pub const KERN_ALERT: Self = Self(1);
    pub const KERN_CRIT: Self = Self(2);
    pub const KERN_ERR: Self = Self(3);
    pub const KERN_WARNING: Self = Self(4);
    pub const KERN_NOTICE: Self = Self(5);
    pub const KERN_INFO: Self = Self(6);
    pub const KERN_DEBUG: Self = Self(7);
}
