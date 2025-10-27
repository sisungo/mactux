//! Implementation of `procfs`.
//!
//! Actually, it is a special kind of `tmpfs`.

use crate::filesystem::tmpfs::Tmpfs;

pub struct Procfs {
    tmpfs: Tmpfs,
}
