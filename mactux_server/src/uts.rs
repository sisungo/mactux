//! Support of UTS namespace.

use std::sync::RwLock;
use structures::error::LxError;

/// An UTS namespace.
pub trait UtsNamespace: Send + Sync {
    /// Sets the nodename.
    fn set_nodename(&self, new: &[u8]) -> Result<(), LxError>;

    /// Sets the domainname.
    fn set_domainname(&self, new: &[u8]) -> Result<(), LxError>;

    /// Gets the nodename.
    fn nodename(&self) -> Vec<u8>;

    /// Gets the domainname.
    fn domainname(&self) -> Vec<u8>;
}

/// The initial UTS namespace, mapping to the underlying macOS host directly.
#[derive(Debug)]
pub struct InitUts;
impl UtsNamespace for InitUts {
    fn set_nodename(&self, new: &[u8]) -> Result<(), LxError> {
        Err(LxError::EPERM)
    }

    fn set_domainname(&self, new: &[u8]) -> Result<(), LxError> {
        Err(LxError::EPERM)
    }

    fn nodename(&self) -> Vec<u8> {
        unsafe {
            let mut utsname = std::mem::zeroed();
            libc::uname(&mut utsname);
            utsname.nodename[..65]
                .iter()
                .map(|x| *x as u8)
                .filter(|x| *x != 0)
                .collect()
        }
    }

    fn domainname(&self) -> Vec<u8> {
        self.nodename()
    }
}

/// A custom UTS namespace, storing the values in server memory.
#[derive(Debug)]
pub struct CustomUts {
    nodename: RwLock<Vec<u8>>,
    domainname: RwLock<Vec<u8>>,
}
impl CustomUts {
    pub fn from_ns<T: UtsNamespace>(value: T) -> Self {
        Self {
            nodename: RwLock::new(value.nodename()),
            domainname: RwLock::new(value.domainname()),
        }
    }
}
impl UtsNamespace for CustomUts {
    fn nodename(&self) -> Vec<u8> {
        self.nodename.read().unwrap().clone()
    }

    fn domainname(&self) -> Vec<u8> {
        self.domainname.read().unwrap().clone()
    }

    fn set_nodename(&self, new: &[u8]) -> Result<(), LxError> {
        *self.nodename.write().unwrap() = new.to_vec();
        Ok(())
    }

    fn set_domainname(&self, new: &[u8]) -> Result<(), LxError> {
        *self.domainname.write().unwrap() = new.to_vec();
        Ok(())
    }
}
