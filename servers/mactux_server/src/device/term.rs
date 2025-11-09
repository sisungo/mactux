//! Implementation of `MAJOR=2char | MAJOR=3char | MAJOR=5char` terminal-related devices.

use crate::{
    device::{Device, DeviceTable},
    vfd::Stream,
};
use std::{
    io::{Read, Write},
    path::PathBuf,
    sync::Arc,
};
use structures::error::LxError;

struct Tty;
impl Stream for Tty {
    fn read(&self, _: &mut [u8], _: &mut i64) -> Result<usize, LxError> {
        Err(LxError::EOPNOTSUPP)
    }
}
impl Device for Tty {
    fn macos_device(&self) -> Option<PathBuf> {
        Some(PathBuf::from("/dev/tty"))
    }
}

struct Console;
impl Stream for Console {
    fn read(&self, buf: &mut [u8], _: &mut i64) -> Result<usize, LxError> {
        Ok(std::io::stdin().read(buf)?)
    }

    fn write(&self, buf: &[u8], _: &mut i64) -> Result<usize, LxError> {
        Ok(std::io::stdout().write(buf)?)
    }
}
impl Device for Console {}

pub fn discover(devices: &DeviceTable) {
    devices.add_chr_fixed(5, 0, || Arc::new(Tty));
    devices.add_chr_fixed(5, 1, || Arc::new(Console));
}
