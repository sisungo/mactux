//! The Linux device model.

use crate::filesystem::{kernfs::KernFsFile, vfs::NewlyOpen};
use async_trait::async_trait;
use std::path::PathBuf;
use structures::{error::LxError, fs::OpenFlags};

#[derive(Debug, Clone, Copy)]
pub struct Null;
#[async_trait]
impl KernFsFile for Null {
    async fn open(&self, _: OpenFlags) -> Result<NewlyOpen, LxError> {
        Ok(NewlyOpen::AtNative(PathBuf::from("/dev/null")))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Zero;
#[async_trait]
impl KernFsFile for Zero {
    async fn open(&self, _: OpenFlags) -> Result<NewlyOpen, LxError> {
        Ok(NewlyOpen::AtNative(PathBuf::from("/dev/zero")))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Random;
#[async_trait]
impl KernFsFile for Random {
    async fn open(&self, _: OpenFlags) -> Result<NewlyOpen, LxError> {
        Ok(NewlyOpen::AtNative(PathBuf::from("/dev/random")))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct URandom;
#[async_trait]
impl KernFsFile for URandom {
    async fn open(&self, _: OpenFlags) -> Result<NewlyOpen, LxError> {
        Ok(NewlyOpen::AtNative(PathBuf::from("/dev/urandom")))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Tty;
#[async_trait]
impl KernFsFile for Tty {
    async fn open(&self, _: OpenFlags) -> Result<NewlyOpen, LxError> {
        Ok(NewlyOpen::AtNative(PathBuf::from("/dev/tty")))
    }
}
