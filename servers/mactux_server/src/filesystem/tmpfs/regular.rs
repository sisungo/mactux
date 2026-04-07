//! Regular file support for tmpfs.

use super::{BLOCK_SIZE, File, Metadata};
use crate::vfd::{Stream, VfdContent};
use std::sync::{Arc, RwLock};
use structures::{
    error::LxError,
    fs::{FileType, OpenFlags, Statx, StatxMask},
    io::Whence,
};

#[derive(Debug)]
pub struct Reg {
    metadata: Arc<Metadata>,
    buf: RegBuf,
}
impl Reg {
    pub fn new(metadata: Arc<Metadata>) -> Arc<Self> {
        Arc::new(Self {
            metadata,
            buf: RegBuf::new(),
        })
    }
}
impl File for Reg {
    fn open_vfd(self: Arc<Self>, _: OpenFlags) -> Result<Arc<dyn VfdContent>, LxError> {
        Ok(self.clone())
    }
}
impl Stream for Reg {
    fn read(&self, buf: &mut [u8], off: &mut i64) -> Result<usize, LxError> {
        let ret = self.buf.read(buf, *off as u64);
        *off += ret as i64;
        Ok(ret)
    }

    fn write(&self, buf: &[u8], off: &mut i64) -> Result<usize, LxError> {
        let ret = self.buf.write(buf, *off as u64);
        *off += ret as i64;
        Ok(ret)
    }

    fn seek(&self, orig_off: i64, whence: Whence, off: i64) -> Result<i64, LxError> {
        crate::util::plain_seek(orig_off, self.buf.size() as _, whence, off)
    }
}
impl VfdContent for Reg {
    fn stat(&self, mask: StatxMask) -> Result<Statx, LxError> {
        let mut stat = self.metadata.stat_template(mask);

        stat.stx_size = self.buf.size();
        stat.stx_blocks = self.buf.blocks() * (BLOCK_SIZE as u64 / 512);

        stat.stx_mode.set_file_type(FileType::RegularFile);

        Ok(stat)
    }

    fn utimens(&self, times: [structures::time::Timespec; 2]) -> Result<(), LxError> {
        self.metadata.utimens(times);
        Ok(())
    }

    fn chmod(&self, mode: u16) -> Result<(), LxError> {
        self.metadata
            .permbits
            .store(mode, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}

/// A buffer for regular files. Supports sparse files.
#[derive(Debug)]
pub struct RegBuf {
    inner: RwLock<Vec<u8>>,
}
impl RegBuf {
    pub const fn new() -> Self {
        Self {
            inner: RwLock::new(Vec::new()),
        }
    }

    pub fn blocks(&self) -> u64 {
        self.size().div_ceil(BLOCK_SIZE as _)
    }

    pub fn size(&self) -> u64 {
        self.inner.read().unwrap().len() as _
    }

    pub fn read(&self, buf: &mut [u8], off: u64) -> usize {
        let data = self.inner.read().unwrap();
        let bytes_to_read = (buf.len() as u64).min(data.len() as u64 - off);
        let actual_read = (bytes_to_read as usize).min(buf.len());
        buf[..actual_read].copy_from_slice(&data[off as usize..off as usize + actual_read]);
        actual_read
    }

    pub fn write(&self, buf: &[u8], off: u64) -> usize {
        let mut data = self.inner.write().unwrap();
        if data.len() < buf.len() + off as usize {
            let adding = buf.len() + off as usize - data.len();
            data.extend(std::iter::repeat_n(0, adding));
        }
        data[off as usize..off as usize + buf.len()].copy_from_slice(&buf);
        buf.len()
    }
}
