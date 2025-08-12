use crate::{
    filesystem::kernfs::{DirEntry, KernFs, fn_file},
    filesystem::vfs::Mountable,
};
use std::sync::Arc;
use structures::error::LxError;

pub fn mountable() -> Result<Arc<dyn Mountable>, LxError> {
    let kernfs = KernFs::new();
    let mut writer = kernfs.0.table.write().unwrap();

    

    drop(writer);
    Ok(Arc::new(kernfs))
}
