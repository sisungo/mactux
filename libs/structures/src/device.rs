use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DeviceNumber(pub u64);
impl DeviceNumber {
    pub fn new(major: u32, minor: u32) -> Self {
        Self(
            ((major as u64 & 0xfffff000) << 32)
                | ((major as u64 & 0xfff) << 8)
                | ((minor as u64 & 0xffffff00) << 12)
                | ((minor as u64) & 0xff),
        )
    }

    pub fn major(self) -> u32 {
        (((self.0 >> 32) & 0xfffff000) | ((self.0 >> 8) & 0xfff)) as u32
    }

    pub fn minor(self) -> u32 {
        (((self.0 >> 12) & 0xffffff00) | (self.0 & 0xff)) as u32
    }
}
impl Debug for DeviceNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DeviceNumber({}:{})", self.major(), self.minor())
    }
}
