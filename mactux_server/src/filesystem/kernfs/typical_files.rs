use super::KernFsFile;
use crate::{
    filesystem::vfs::NewlyOpen,
    util::FileAttrs,
    vfd::{VirtualFd, VirtualFile},
};
use async_trait::async_trait;
use std::sync::Arc;
use structures::{
    error::LxError,
    fs::{OpenFlags, Statx},
};

#[derive(Debug, Clone)]
struct FnFile<F> {
    func: F,
    attrs: FileAttrs,
}
#[async_trait]
impl<F: Fn() -> Result<Vec<u8>, LxError> + Clone + Send + Sync> VirtualFile for FnFile<F> {
    async fn read(&self, buf: &mut [u8], off: &mut u64) -> Result<usize, LxError> {
        let s = (self.func)()?;
        if *off >= s.len() as _ {
            return Ok(0);
        }
        let bytes_read = buf.len().min(s.len() - *off as usize);
        buf[..bytes_read].copy_from_slice(&s[(*off as _)..(*off as usize + bytes_read)]);
        *off += bytes_read as u64;
        Ok(bytes_read)
    }

    async fn stat(&self) -> Result<Statx, LxError> {
        Ok(Statx {
            stx_mask: 0,
            stx_dev_major: 0,
            stx_dev_minor: 0,
            stx_ino: 0,
            stx_nlink: 0,
            stx_uid: self.attrs.uid,
            stx_gid: self.attrs.gid,
            stx_mode: self.attrs.mode as u16 | 0o20000,
            stx_attributes: 0,
            stx_attributes_mask: 0,
            stx_rdev_major: 0,
            stx_rdev_minor: 0,
            stx_size: 0,
            stx_blksize: 0,
            stx_blocks: 0,
            stx_atime: self.attrs.atime.into(),
            stx_btime: self.attrs.btime.into(),
            stx_ctime: self.attrs.ctime.into(),
            stx_mtime: self.attrs.mtime.into(),
            stx_mnt_id: 0,
            stx_dio_mem_align: 0,
            stx_dio_offset_align: 0,
            stx_dio_read_offset_align: 0,
            stx_atomic_write_segments_max: 0,
            stx_atomic_write_unit_min: 0,
            stx_atomic_write_unit_max: 0,
            stx_subvol: 0,
        })
    }
}
#[async_trait]
impl<F: Fn() -> Result<Vec<u8>, LxError> + Clone + Send + Sync + 'static> KernFsFile for FnFile<F> {
    async fn open(&self, flags: OpenFlags) -> Result<NewlyOpen, LxError> {
        Ok(NewlyOpen::Virtual(VirtualFd::new(
            Box::new(self.clone()),
            flags,
        )))
    }
}

pub fn fn_file<F: Fn() -> Result<Vec<u8>, LxError> + Clone + Send + Sync + 'static>(
    func: F,
) -> Arc<dyn KernFsFile> {
    Arc::new(FnFile {
        func,
        attrs: FileAttrs::common(),
    })
}
