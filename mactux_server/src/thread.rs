//! The threading model.

use std::sync::RwLock;

pub struct ThreadCtx {
    name: RwLock<Vec<u8>>,
}
