use std::sync::RwLock;
use structures::error::LxError;

pub trait UtsNamespace: Send + Sync {
    fn set_nodename(&self, new: &[u8]) -> Result<(), LxError>;
    fn set_domainname(&self, new: &[u8]) -> Result<(), LxError>;
    fn nodename(&self) -> Vec<u8>;
    fn domainname(&self) -> Vec<u8>;
}

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
