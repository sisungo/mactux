//! Multimedia infrastructure.
//!
//! Note that this does not include implementations of the device files. Instead, it provides infrastructures to build
//! the devices files.

#[cfg(feature = "audio")]
pub mod audio;

#[cfg(feature = "gui")]
pub mod gui;
