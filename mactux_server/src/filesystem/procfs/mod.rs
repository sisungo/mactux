//! Implementation of `procfs`.
//!
//! Actually, it is a special kind of `tmpfs`.

use crate::filesystem::tmpfs::Tmpfs;
use std::sync::Arc;
use structures::error::LxError;

pub fn new() -> Result<Arc<Tmpfs>, LxError> {
    let tmpfs = Tmpfs::new()?;
    Ok(tmpfs)
}
