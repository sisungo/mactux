//! The device model and simple standard devices.

mod auxmem;
mod loopdev;
mod term;

#[cfg(feature = "audio")]
mod oss;

use crate::vfd::IoctlOutput;
use dashmap::DashMap;
use mactux_ipc::response::{Response, VirtualFdAvailCtrl};
use rustc_hash::FxBuildHasher;
use std::{path::PathBuf, sync::Arc};
use structures::{device::DeviceNumber, error::LxError, fs::OpenFlags, io::IoctlCmd};

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
    fn open(&self, flags: OpenFlags) -> Result<(), LxError> {
        Ok(())
    }

    /// Reads from the device.
    fn read(&self, buf: &mut [u8], off: &mut i64) -> Result<usize, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    /// Writes to the device.
    fn write(&self, buf: &[u8], off: &mut i64) -> Result<usize, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    fn ioctl_query(&self, cmd: IoctlCmd) -> Result<VirtualFdAvailCtrl, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    /// Controls the device.
    fn ioctl(&self, cmd: IoctlCmd, buf: &[u8]) -> Result<Response, LxError> {
        Err(LxError::EOPNOTSUPP)
    }

    /// This is called when the device is closed.
    fn close(&self) {}
}
