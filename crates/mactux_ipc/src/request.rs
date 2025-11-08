use crate::types::NetworkNames;
use bincode::{Decode, Encode};
use std::time::Duration;
use structures::{
    device::DeviceNumber,
    fs::{AccessFlags, FileMode, OpenHow, UmountFlags},
    io::{FcntlCmd, IoctlCmd, Whence},
    misc::LogLevel,
};

/// An uninterruptible MacTux IPC request.
#[derive(Debug, Clone, Encode, Decode)]
pub enum Request {
    SetMountNamespace(u64),
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
    VfdStat(u64),
    VfdTruncate(u64, u64),
    VfdChown(u64, u32, u32),
    VfdDup(u64),
    VfdClose(u64),
    VfdOrigPath(u64),
    VfdSync(u64),
    VfdReadlink(u64),

    EventFd(u64, u32),

    GetNetworkNames,
    SetNetworkNames(NetworkNames),
    SysInfo,

    WriteSyslog(LogLevel, Vec<u8>),

    AfterFork(i32),
    AfterExec,

    GetThreadName,
    SetThreadName(Vec<u8>),

    CallInterruptible(InterruptibleRequest),
}

/// An interruptible MacTux IPC request.
#[derive(Debug, Clone, Encode, Decode)]
pub enum InterruptibleRequest {
    VirtualFdPoll(Vec<(u64, u16)>, Option<Duration>),
}
