use structures::{error::LxError, mapper::PidMapper};

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

/// Converts a POSIX function that returns something like what `close()` returns to [`Result<(), LxError>`] in Rust.
#[macro_export]
macro_rules! posix_bi {
    ($x:expr) => {
        match $x {
            -1 => Err(LxError::last_apple_error()),
            _ => Ok(()),
        }
    };
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
