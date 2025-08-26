use bincode::{Decode, Encode};
use std::time::Duration;
use structures::io::Whence;

#[derive(Debug, Clone, Encode, Decode)]
pub enum Request {
    SetMountNamespace(u64),
    Umount(Vec<u8>, u32),

    Open(Vec<u8>, u32, u32),
    Access(Vec<u8>, u32),
    Unlink(Vec<u8>),
    Rmdir(Vec<u8>),
    Symlink(Vec<u8>, Vec<u8>),
    Rename(Vec<u8>, Vec<u8>),
    Mkdir(Vec<u8>, u32),
    GetSockPath(Vec<u8>, bool),

    VirtualFdRead(u64, usize),
    VirtualFdPread(u64, i64, usize),
    VirtualFdWrite(u64, Vec<u8>),
    VirtualFdPwrite(u64, i64, Vec<u8>),
    VirtualFdLseek(u64, Whence, i64),
    VirtualFdIoctlQuery(u64, u32),
    VirtualFdIoctl(u64, u32, Vec<u8>),
    VirtualFdFcntl(u64, u32, Vec<u8>),
    VirtualFdGetDents64(u64),
    VirtualFdStat(u64),
    VirtualFdTruncate(u64, u64),
    VirtualFdChown(u64, u32, u32),
    VirtualFdDup(u64),
    VirtualFdClose(u64),
    VirtualFdOrigPath(u64),

    EventFd(u64, u32),

    GetNetworkNames,
    SetNetworkNames(Vec<u8>, Vec<u8>),
    SysInfo,

    WriteSyslog(Vec<u8>),

    BeforeFork,
    AfterFork(i32),
    AfterExec,

    CallInterruptible(InterruptibleRequest),
}

#[derive(Debug, Clone, Encode, Decode)]
pub enum InterruptibleRequest {
    VirtualFdPoll(Vec<(u64, u16)>, Option<Duration>),
}
