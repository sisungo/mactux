use crate::{FromApple, ToApple, error::LxError, impl_bincode_for_bitflags, time::Timespec};
use bincode::{Decode, Encode};
use bitflags::bitflags;
use libc::c_int;
use std::{mem::offset_of, os::unix::fs::FileTypeExt};

pub const XATTR_NAMESPACE_USER_PREFIX: &[u8] = b"user.";
pub const XATTR_NAMESPACE_SYSTEM_PREFIX: &[u8] = b"system.";
pub const XATTR_NAMESPACE_SECURITY_PREFIX: &[u8] = b"security.";
pub const XATTR_NAMESPACE_TRUSTED_PREFIX: &[u8] = b"trusted.";
pub const XATTR_NAMESPACE_MACTUX_INTERNAL_PREFIX: &[u8] = b"_mactux.";

pub const XATTR_NAMESPACE_PREFIXES: &[&[u8]] = &[
    XATTR_NAMESPACE_USER_PREFIX,
    XATTR_NAMESPACE_SYSTEM_PREFIX,
    XATTR_NAMESPACE_SECURITY_PREFIX,
    XATTR_NAMESPACE_TRUSTED_PREFIX,
    XATTR_NAMESPACE_MACTUX_INTERNAL_PREFIX,
];

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct OpenFlags: u32 {
        const O_RDONLY = 0;
        const O_WRONLY = 1;
        const O_RDWR = 2;
        const O_CREAT = 0o100;
        const O_EXCL = 0o200;
        const O_NOCTTY = 0o400;
        const O_TRUNC = 0o1000;
        const O_NONBLOCK = 0o4000;
        const O_DSYNC = 0o10000;
        const O_ASYNC = 0o20000;
        const O_DIRECT = 0o40000;
        const O_LARGEFILE = 0o100000;
        const O_DIRECTORY = 0o200000;
        const O_NOFOLLOW = 0o400000;
        const O_NOATIME = 0o1000000;
        const O_CLOEXEC = 0o2000000;
        const O_SYNC = 0o4010000;
        const O_PATH = 0o10000000;
        const O_TMPFILE = 0o20200000;
    }
}
impl OpenFlags {
    pub fn is_readable(self) -> bool {
        let path_only = self.contains(Self::O_PATH);
        let write_only = self.contains(Self::O_WRONLY);
        !(path_only || write_only)
    }

    pub fn is_writable(self) -> bool {
        let write_only = self.contains(Self::O_WRONLY);
        let read_write = self.contains(Self::O_RDWR);
        let path_only = self.contains(Self::O_PATH);
        write_only || read_write && !path_only
    }
}
crate::bitflags_impl_from_to_apple!(
OpenFlags;
    type Apple = c_int;
    values = O_RDONLY,
    O_WRONLY,
    O_RDWR,
    O_CREAT,
    O_EXCL,
    O_NOCTTY,
    O_TRUNC,
    O_NONBLOCK,
    O_DSYNC,
    O_ASYNC,
    O_DIRECTORY,
    O_NOFOLLOW,
    O_CLOEXEC,
    O_SYNC
);

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct AtFlags: u32 {
        const AT_EMPTY_PATH = 0x1000;
        const AT_SYMLINK_NOFOLLOW = 0x100;
        const AT_REMOVEDIR = 0x200;
        const _AT_APPLE_SYMLINK = 0x8000;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct AccessFlags: u32 {
        const F_OK = 0;
        const R_OK = 4;
        const W_OK = 2;
        const X_OK = 1;
    }
}
crate::bitflags_impl_from_to_apple!(
    AccessFlags;
    type Apple = c_int;
    values = F_OK, R_OK, W_OK, X_OK
);
impl_bincode_for_bitflags!(AccessFlags: u32);

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct UmountFlags: u32 {
        const MNT_FORCE = 1;
        const MNT_DETACH = 2;
        const MNT_EXPIRE = 4;
        const UMOUNT_NOFOLLOW = 8;
    }
}

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct Dirent64Hdr {
    pub d_ino: u64,
    pub d_off: i64,
    pub d_reclen: u16,
    pub d_type: DirentType,
    pub _align: [u8; 5],
}

#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Dirent64 {
    hdr: Dirent64Hdr,
    name: Vec<u8>,
}
impl Dirent64 {
    pub fn new(mut hdr: Dirent64Hdr, name: Vec<u8>) -> Self {
        hdr.d_reclen = (size_of::<Dirent64Hdr>() + name.len() + 1) as _;
        Self { hdr, name }
    }

    pub fn size(&self) -> usize {
        self.hdr.d_reclen as usize
    }

    pub fn name(&self) -> &[u8] {
        &self.name
    }

