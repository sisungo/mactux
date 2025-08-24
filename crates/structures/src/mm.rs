use crate::{error::LxError, unixvariants, ToApple};
use bitflags::bitflags;
use libc::c_int;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct MmapProt: u32 {
        const PROT_READ = 1;
        const PROT_WRITE = 2;
        const PROT_EXEC = 4;
    }
}
impl ToApple for MmapProt {
    type Apple = c_int;

    fn to_apple(self) -> Result<c_int, LxError> {
        let mut apple = 0;
        if self.contains(Self::PROT_READ) {
            apple |= libc::PROT_READ;
        }
        if self.contains(Self::PROT_WRITE) {
            apple |= libc::PROT_WRITE;
        }
        if self.contains(Self::PROT_EXEC) {
            apple |= libc::PROT_EXEC | libc::PROT_WRITE;
        }
        Ok(apple)
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct MmapFlags: u32 {
        const MAP_ANON = 0x20;
        const MAP_PRIVATE = 0x02;
        const MAP_FIXED = 0x10;
    }
}
crate::bitflags_impl_from_to_apple!(
    MmapFlags;
    type Apple = c_int;
    values = MAP_ANON, MAP_PRIVATE, MAP_FIXED
);

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct MsyncFlags: u32 {
        const MS_ASYNC = 1;
        const MS_INVALIDATE = 2;
        const MS_SYNC = 4;
    }
}
crate::bitflags_impl_from_to_apple!(
    MsyncFlags;
    type Apple = c_int;
    values = MS_ASYNC, MS_INVALIDATE, MS_SYNC
);

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct MremapFlags: u32 {
        const MREMAP_MAYMOVE = 1;
        const MREMAP_FIXED = 2;
        const MREMAP_DONTUNMAP = 4;
    }
}

unixvariants! {
    pub struct Madvice: u32 {
        const MADV_NORMAL = 0;
        const MADV_RANDOM = 1;
        const MADV_SEQUENTIAL = 2;
        const MADV_WILLNEED = 3;
        const MADV_DONTNEED = 4;
        const MADV_FREE = 8;
        #[linux_only] const MADV_REMOVE = 9;
        #[linux_only] const MADV_DONTFORK = 10;
        #[linux_only] const MADV_DOFORK = 11;
        #[linux_only] const MADV_MERGEABLE = 12;
        #[linux_only] const MADV_UNMERGEABLE = 13;
        #[linux_only] const MADV_HUGEPAGE = 14;
        #[linux_only] const MADV_NOHUGEPAGE = 15;
        #[linux_only] const MADV_DONTDUMP = 16;
        #[linux_only] const MADV_DODUMP = 17;
        #[linux_only] const MADV_WIPEONFORK = 18;
        #[linux_only] const MADV_KEEPONFORK = 19;
        #[linux_only] const MADV_COLD = 20;
        #[linux_only] const MADV_PAGEOUT = 21;
        #[linux_only] const MADV_POPULATE_READ = 22;
        #[linux_only] const MADV_POPULATE_WRITE = 23;
        #[linux_only] const MADV_COLLAPSE = 25;
        #[linux_only] const MADV_HWPOISON = 100;
        #[linux_only] const MADV_SOFT_OFFLINE = 101;
        #[linux_only] const MADV_GUARD_INSTALL = 102;
        #[linux_only] const MADV_GUARD_REMOVE = 103;
        fn from_apple(apple: c_int) -> Result<Self, LxError>;
        fn to_apple(self) -> Result<c_int, LxError>;
    }
}
