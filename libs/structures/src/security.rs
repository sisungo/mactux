use crate::error::LxError;

#[derive(Debug, Clone)]
pub struct UserCap {
    pub version: UserCapVersion,
    pub pid: i32,
    pub effective: [u32; 2],
    pub permitted: [u32; 2],
    pub inheritable: [u32; 2],
}
impl UserCap {
    pub unsafe fn read_from(
        header: *const UserCapHeader,
        data: *const UserCapData,
    ) -> Result<Self, LxError> {
        unsafe {
            let header = header.read();
            let mut effective = [0; 2];
            let mut permitted = [0; 2];
            let mut inheritable = [0; 2];
            for i in 0..header.version.u32s()? {
                let data = data.add(i).read();
                effective[i] = data.effective;
                permitted[i] = data.permitted;
                inheritable[i] = data.inheritable;
            }
            Ok(Self {
                version: header.version,
                pid: header.pid,
                effective,
                permitted,
                inheritable,
            })
        }
    }

    pub unsafe fn write_to(&self, headerp: *mut UserCapHeader, datap: *mut UserCapData) {
        unsafe {
            let header = UserCapHeader {
                version: self.version,
                pid: self.pid,
            };
            headerp.write(header.clone());
            let data = [
                UserCapData {
                    effective: self.effective[0],
                    permitted: self.permitted[0],
                    inheritable: self.inheritable[0],
                },
                UserCapData {
                    effective: self.effective[1],
                    permitted: self.permitted[1],
                    inheritable: self.inheritable[1],
                },
            ];
            if !datap.is_null() {
                datap.copy_from(data.as_ptr(), header.version.u32s().unwrap());
            }
        }
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct UserCapHeader {
    pub version: UserCapVersion,
    pub pid: i32,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct UserCapData {
    pub effective: u32,
    pub permitted: u32,
    pub inheritable: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct UserCapVersion(pub u32);
impl UserCapVersion {
    pub const LINUX_CAPABILITY_VERSION_1: Self = Self(0x19980330);
    pub const LINUX_CAPABILITY_VERSION_2: Self = Self(0x20071026);
    pub const LINUX_CAPABILITY_VERSION_3: Self = Self(0x20080522);

    pub const fn u32s(self) -> Result<usize, LxError> {
        match self {
            Self::LINUX_CAPABILITY_VERSION_1 => Ok(1),
            Self::LINUX_CAPABILITY_VERSION_2 | Self::LINUX_CAPABILITY_VERSION_3 => Ok(2),
            _ => Err(LxError::EINVAL),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapId(pub u32);
impl CapId {}