    pub unsafe fn write_to(&self, pos: *mut u8) {
        unsafe {
            pos.copy_from(
                (&raw const self.hdr).cast(),
                offset_of!(Dirent64Hdr, _align),
            );
            pos.add(offset_of!(Dirent64Hdr, _align))
                .copy_from(self.name.as_ptr(), self.name.len());
            pos.add(offset_of!(Dirent64Hdr, _align) + self.name.len())
                .write(0);
        }
    }
}

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct DirentType(pub u8);
impl DirentType {
    pub const DT_UNKNOWN: Self = Self(0);
    pub const DT_FIFO: Self = Self(1);
    pub const DT_CHR: Self = Self(2);
    pub const DT_DIR: Self = Self(4);
    pub const DT_BLK: Self = Self(6);
    pub const DT_REG: Self = Self(8);
    pub const DT_LNK: Self = Self(10);
    pub const DT_SOCK: Self = Self(12);

    #[inline]
    pub fn from_std(ty: std::fs::FileType) -> Self {
        if ty.is_dir() {
            Self::DT_DIR
        } else if ty.is_file() {
            Self::DT_REG
        } else if ty.is_symlink() {
            Self::DT_LNK
        } else if ty.is_fifo() {
            Self::DT_FIFO
        } else if ty.is_block_device() {
            Self::DT_BLK
        } else if ty.is_char_device() {
            Self::DT_CHR
        } else if ty.is_socket() {
            Self::DT_SOCK
        } else {
            Self::DT_UNKNOWN
        }
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[repr(C)]
pub struct Stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_nlink: u64,
    pub st_mode: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub _pad0: u32,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_atime: i64,
    pub st_atimensec: u64,
    pub st_mtime: i64,
    pub st_mtimensec: u64,
    pub st_ctime: i64,
    pub st_ctimensec: u64,
    pub _unused: [i64; 3],
}
impl From<Statx> for Stat {
    fn from(val: Statx) -> Self {
        Stat {
            st_dev: val.stx_dev_major as _,
            st_ino: val.stx_ino,
            st_nlink: val.stx_nlink as _,
            st_mode: val.stx_mode.0 as _,
            st_uid: val.stx_uid,
            st_gid: val.stx_gid,
            _pad0: 0,
            st_rdev: val.stx_rdev_major as _,
            st_size: val.stx_size as _,
            st_blksize: val.stx_blksize as _,
            st_blocks: val.stx_blocks as _,
            st_atime: val.stx_atime.tv_sec,
            st_atimensec: val.stx_atime.tv_nsec as _,
            st_mtime: val.stx_mtime.tv_sec,
            st_mtimensec: val.stx_mtime.tv_nsec as _,
            st_ctime: val.stx_ctime.tv_sec,
            st_ctimensec: val.stx_ctime.tv_nsec as _,
            _unused: [0; _],
        }
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[repr(C)]
pub struct Statx {
    pub stx_mask: StatxMask,
    pub stx_blksize: u32,
    pub stx_attributes: StatxAttrs,
    pub stx_nlink: u32,
    pub stx_uid: u32,
    pub stx_gid: u32,
    pub stx_mode: FileMode,
    pub stx_ino: u64,
    pub stx_size: u64,
    pub stx_blocks: u64,
    pub stx_attributes_mask: u64,
    pub stx_atime: StatxTimestamp,
    pub stx_btime: StatxTimestamp,
    pub stx_ctime: StatxTimestamp,
    pub stx_mtime: StatxTimestamp,
    pub stx_rdev_major: u32,
    pub stx_rdev_minor: u32,
    pub stx_dev_major: u32,
    pub stx_dev_minor: u32,
    pub stx_mnt_id: u64,
    pub stx_dio_mem_align: u32,
    pub stx_dio_offset_align: u32,
    pub stx_subvol: u64,
    pub stx_atomic_write_unit_min: u32,
    pub stx_atomic_write_unit_max: u32,
    pub stx_atomic_write_segments_max: u32,
    pub stx_dio_read_offset_align: u32,
}
impl Statx {
    pub fn from_apple(stat: libc::stat) -> Self {
        Self {
            stx_mask: StatxMask::STATX_BASIC_STATS | StatxMask::STATX_BTIME,
            stx_blksize: stat.st_blksize as _,
            stx_attributes: StatxAttrs::empty(),
            stx_nlink: stat.st_nlink as _,
            stx_uid: stat.st_uid,
            stx_gid: stat.st_gid,
            stx_mode: FileMode::from_apple(stat.st_mode).unwrap(),
            stx_ino: stat.st_ino,
            stx_size: stat.st_size as _,
            stx_blocks: stat.st_blocks as _,
            stx_attributes_mask: 0,
            stx_atime: StatxTimestamp {
                tv_sec: stat.st_atime,
                tv_nsec: stat.st_atime_nsec as _,
            },
            stx_btime: StatxTimestamp {
                tv_sec: stat.st_birthtime,
                tv_nsec: stat.st_birthtime_nsec as _,
            },
            stx_ctime: StatxTimestamp {
                tv_sec: stat.st_ctime,
                tv_nsec: stat.st_ctime_nsec as _,
            },
            stx_mtime: StatxTimestamp {
                tv_sec: stat.st_mtime,
                tv_nsec: stat.st_mtime_nsec as _,
            },
            stx_rdev_major: libc::major(stat.st_rdev) as _,
            stx_rdev_minor: libc::minor(stat.st_rdev) as _,
            stx_dev_major: libc::major(stat.st_dev) as _,
            stx_dev_minor: libc::minor(stat.st_dev) as _,
            stx_mnt_id: 0,
            stx_dio_mem_align: 0,
            stx_dio_offset_align: 0,
            stx_subvol: 0,
            stx_atomic_write_unit_min: 0,
            stx_atomic_write_unit_max: 0,
            stx_atomic_write_segments_max: 0,
            stx_dio_read_offset_align: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Encode, Decode)]
#[repr(C)]
pub struct StatxTimestamp {
    pub tv_sec: i64,
    pub tv_nsec: u32,
}
impl StatxTimestamp {
    pub fn to_timespec(self) -> Timespec {
        Timespec {
            tv_sec: self.tv_sec,
            tv_nsec: self.tv_nsec as _,
        }
    }
}
impl From<Timespec> for StatxTimestamp {
    fn from(value: Timespec) -> Self {
        Self {
            tv_sec: value.tv_sec,
            tv_nsec: value.tv_nsec as _,
        }
    }
}

#[derive(Debug, Clone, Copy, Encode, Decode)]
#[repr(transparent)]
pub struct FileMode(pub u16);
impl FileMode {
    pub const S_IFMT: u16 = 0o170000;
    pub const S_IFDIR: u16 = 0o40000;
    pub const S_IFCHR: u16 = 0o20000;
    pub const S_IFBLK: u16 = 0o60000;
    pub const S_IFREG: u16 = 0o100000;
    pub const S_IFIFO: u16 = 0o10000;
    pub const S_IFLNK: u16 = 0o120000;
    pub const S_IFSOCK: u16 = 0o140000;

    pub const fn file_type(self) -> FileType {
        let file_type = self.0 & Self::S_IFMT;
        match file_type {
            Self::S_IFDIR => FileType::Directory,
            Self::S_IFCHR => FileType::CharDevice,
            Self::S_IFBLK => FileType::BlockDevice,
            Self::S_IFREG => FileType::RegularFile,
            Self::S_IFIFO => FileType::Fifo,
            Self::S_IFLNK => FileType::Symlink,
            Self::S_IFSOCK => FileType::Socket,
            _ => FileType::Unknown,
        }
    }

    pub const fn set_file_type(&mut self, file_type: FileType) {
        self.0 &= !Self::S_IFMT;
        self.0 &= match file_type {
            FileType::Directory => Self::S_IFDIR,
            FileType::CharDevice => Self::S_IFCHR,
            FileType::BlockDevice => Self::S_IFBLK,
            FileType::RegularFile => Self::S_IFREG,
            FileType::Fifo => Self::S_IFIFO,
            FileType::Symlink => Self::S_IFLNK,
            FileType::Socket => Self::S_IFSOCK,
            FileType::Unknown => 0,
        };
    }

    pub const fn permbits(self) -> u16 {
        self.0 & !Self::S_IFMT
    }
}
impl FromApple for FileMode {
    type Apple = u16;

    fn from_apple(apple: Self::Apple) -> Result<Self, LxError> {
        Ok(Self(apple))
    }
}
impl ToApple for FileMode {
    type Apple = u16;

    fn to_apple(self) -> Result<Self::Apple, LxError> {
        Ok(self.0)
    }
}

/// A type for representing file types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileType {
    Directory,
    CharDevice,
    BlockDevice,
    RegularFile,
    Fifo,
    Symlink,
    Socket,
    Unknown,
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct StatxAttrs: u64 {}
}
impl_bincode_for_bitflags!(StatxAttrs: u64);

bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct StatxMask: u32 {
        const STATX_TYPE = 1;
        const STATX_MODE = 2;
        const STATX_NLINK = 4;
        const STATX_UID = 8;
        const STATX_GID = 16;
        const STATX_ATIME = 32;
        const STATX_MTIME = 64;
        const STATX_CTIME = 128;
        const STATX_INO = 256;
        const STATX_SIZE = 512;
        const STATX_BLOCKS = 1024;
        const STATX_BTIME = 2048;
        const STATX_MNT_ID = 4096;
        const STATX_DIOALIGN = 8192;
        const STATX_MNT_ID_UNIQUE = 16384;
        const STATX_SUBVOL = 32768;
        const STATX_WRITE_ATOMIC = 65536;
        const STATX_DIO_READ_ALIGN = 131072;
        const STATX_BASIC_STATS = 2047;
    }
}
impl_bincode_for_bitflags!(StatxMask: u32);
