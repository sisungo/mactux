use crate::error::LxError;
use bincode::{Decode, Encode};
use bitflags::bitflags;

#[derive(Debug, Clone, Encode, Decode)]
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

#[derive(Debug, Clone, Encode, Decode)]
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
