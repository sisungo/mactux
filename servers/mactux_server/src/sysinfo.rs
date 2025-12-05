//! UTS and system information.

use crate::{app, util::sysctl_read};
use libc::{
    host_cpu_load_info_data_t, host_processor_info, host_statistics, host_statistics64,
    processor_cpu_load_info_data_t, processor_info_array_t,
};
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
            let mut nodename = vec![0; 256];
            if libc::gethostname(nodename.as_mut_ptr().cast(), nodename.len()) == -1 {
                return b"localhost".into();
            }
            let zero_pos = nodename
                .iter()
                .enumerate()
                .find(|(_, x)| **x == 0)
                .map(|x| x.0)
                .unwrap_or(255);
            nodename.truncate(zero_pos);
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
            let mut domainname = vec![0; 256];
            if libc::getdomainname(domainname.as_mut_ptr().cast(), domainname.len() as _) == -1 {
                return b"localhost.local".into();
            }
            let zero_pos = domainname
                .iter()
                .enumerate()
                .find(|(_, x)| **x == 0)
                .map(|x| x.0)
                .unwrap_or(255);
            domainname.truncate(zero_pos);
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
    let boottime = boot_time()?;
    let loads = loadavg()?;
    let loads = [loads[0] as _, loads[1] as _, loads[2] as _];

    Ok(SysInfo {
        uptime: Timespec::now().tv_sec - boottime.tv_sec,
        loads,
        totalram: mem_info.total_ram as _,
        freeram: mem_info.free_ram() as _,
        sharedram: 0,
        bufferram: 0,
        totalswap: mem_info.swap_usage.xsu_total,
        freeswap: mem_info.swap_usage.xsu_avail,
        procs: app().processes.len() as _,
        totalhigh: 0,
        freehigh: 0,
        mem_unit: 1,
    })
}

/// Memory information acquired from macOS.
#[derive(Debug, Clone)]
pub struct MemInfo {
    pub total_ram: usize,
    pub swap_usage: libc::xsw_usage,
    pub vm_statistics: vm_statistics64_data_t,
}
impl MemInfo {
    /// Acquires memory information from the host system.
    pub fn acquire() -> Result<Self, LxError> {
        Ok(Self {
            total_ram: total_ram()? as _,
            swap_usage: swap_usage()?,
            vm_statistics: mach_host_vm_info()?,
        })
    }

    pub fn free_ram(&self) -> usize {
        self.vm_statistics.free_count as usize * page_size()
    }

    pub fn avail_ram(&self) -> usize {
        (self.vm_statistics.free_count
            + self.vm_statistics.active_count
            + self.vm_statistics.inactive_count
            + self.vm_statistics.wire_count
            + self.vm_statistics.speculative_count
            + self.vm_statistics.compressor_page_count
            - self.vm_statistics.internal_page_count
            - self.vm_statistics.wire_count) as usize
            * page_size()
    }
}

pub const fn page_size() -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        0x1000
    }

    #[cfg(target_arch = "aarch64")]
    {
        0x4000
    }
}

/// Retrieves the system boot time.
pub fn boot_time() -> Result<libc::timeval, LxError> {
    unsafe { sysctl_read([libc::CTL_KERN, libc::KERN_BOOTTIME]) }
}

/// Retrieves system load average.
pub fn loadavg() -> Result<[f64; 3], LxError> {
    let mut result = [0.; 3];
    unsafe {
        if libc::getloadavg(result.as_mut_ptr(), 3) == -1 {
            return Err(LxError::last_apple_error());
        }
    }
    Ok(result)
}

/// Retrieves Mach CPU statistics.
pub fn mach_host_cpu_load_info() -> Result<host_cpu_load_info_data_t, LxError> {
    unsafe {
        let mut cpu_load_info: host_cpu_load_info_data_t = std::mem::zeroed();
        let mut cnt = libc::HOST_CPU_LOAD_INFO_COUNT;
        let host = mach_host_self();
        let status = host_statistics(
            host,
            libc::HOST_CPU_LOAD_INFO,
            (&raw mut cpu_load_info).cast(),
            &mut cnt,
        );
        mach_port_deallocate(mach2::traps::mach_task_self(), host);
        match status {
            libc::KERN_SUCCESS => Ok(cpu_load_info),
            _ => Err(LxError::EPERM),
        }
    }
}

/// Retrieves Mach CPU statistics.
pub fn mach_cpu_core_load_info() -> Result<processor_info_array_t, LxError> {
    unsafe {
        let mut proc_cnt = 0;
        let mut info_cnt = 0;
        let mut result: processor_info_array_t = std::mem::zeroed();
        let host = mach_host_self();
        let status = host_processor_info(
            host,
            libc::PROCESSOR_CPU_LOAD_INFO,
            &mut proc_cnt,
            (&raw mut result).cast(),
            &mut info_cnt,
        );
        mach_port_deallocate(mach2::traps::mach_task_self(), host);
        match status {
            libc::KERN_SUCCESS => Ok(result),
            _ => Err(LxError::EPERM),
        }
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

/// Retrieves swap usage information.
fn total_ram() -> Result<u64, LxError> {
    unsafe { sysctl_read([libc::CTL_HW, libc::HW_MEMSIZE]) }
}
