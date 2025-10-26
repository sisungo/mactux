//! UTS and system information.

use std::sync::RwLock;
use structures::error::LxError;

pub trait UtsNamespace: Send + Sync {
    fn nodename(&self) -> Vec<u8>;
    fn set_nodename(&self, name: Vec<u8>) -> Result<(), LxError>;

    fn domainname(&self) -> Vec<u8>;
    fn set_domainname(&self, name: Vec<u8>) -> Result<(), LxError>;
}

#[derive(Debug)]
pub struct InitUts;
impl UtsNamespace for InitUts {
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

    fn set_nodename(&self, _name: Vec<u8>) -> Result<(), LxError> {
        Err(LxError::EPERM)
    }

    fn domainname(&self) -> Vec<u8> {
        self.nodename()
    }

    fn set_domainname(&self, _name: Vec<u8>) -> Result<(), LxError> {
        Err(LxError::EPERM)
    }
}

#[derive(Debug)]
pub struct CustomUts {
    nodename: RwLock<Vec<u8>>,
    domainname: RwLock<Vec<u8>>,
}
impl UtsNamespace for CustomUts {
    fn nodename(&self) -> Vec<u8> {
        self.nodename.read().unwrap().clone()
    }

    fn set_nodename(&self, name: Vec<u8>) -> Result<(), LxError> {
        *self.nodename.write().unwrap() = name;
        Ok(())
    }

    fn domainname(&self) -> Vec<u8> {
        self.domainname.read().unwrap().clone()
    }

    fn set_domainname(&self, name: Vec<u8>) -> Result<(), LxError> {
        *self.domainname.write().unwrap() = name;
        Ok(())
    }
}
