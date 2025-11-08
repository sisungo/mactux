//! The device model and simple standard devices.

mod auxmem;
mod loopdev;
mod term;

#[cfg(feature = "audio")]
mod oss;

use crate::vfd::Stream;
use dashmap::DashMap;
use rustc_hash::FxBuildHasher;
use std::{path::PathBuf, sync::Arc};
use structures::{device::DeviceNumber, error::LxError, fs::OpenFlags};

pub struct DeviceTable {
    chr: DashMap<DeviceNumber, Arc<dyn Device>, FxBuildHasher>,
    blk: DashMap<DeviceNumber, Arc<dyn Device>, FxBuildHasher>,
}
impl DeviceTable {
    pub fn new() -> Self {
        Self {
            chr: DashMap::default(),
            blk: DashMap::default(),
        }
    }

    pub fn find_chr(&self, dev: DeviceNumber) -> Result<Arc<dyn Device>, LxError> {
        self.chr.get(&dev).map(|x| x.clone()).ok_or(LxError::ENODEV)
    }

    pub fn find_blk(&self, dev: DeviceNumber) -> Result<Arc<dyn Device>, LxError> {
        self.blk.get(&dev).map(|x| x.clone()).ok_or(LxError::ENODEV)
    }

    pub fn add_chr_fixed<F: FnOnce() -> Arc<dyn Device>>(&self, major: u32, minor: u32, f: F) {
        self.chr
            .entry(DeviceNumber::new(major, minor))
            .or_insert_with(f);
    }

    pub fn add_blk_fixed<F: FnOnce() -> Arc<dyn Device>>(&self, major: u32, minor: u32, f: F) {
        self.blk
            .entry(DeviceNumber::new(major, minor))
            .or_insert_with(f);
    }

    pub fn discover(&self) {
        auxmem::discover(self);
        term::discover(self);

        #[cfg(feature = "audio")]
        oss::discover(self);
    }
}

/// A device.
pub trait Device: Send + Sync {
    /// If this device can map to a macOS device directly, returns its path, otherwise `None`.
    ///
    /// If the call to this device is requested by client Linux programs, it is guaranteed to use the macOS device if
    /// possible. Otherwise, this is not required to be checked first.
    fn macos_device(&self) -> Option<PathBuf> {
        None
    }

    /// This is called when the device is opened with given flags.
    fn open(&self, _flags: OpenFlags) -> Result<Arc<dyn Stream + Send + Sync>, LxError> {
        Err(LxError::EINVAL)
    }
}
