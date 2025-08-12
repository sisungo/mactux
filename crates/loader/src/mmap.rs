use std::{ffi::c_int, os::fd::RawFd};

/// A mapped memory area that does RAII.
#[derive(Debug)]
pub struct MappedArea {
    addr: *mut u8,
    len: usize,
    auto_unmap: bool,
}
impl MappedArea {
    pub fn null() -> Self {
        Self {
            addr: std::ptr::null_mut(),
            len: 0,
            auto_unmap: false,
        }
    }

    pub fn builder() -> MappedAreaBuilder {
        MappedAreaBuilder::new()
    }

    pub fn addr(&self) -> *mut u8 {
        self.addr
    }
}
impl Drop for MappedArea {
    fn drop(&mut self) {
        unsafe {
            if self.auto_unmap {
                libc::munmap(self.addr as _, self.len);
            }
        }
    }
}

/// A builder of a mapped memory area.
#[derive(Debug, Clone, Copy)]
pub struct MappedAreaBuilder {
    addr: usize,
    len: usize,
    prot: c_int,
    flags: c_int,
    fd: c_int,
    offset: u64,
    auto_unmap: bool,
}
impl MappedAreaBuilder {
    pub fn new() -> Self {
        MappedAreaBuilder {
            addr: 0,
            len: 0,
            prot: 0,
            flags: libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            fd: -1,
            offset: 0,
            auto_unmap: true,
        }
    }

    pub fn destination(mut self, addr: usize) -> Self {
        self.addr = addr;
        self.flags |= libc::MAP_FIXED;
        self
    }

    pub fn len(mut self, len: usize) -> Self {
        self.len = len;
        self
    }

    pub fn readable(mut self) -> Self {
        self.prot |= libc::PROT_READ;
        self
    }

    pub fn writable(mut self) -> Self {
        self.prot |= libc::PROT_WRITE;
        self
    }

    pub fn executable(mut self) -> Self {
        self.prot |= libc::PROT_EXEC;
        self
    }

    pub fn file(mut self, fd: RawFd, offset: u64) -> Self {
        self.fd = fd;
        self.offset = offset;
        self.flags &= !libc::MAP_ANONYMOUS;
        self
    }

    /// Indicates whether RAII is enabled for the built [`MappedArea`].
    pub fn auto_unmap(mut self, value: bool) -> Self {
        self.auto_unmap = value;
        self
    }

    /// Performs the mapping.
    pub unsafe fn build(self) -> std::io::Result<MappedArea> {
        let addr = unsafe {
            libc::mmap(
                self.addr as _,
                self.len,
                self.prot,
                self.flags,
                self.fd,
                self.offset as _,
            )
        };
        if addr == libc::MAP_FAILED {
            return Err(std::io::Error::last_os_error());
        }

        Ok(MappedArea {
            addr: addr as _,
            len: self.len,
            auto_unmap: self.auto_unmap,
        })
    }
}
