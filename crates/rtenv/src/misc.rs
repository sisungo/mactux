use crate::{ipc_client::with_client, util::ipc_fail};
use mactux_ipc::{request::Request, response::Response, types::NetworkNames};
use structures::{
    error::LxError,
    misc::{LogLevel, SysInfo, UtsName, uname_str},
};

pub fn sysinfo() -> Result<SysInfo, LxError> {
    with_client(|client| match client.invoke(Request::SysInfo).unwrap() {
        Response::SysInfo(sysinfo) => Ok(sysinfo),
        Response::Error(err) => Err(err),
        _ => ipc_fail(),
    })
}

pub fn uname() -> Result<UtsName, LxError> {
    let (mut rnodename, mut rdomainname) = get_network_names()?;
    rnodename.truncate(64);
    rdomainname.truncate(64);
    let mut nodename = [0; _];
    let mut domainname = [0; _];
    nodename[..rnodename.len()].copy_from_slice(&rnodename);
    domainname[..rdomainname.len()].copy_from_slice(&rdomainname);

    Ok(UtsName {
        sysname: uname_str(b"Linux").unwrap(),
        nodename,
        release: release(),
        version: version(),
        machine: machine(),
        domainname,
    })
}

pub fn get_network_names() -> Result<(Vec<u8>, Vec<u8>), LxError> {
    let network_names =
        with_client(
            |client| match client.invoke(Request::GetNetworkNames).unwrap() {
                Response::NetworkNames(names) => Ok(names),
                Response::Error(err) => Err(err),
                _ => ipc_fail(),
            },
        )?;
    Ok((network_names.nodename, network_names.domainname))
}

pub fn set_network_names(nodename: Vec<u8>, domainname: Vec<u8>) -> Result<(), LxError> {
    with_client(|client| {
        match client
            .invoke(Request::SetNetworkNames(NetworkNames {
                nodename,
                domainname,
            }))
            .unwrap()
        {
            Response::Nothing => Ok(()),
            Response::Error(err) => Err(err),
            _ => ipc_fail(),
        }
    })
}

pub fn write_syslog(level: LogLevel, content: Vec<u8>) {
    with_client(|client| {
        client.invoke(Request::WriteSyslog(level, content)).unwrap();
    });
}

#[derive(Debug)]
pub struct RustLogger;
impl log::Log for RustLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn flush(&self) {}

    fn log(&self, record: &log::Record) {
        let level = match record.level() {
            log::Level::Trace => LogLevel::KERN_DEBUG,
            log::Level::Debug => LogLevel::KERN_INFO,
            log::Level::Info => LogLevel::KERN_NOTICE,
            log::Level::Warn => LogLevel::KERN_WARNING,
            log::Level::Error => LogLevel::KERN_ERR,
        };
        let content = format!(
            "{}: {}",
            record.module_path().unwrap_or("mactux"),
            record.args()
        );
        write_syslog(level, content.into_bytes());
    }
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
