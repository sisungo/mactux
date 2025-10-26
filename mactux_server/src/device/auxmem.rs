//! Implementation of `MAJOR=1char`, auxiliary memory devices.

use crate::device::{Device, DeviceTable};
use std::{path::PathBuf, sync::Arc};

struct Zero;
impl Device for Zero {
    fn macos_device(&self) -> Option<PathBuf> {
        Some(PathBuf::from("/dev/zero"))
    }
}

struct Null;
impl Device for Null {
    fn macos_device(&self) -> Option<PathBuf> {
        Some(PathBuf::from("/dev/null"))
    }
}

struct Full;
impl Device for Full {
    fn macos_device(&self) -> Option<PathBuf> {
        Some(PathBuf::from("/dev/full"))
    }
}

struct Random;
impl Device for Random {
    fn macos_device(&self) -> Option<PathBuf> {
        Some(PathBuf::from("/dev/random"))
    }
}

struct URandom;
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
