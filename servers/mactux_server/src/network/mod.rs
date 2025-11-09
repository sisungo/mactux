//! Networking.

mod abs;

use abs::AbstractNamespace;
use std::sync::atomic::{self, AtomicU64};

use crate::app;

/// A network namespace.
#[derive(Debug)]
pub struct NetNamespace {
    salt: String,
    pub abs: AbstractNamespace,
}
impl NetNamespace {
    pub fn new() -> anyhow::Result<Self> {
        let salt = salt();
        let abs = AbstractNamespace::new(app().work_dir.net().join(&salt))?;
        Ok(Self { salt, abs })
    }
}

fn salt() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, atomic::Ordering::Relaxed).to_string()
}
