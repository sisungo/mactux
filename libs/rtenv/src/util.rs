use std::{ffi::c_int, panic::PanicHookInfo};
use structures::{error::LxError, mapper::PidMapper, misc::LogLevel};

/// Converts a POSIX function that returns something like what `read()`/`write()` returns to [`Result<Integer, LxError>`] in
/// Rust.
#[macro_export]
macro_rules! posix_num {
    ($x:expr) => {
        match $x {
            -1 => Err(LxError::last_apple_error()),
            n => Ok(n as _),
        }
    };
}

pub fn posix_result(x: c_int) -> Result<(), LxError> {
    match x {
        -1 => Err(LxError::last_apple_error()),
        _ => Ok(()),
    }
}

/// Converts from a Rust byte vector to a NUL-terminated C string.
pub fn c_path(mut dat: Vec<u8>) -> Vec<u8> {
    dat.push(0);
    dat
}

/// Fails the process with reason that the server is not giving an expected response.
pub fn ipc_fail() -> ! {
    panic!("unexpected server response");
}

#[derive(Debug, Clone, Copy)]
pub struct RtenvPidMapper;
impl PidMapper for RtenvPidMapper {
    // TODO
    fn apple_to_linux(&self, apple: libc::pid_t) -> Result<i32, LxError> {
        Ok(apple)
    }

    fn linux_to_apple(&self, linux: i32) -> Result<libc::pid_t, LxError> {
        Ok(linux)
    }
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
        crate::misc::write_syslog(level, content.into_bytes());
    }
}

pub fn panic_hook(info: &PanicHookInfo) {
    eprintln!(
        "mactux: process {} at module `{}` panicked: {}",
        std::process::id(),
        info.location()
            .map(ToString::to_string)
            .unwrap_or_else(|| "<unknown>".into()),
        info.payload_as_str().unwrap_or("no information provided"),
    );
}
