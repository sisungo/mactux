use crate::{
    ipc_client::{call_server, with_client},
    util::ipc_fail,
};
use structures::{
    error::LxError,
    mactux_ipc::{NetworkNames, Request, Response},
    misc::{LogLevel, SysInfo, UtsName, uname_str},
};

pub fn sysinfo() -> Result<SysInfo, LxError> {
    call_server(Request::SysInfo)
}

pub fn uname() -> Result<UtsName, LxError> {
    let mut network_names = get_network_names()?;
    network_names.nodename.truncate(64);
    network_names.domainname.truncate(64);
    let mut nodename = [0; _];
    let mut domainname = [0; _];
    nodename[..network_names.nodename.len()].copy_from_slice(&network_names.nodename);
    domainname[..network_names.domainname.len()].copy_from_slice(&network_names.domainname);

    Ok(UtsName {
        sysname: uname_str(b"Linux").unwrap(),
        nodename,
        release: release(),
        version: version(),
        machine: machine(),
        domainname,
    })
}

pub fn get_network_names() -> Result<NetworkNames, LxError> {
    call_server(Request::GetNetworkNames)
}

pub fn set_network_names(names: NetworkNames) -> Result<(), LxError> {
    call_server(Request::SetNetworkNames(names))
}

pub fn write_syslog(level: LogLevel, content: Vec<u8>) {
    call_server(Request::WriteSyslog(level, content))
}

pub fn read_syslog_all(buf: &mut [u8]) -> Result<usize, LxError> {
    with_client(
        |client| match client.invoke(Request::ReadSyslogAll(buf.len())).unwrap() {
            Response::Bytes(blob) => {
                debug_assert!(blob.len() <= buf.len());
                buf[..blob.len()].copy_from_slice(&blob);
                Ok(blob.len())
            }
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        },
    )
}

fn machine() -> [u8; 65] {
    if cfg!(target_arch = "x86_64") {
        uname_str(b"x86_64").unwrap()
    } else if cfg!(target_arch = "aarch64") {
        uname_str(b"aarch64").unwrap()
    } else {
        uname_str(b"unknown").unwrap()
    }
}

fn release() -> [u8; 65] {
    uname_str(b"6.15.8-11-generic").unwrap()
}

fn version() -> [u8; 65] {
    uname_str(b"#1~0.1.0-MacTux SMP Fri Aug 28 14:01:22 UTC 2025").unwrap()
}
