//! UTS and system information.

use crate::util::sysctl_read;
use libc::host_statistics64;
use mach2::{
    mach_init::mach_host_self, mach_port::mach_port_deallocate,
    vm_statistics::vm_statistics64_data_t,
};
use std::sync::RwLock;
use structures::{error::LxError, misc::SysInfo, time::Timespec};

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
            let mut nodename = vec![0; 65];
            if libc::gethostname(nodename.as_mut_ptr().cast(), nodename.len()) == -1 {
                return b"localhost".into();
            }
            nodename
        }
    }

    fn set_nodename(&self, name: Vec<u8>) -> Result<(), LxError> {
        unsafe {
            match libc::sethostname(name.as_ptr().cast(), name.len() as _) {
                -1 => Err(LxError::last_apple_error()),
                _ => Ok(()),
            }
        }
    }

    fn domainname(&self) -> Vec<u8> {
        unsafe {
            let mut domainname = vec![0; 65];
            if libc::getdomainname(domainname.as_mut_ptr().cast(), domainname.len() as _) == -1 {
                return b"localhost.local".into();
            }
            domainname
        }
    }

    fn set_domainname(&self, name: Vec<u8>) -> Result<(), LxError> {
        unsafe {
            match libc::setdomainname(name.as_ptr().cast(), name.len() as _) {
                -1 => Err(LxError::last_apple_error()),
                _ => Ok(()),
            }
        }
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

/// Retrieves [`SysInfo`] information.
pub fn sysinfo() -> Result<SysInfo, LxError> {
    let mem_info = MemInfo::acquire()?;
    let boottime = boottime()?;

    Ok(SysInfo {
        uptime: Timespec::now().tv_sec - boottime.tv_sec,
        loads: [0; 3],
        totalram: mem_info.total_ram as _,
        freeram: mem_info.free_ram as _,
        sharedram: 0,
        bufferram: 0,
        totalswap: mem_info.total_swap as _,
        freeswap: mem_info.free_swap as _,
        procs: 0,
        totalhigh: 0,
        freehigh: 0,
        mem_unit: 0,
    })
}

/// Memory information.
#[derive(Debug, Clone)]
pub struct MemInfo {
    pub total_ram: usize,
    pub free_ram: usize,
    pub avail_ram: usize,
    pub active: usize,
    pub inactive: usize,
    pub total_swap: usize,
    pub free_swap: usize,
}
impl MemInfo {
    #[cfg(target_arch = "x86_64")]
    pub const PAGE_SIZE: usize = 0x1000;

    #[cfg(target_arch = "aarch64")]
    pub const PAGE_SIZE: usize = 0x4000;

    /// Acquires memory information from the host system.
    pub fn acquire() -> Result<Self, LxError> {
        let vm_info = mach_host_vm_info()?;
        let swap_usage = swap_usage()?;

        let total_ram = (vm_info.free_count
            + vm_info.active_count
            + vm_info.inactive_count
            + vm_info.wire_count
            + vm_info.speculative_count
            + vm_info.compressor_page_count) as usize
            * Self::PAGE_SIZE;
        let free_ram = vm_info.free_count as usize * Self::PAGE_SIZE;
        let avail_ram = total_ram
            - (vm_info.internal_page_count + vm_info.wire_count) as usize * Self::PAGE_SIZE;
        let active = vm_info.active_count as usize * Self::PAGE_SIZE;
        let inactive = vm_info.inactive_count as usize * Self::PAGE_SIZE;

        let total_swap = swap_usage.xsu_total as usize;
        let free_swap = swap_usage.xsu_avail as usize;

        Ok(Self {
            total_ram,
            free_ram,
            avail_ram,
            active,
            inactive,
            total_swap,
            free_swap,
        })
    }
}

/// Retrieves Mach VM statistics.
fn mach_host_vm_info() -> Result<vm_statistics64_data_t, LxError> {
    unsafe {
        let mut vm_stat: vm_statistics64_data_t = std::mem::zeroed();
        let mut vm_statcnt = libc::HOST_VM_INFO64_COUNT;
        let host = mach_host_self();
        let status = host_statistics64(
            host,
            libc::HOST_VM_INFO64,
            (&raw mut vm_stat).cast(),
            &mut vm_statcnt,
        );
        mach_port_deallocate(mach2::traps::mach_task_self(), host);
        match status {
            libc::KERN_SUCCESS => Ok(vm_stat),
            _ => Err(LxError::EPERM),
        }
    }
}

/// Retrieves swap usage information.
fn swap_usage() -> Result<libc::xsw_usage, LxError> {
    unsafe { sysctl_read([libc::CTL_VM, libc::VM_SWAPUSAGE]) }
}

/// Retrieves the system boot time.
fn boottime() -> Result<libc::timeval, LxError> {
    unsafe { sysctl_read([libc::CTL_KERN, libc::KERN_BOOTTIME]) }
}
