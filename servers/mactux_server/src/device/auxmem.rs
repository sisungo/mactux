//! Implementation of `MAJOR=1char`, auxiliary memory devices.

use crate::{
    device::{Device, DeviceTable},
    vfd::Stream,
};
use std::{path::PathBuf, sync::Arc};
use structures::error::LxError;

struct Zero;
impl Stream for Zero {
    fn read(&self, buf: &mut [u8], _off: &mut i64) -> Result<usize, LxError> {
        buf.fill(0);
        Ok(buf.len())
    }
}
impl Device for Zero {
    fn macos_device(&self) -> Option<PathBuf> {
        Some(PathBuf::from("/dev/zero"))
    }
}

struct Null;
impl Stream for Null {
    fn read(&self, _buf: &mut [u8], _off: &mut i64) -> Result<usize, LxError> {
        Ok(0)
    }

    fn write(&self, buf: &[u8], _off: &mut i64) -> Result<usize, LxError> {
        Ok(buf.len())
    }
}
impl Device for Null {
    fn macos_device(&self) -> Option<PathBuf> {
        Some(PathBuf::from("/dev/null"))
    }
}

struct Full;
impl Stream for Full {
    fn read(&self, buf: &mut [u8], _off: &mut i64) -> Result<usize, LxError> {
        buf.fill(0);
        Ok(buf.len())
    }

    fn write(&self, _buf: &[u8], _off: &mut i64) -> Result<usize, LxError> {
        Err(LxError::ENOSPC)
    }
}
impl Device for Full {}

struct Random;
impl Stream for Random {}
impl Device for Random {
    fn macos_device(&self) -> Option<PathBuf> {
        Some(PathBuf::from("/dev/random"))
    }
}

struct URandom;
impl Stream for URandom {}
impl Device for URandom {
    fn macos_device(&self) -> Option<PathBuf> {
        Some(PathBuf::from("/dev/urandom"))
    }
}

pub fn discover(devices: &DeviceTable) {
    devices.add_chr_fixed(1, 3, || Arc::new(Null));
    devices.add_chr_fixed(1, 5, || Arc::new(Zero));
    devices.add_chr_fixed(1, 7, || Arc::new(Full));
    devices.add_chr_fixed(1, 8, || Arc::new(Random));
    devices.add_chr_fixed(1, 9, || Arc::new(URandom));
}
