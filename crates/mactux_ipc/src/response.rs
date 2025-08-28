use bincode::{Decode, Encode};
use structures::{
    error::LxError,
    fs::{Dirent64, Statx},
    misc::SysInfo,
};

/// A response to a MacTux IPC request.
#[derive(Debug, Clone, Encode, Decode)]
pub enum Response {
    Nothing,

    OpenNativePath(Vec<u8>),
    OpenVirtualFd(u64),
    SockPath(Vec<u8>),
    EventFd(u64),

    Read(Vec<u8>),
    Write(usize),
    Lseek(u64),
    Ctrl(u64),
    CtrlBlob(u64, Vec<u8>),
    DupVirtualFd(u64),
    OrigPath(Vec<u8>),
    VirtualFdAvailCtrl(VirtualFdAvailCtrl),
    Stat(Statx),
    Dirent64(Dirent64),

    NetworkNames(NetworkNames),
    SysInfo(SysInfo),

    Poll(u64, u16),

    Error(LxError),
}

/// Information about a virtual file descriptor's specific "ioctl" availability.
#[derive(Debug, Clone, Encode, Decode)]
pub struct VirtualFdAvailCtrl {
    pub in_size: isize,
    pub out_size: usize,
}

/// Network names of current UTS namespace.
#[derive(Debug, Clone, Encode, Decode)]
pub struct NetworkNames {
    pub nodename: Vec<u8>,
    pub domainname: Vec<u8>,
}
