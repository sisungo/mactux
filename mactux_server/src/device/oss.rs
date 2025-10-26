//! OSS audio devices.

use crate::device::{Device, DeviceTable};
use std::sync::Arc;

/// The `/dev/dsp` device.
#[derive(Debug)]
struct Dsp {}
impl Dsp {
    fn new() -> Self {
        Self {}
    }
}
impl Device for Dsp {}

pub fn discover(devices: &DeviceTable) {
    devices.add_chr_fixed(14, 3, || Arc::new(Dsp::new()));
}
