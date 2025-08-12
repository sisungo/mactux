use crate::util::now;
use libc::host_statistics64;
use mach2::{
    kern_return::KERN_SUCCESS, mach_init::mach_host_self, mach_port::mach_port_deallocate,
    vm_statistics::vm_statistics64_data_t,
};
use std::ffi::c_int;
use structures::{error::LxError, misc::SysInfo};

#[cfg(target_arch = "x86_64")]
const PAGE_SIZE: usize = 0x1000;

#[cfg(target_arch = "aarch64")]
const PAGE_SIZE: usize = 0x4000;

pub fn sysinfo() -> Result<SysInfo, LxError> {
    let mem_info = MemInfo::acquire()?;
    let boottime = boottime()?;

    Ok(SysInfo {
        uptime: now().tv_sec - boottime.tv_sec,
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
    pub fn acquire() -> Result<Self, LxError> {
        let vm_info = mach_host_vm_info()?;
        let swap_usage = swap_usage()?;

        let total_ram = (vm_info.free_count
            + vm_info.active_count
            + vm_info.inactive_count
            + vm_info.wire_count
            + vm_info.speculative_count
            + vm_info.compressor_page_count) as usize
            * PAGE_SIZE;
        let free_ram = vm_info.free_count as usize * PAGE_SIZE;
        let avail_ram =
            total_ram - (vm_info.internal_page_count + vm_info.wire_count) as usize * PAGE_SIZE;
        let active = vm_info.active_count as usize * PAGE_SIZE;
        let inactive = vm_info.inactive_count as usize * PAGE_SIZE;

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
            KERN_SUCCESS => Ok(vm_stat),
            _ => Err(LxError::EPERM),
        }
    }
}

fn swap_usage() -> Result<libc::xsw_usage, LxError> {
    unsafe { sysctl_read([libc::CTL_VM, libc::VM_SWAPUSAGE]) }
}

fn boottime() -> Result<libc::timeval, LxError> {
    unsafe { sysctl_read([libc::CTL_KERN, libc::KERN_BOOTTIME]) }
}

unsafe fn sysctl_read<T: Copy, const N: usize>(mut name: [c_int; N]) -> Result<T, LxError> {
    unsafe {
        let mut data: T = std::mem::zeroed();
        let mut size = size_of::<T>();
        match libc::sysctl(
            name.as_mut_ptr(),
            N as _,
            (&raw mut data).cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        ) {
            -1 => Err(LxError::EINVAL),
            _ => Ok(data),
        }
    }
}
