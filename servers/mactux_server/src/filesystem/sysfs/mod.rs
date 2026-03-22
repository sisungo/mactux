use crate::filesystem::{
    tmpfs::Tmpfs,
    vfs::{Filesystem, MakeFilesystem},
};
use std::sync::Arc;
use structures::{
    error::LxError,
    fs::{FsMagic, MountFlags},
};

pub fn new() -> Result<Arc<Tmpfs>, LxError> {
    let tmpfs = Tmpfs::new()?;
    tmpfs.set_fs_magic(FsMagic::SYSFS_MAGIC);

    Ok(tmpfs)
}

pub struct MakeSysfs;
impl MakeFilesystem for MakeSysfs {
    fn make_filesystem(
        &self,
        _: &[u8],
        _: MountFlags,
        _: &[u8],
    ) -> Result<Arc<dyn Filesystem>, LxError> {
        Ok(new()?)
    }

    fn is_nodev(&self) -> bool {
        true
    }
}
