use crate::{bitflags_impl_to_apple, newtype_impl_to_apple};
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
impl MmapProt {
    #[inline]
    pub fn to_apple(self) -> c_int {
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
        apple
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
impl MmapFlags {
    #[inline]
    pub fn to_apple(self) -> c_int {
        bitflags_impl_to_apple!(self = MAP_ANON, MAP_PRIVATE, MAP_FIXED)
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct MsyncFlags: u32 {
        const MS_ASYNC = 1;
        const MS_INVALIDATE = 2;
        const MS_SYNC = 4;
    }
}
impl MsyncFlags {
    #[inline]
    pub fn to_apple(self) -> c_int {
        bitflags_impl_to_apple!(self = MS_ASYNC, MS_INVALIDATE, MS_SYNC)
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct MremapFlags: u32 {
        const MREMAP_MAYMOVE = 1;
        const MREMAP_FIXED = 2;
        const MREMAP_DONTUNMAP = 4;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Madvice(pub u32);
impl Madvice {
    pub const MADV_NORMAL: Self = Self(0);
    pub const MADV_RANDOM: Self = Self(1);
    pub const MADV_SEQUENTIAL: Self = Self(2);
    pub const MADV_WILLNEED: Self = Self(3);
    pub const MADV_DONTNEED: Self = Self(4);
    pub const MADV_FREE: Self = Self(8);
    pub const MADV_REMOVE: Self = Self(9);
    pub const MADV_DONTFORK: Self = Self(10);
    pub const MADV_DOFORK: Self = Self(11);
    pub const MADV_MERGEABLE: Self = Self(12);
    pub const MADV_UNMERGEABLE: Self = Self(13);
    pub const MADV_HUGEPAGE: Self = Self(14);
    pub const MADV_NOHUGEPAGE: Self = Self(15);
    pub const MADV_DONTDUMP: Self = Self(16);
    pub const MADV_DODUMP: Self = Self(17);
    pub const MADV_WIPEONFORK: Self = Self(18);
    pub const MADV_KEEPONFORK: Self = Self(19);
    pub const MADV_COLD: Self = Self(20);
    pub const MADV_PAGEOUT: Self = Self(21);
    pub const MADV_POPULATE_READ: Self = Self(22);
    pub const MADV_POPULATE_WRITE: Self = Self(23);
    pub const MADV_COLLAPSE: Self = Self(25);
    pub const MADV_HWPOISON: Self = Self(100);
    pub const MADV_SOFT_OFFLINE: Self = Self(101);
    pub const MADV_GUARD_INSTALL: Self = Self(102);
    pub const MADV_GUARD_REMOVE: Self = Self(103);

    pub fn to_apple(self) -> Option<c_int> {
        newtype_impl_to_apple!(
            self = MADV_NORMAL,
            MADV_RANDOM,
            MADV_SEQUENTIAL,
            MADV_WILLNEED,
            MADV_DONTNEED,
            MADV_FREE
        )
    }
}
