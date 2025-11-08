//! Implementation of `MAJOR=2char | MAJOR=3char | MAJOR=5char` terminal-related devices.

use crate::{
    device::{Device, DeviceTable},
    vfd::Stream,
};
use std::{path::PathBuf, sync::Arc};
use structures::error::LxError;

struct Tty;
impl Stream for Tty {
    fn read(&self, buf: &mut [u8], _off: &mut i64) -> Result<usize, LxError> {
        Err(LxError::EOPNOTSUPP)
    }
}
impl Device for Tty {
    fn macos_device(&self) -> Option<PathBuf> {
        Some(PathBuf::from("/dev/tty"))
    }
}

pub fn discover(devices: &DeviceTable) {
    devices.add_chr_fixed(5, 0, || Arc::new(Tty));
}
