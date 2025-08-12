use crate::error::LxError;
use std::sync::{LazyLock, RwLock};

static PID_MAPPER: LazyLock<RwLock<Box<dyn PidMapper>>> =
    LazyLock::new(|| RwLock::new(Box::new(IdentityMapper)));

pub trait PidMapper: Send + Sync {
    fn apple_to_linux(&self, apple: libc::pid_t) -> Result<i32, LxError>;
    fn linux_to_apple(&self, linux: i32) -> Result<libc::pid_t, LxError>;
}

pub fn with_pid_mapper<T>(f: impl FnOnce(&dyn PidMapper) -> T) -> T {
    f(&**PID_MAPPER.read().unwrap())
}

pub fn set_pid_mapper(val: Box<dyn PidMapper>) {
    *PID_MAPPER.write().unwrap() = val;
}

#[derive(Debug)]
struct IdentityMapper;
impl PidMapper for IdentityMapper {
    fn apple_to_linux(&self, apple: libc::pid_t) -> Result<i32, LxError> {
        Ok(apple)
    }

    fn linux_to_apple(&self, linux: i32) -> Result<libc::pid_t, LxError> {
        Ok(linux)
    }
}
