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
    fs::{OpenFlags, Stat},
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
        buf[..bytes_read].copy_from_slice(&s[*off as _..]);
        *off += bytes_read as u64;
        Ok(bytes_read)
    }

    async fn stat(&self) -> Result<Stat, LxError> {
        Ok(Stat {
            st_dev: 0,
            st_ino: 0,
            st_nlink: 0,
            st_mode: self.attrs.mode | 0o20000,
            st_uid: self.attrs.uid,
            st_gid: self.attrs.gid,
            _pad0: 0,
            st_rdev: 0,
            st_size: 0,
            st_blksize: 0,
            st_blocks: 0,
            st_atime: self.attrs.atime.tv_sec,
            st_atimensec: self.attrs.atime.tv_nsec as _,
            st_mtime: self.attrs.mtime.tv_sec,
            st_mtimensec: self.attrs.mtime.tv_nsec as _,
            st_ctime: self.attrs.ctime.tv_sec,
            st_ctimensec: self.attrs.ctime.tv_nsec as _,
            _unused: [0; _],
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
