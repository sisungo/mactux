use crate::{
    device::DeviceNumber,
    error::LxError,
    fs::{
        AccessFlags, Dirent64, FileMode, OpenFlags, OpenHow, StatFs, Statx, StatxMask, UmountFlags,
    },
    io::{EventFdFlags, FcntlCmd, IoctlCmd, PollEvents, VfdAvailCtrl, Whence},
    misc::{LogLevel, SysInfo},
};
use bincode::{Decode, Encode};
use libc::c_int;
use std::time::Duration;

pub const PROTOCOL_VERSION: &str = "9999";

/// A handshake request.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HandshakeRequest {
    pub magic: [u8; 8],
}
impl HandshakeRequest {
    /// The magic number.
    pub const MAGIC: [u8; 8] = *b"MACTUXHQ";

    /// Creates a new [`HandshakeRequest`] instance, in its only valid form.
    pub fn new() -> Self {
        Self { magic: Self::MAGIC }
    }
}
impl Default for HandshakeRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// A handshake response.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HandshakeResponse {
    pub magic: [u8; 8],
    pub version: String,
}
impl HandshakeResponse {
    /// The magic number.
    pub const MAGIC: [u8; 8] = *b"MACTUXHS";

    /// Creates a new [`HandshakeResponse`] instance that fits current library version.
    pub fn new() -> Self {
        Self {
            magic: Self::MAGIC,
            version: PROTOCOL_VERSION.into(),
        }
    }
}
impl Default for HandshakeResponse {
    fn default() -> Self {
        Self::new()
    }
}

/// An uninterruptible MacTux IPC request.
#[derive(Debug, Clone, Encode, Decode)]
pub enum Request {
    SetMntNamespace(u64),
    SetPidNamespace(u64),
    SetUtsNamespace(u64),

    Umount(Vec<u8>, UmountFlags),

    Open(Vec<u8>, OpenHow),
    Access(Vec<u8>, AccessFlags),
    Unlink(Vec<u8>),
    Rmdir(Vec<u8>),
    Symlink(Vec<u8>, Vec<u8>),
    Rename(Vec<u8>, Vec<u8>),
    Link(Vec<u8>, Vec<u8>),
    Mkdir(Vec<u8>, FileMode),
    Mknod(Vec<u8>, FileMode, DeviceNumber),
    GetSockPath(Vec<u8>, bool),

    VfdRead(u64, usize),
    VfdPread(u64, i64, usize),
    VfdWrite(u64, Vec<u8>),
    VfdPwrite(u64, i64, Vec<u8>),
    VfdSeek(u64, Whence, i64),
    VfdIoctlQuery(u64, IoctlCmd),
    VfdIoctl(u64, IoctlCmd, Vec<u8>),
    VfdFcntl(u64, FcntlCmd, Vec<u8>),
    VfdGetdent(u64),
    VfdStat(u64, StatxMask),
    VfdTruncate(u64, u64),
    VfdChown(u64, u32, u32),
    VfdDup(u64),
    VfdClose(u64),
    VfdOrigPath(u64),
    VfdSync(u64),
    VfdReadlink(u64),

    EventFd(u64, EventFdFlags),
    InvalidFd(OpenFlags),

    GetNetworkNames,
    SetNetworkNames(NetworkNames),
    SysInfo,

    WriteSyslog(LogLevel, Vec<u8>),
    ReadSyslogAll(usize),

    AfterFork(i32),
    AfterExec,

    GetThreadName,
    SetThreadName(Vec<u8>),

    PidNativeToLinux(i32),
    PidLinuxToNative(i32),

    CallInterruptible(InterruptibleRequest),
}

/// An interruptible MacTux IPC request.
#[derive(Debug, Clone, Encode, Decode)]
pub enum InterruptibleRequest {
    VfdPoll(Vec<(u64, PollEvents)>, Option<Duration>),
}

/// A response to a MacTux IPC request.
#[derive(Debug, Clone, Encode, Decode)]
pub enum Response {
    Nothing,
    NativePath(Vec<u8>),
    LxPath(Vec<u8>),
    Vfd(u64),
    Pid(i32),
    Bytes(Vec<u8>),
    Length(usize),
    Offset(i64),
    CtrlOutput(CtrlOutput),
    VfdAvailCtrl(VfdAvailCtrl),
    Stat(Box<Statx>),
    Dirent64(Dirent64),
    NetworkNames(NetworkNames),
    SysInfo(Box<SysInfo>),
    StatFs(Box<StatFs>),
    Poll(u64, PollEvents),
    Error(LxError),
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct CtrlOutput {
    pub status: c_int,
    pub blob: Vec<u8>,
}

/// Network names of current UTS namespace.
#[derive(Debug, Clone, Encode, Decode)]
pub struct NetworkNames {
    pub nodename: Vec<u8>,
    pub domainname: Vec<u8>,
}
