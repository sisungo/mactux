//! Memory mapping for the ELF loader.

use rtenv::rust::RawRtFd;
use std::{io::Write, os::fd::IntoRawFd};
use structures::{
    error::LxError,
    io::Whence,
    mm::{MmapFlags, MmapProt},
};

/// A mapped memory area that does RAII.
#[derive(Debug)]
pub struct MappedArea {
    addr: *mut u8,
    len: usize,
    auto_unmap: bool,
}
impl MappedArea {
    /// Creates a "null" mapped area that works as a placeholder.
    pub fn null() -> Self {
        Self {
            addr: std::ptr::null_mut(),
            len: 0,
            auto_unmap: false,
        }
    }

    /// Returns a builder.
    pub fn builder() -> MappedAreaBuilder {
        MappedAreaBuilder::new()
    }

    /// Returns address of the mapped area.
    pub fn addr(&self) -> *mut u8 {
        self.addr
    }
}
impl Drop for MappedArea {
    fn drop(&mut self) {
        unsafe {
            if self.auto_unmap {
                _ = rtenv::mm::unmap(self.addr, self.len);
            }
        }
    }
}

/// A builder of a mapped memory area.
#[derive(Debug, Clone, Copy)]
pub struct MappedAreaBuilder {
    addr: usize,
    len: usize,
    prot: MmapProt,
    flags: MmapFlags,
    fd: RawRtFd,
    offset: u64,
    auto_unmap: bool,
}
impl MappedAreaBuilder {
    /// Creates a new [`MappedAreaBuilder`] instance.
    pub fn new() -> Self {
        MappedAreaBuilder {
            addr: 0,
            len: 0,
            prot: MmapProt::empty(),
            flags: MmapFlags::MAP_PRIVATE | MmapFlags::MAP_ANON,
            fd: RawRtFd(-1),
            offset: 0,
            auto_unmap: true,
        }
    }

    /// Specifies destination of the mapped area.
    pub fn destination(mut self, addr: usize) -> Self {
        self.addr = addr;
        self.flags |= MmapFlags::MAP_FIXED;
        self
    }

    /// Specifies length of the mapped area.
    pub fn len(mut self, len: usize) -> Self {
        self.len = len;
        self
    }

    /// Makes the mapped area readable.
    pub fn readable(mut self) -> Self {
        self.prot |= MmapProt::PROT_READ;
        self
    }

    /// Makes the mapped area writable.
    pub fn writable(mut self) -> Self {
        self.prot |= MmapProt::PROT_WRITE;
        self
    }

    /// Makes the mapped area executable.
    pub fn executable(mut self) -> Self {
        self.prot |= MmapProt::PROT_EXEC;
        self
    }

    /// Specifies file descriptor and offset of the mapped area.
    pub fn file(mut self, fd: RawRtFd, offset: u64) -> Self {
        self.fd = fd;
        self.offset = offset;
        self.flags &= !MmapFlags::MAP_ANON;
        self
    }

    /// Indicates whether RAII is enabled for the built [`MappedArea`].
    pub fn auto_unmap(mut self, value: bool) -> Self {
        self.auto_unmap = value;
        self
    }

    /// Performs the mapping.
    pub unsafe fn build(self) -> Result<MappedArea, LxError> {
        let mut fd = self.fd.0;
        if self.fd.is_virtual() {
            fd = copy_vfd(fd)?;
        }

        let addr = unsafe {
            rtenv::mm::map(
                self.addr as _,
                self.len,
                self.prot,
                self.flags,
                fd,
                self.offset as _,
            )?
        };

        Ok(MappedArea {
            addr,
            len: self.len,
            auto_unmap: self.auto_unmap,
        })
    }
}

fn copy_vfd(fd: i32) -> Result<i32, LxError> {
    let mut tempfile = tempfile::Builder::new().disable_cleanup(true).tempfile()?;
    let mut buf = vec![0u8; 4096];
    let fd_pos = rtenv::io::lseek(fd, 0, Whence::SEEK_CUR)?;
    rtenv::io::lseek(fd, 0, Whence::SEEK_DATA)?;
    loop {
        let n = rtenv::io::read(fd, &mut buf)?;
        if n == 0 {
            break;
        }
        tempfile.write_all(&buf[..n])?;
    }
    let Ok(readable) = std::fs::File::open(tempfile.path()) else {
        eprintln!("mactux: failed to reopen temporary file for reading");
        std::process::exit(101);
    };
    rtenv::io::lseek(fd, fd_pos, Whence::SEEK_DATA)?;
    Ok(readable.into_raw_fd())
}
