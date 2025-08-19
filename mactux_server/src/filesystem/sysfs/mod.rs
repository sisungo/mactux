use crate::{filesystem::kernfs::KernFs, filesystem::vfs::Mountable};
use std::sync::Arc;
use structures::error::LxError;

pub fn mountable() -> Result<Arc<dyn Mountable>, LxError> {
    let kernfs = KernFs::new();
    let writer = kernfs.0.table.write().unwrap();

    drop(writer);
    Ok(Arc::new(kernfs))
}
