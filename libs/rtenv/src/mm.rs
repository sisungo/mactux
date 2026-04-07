use crate::util::posix_result;
use libc::c_int;
use mach2::{
    message::mach_msg_type_number_t,
    port::mach_port_t,
    vm_region::{vm_region_basic_info_data_64_t, vm_region_basic_info_data_t, vm_region_info_t},
    vm_types::mach_vm_size_t,
};
use structures::{
    ToApple,
    error::LxError,
    mm::{Madvice, MmapFlags, MmapProt, MremapFlags},
};

pub unsafe fn map(
    addr: *mut u8,
    len: usize,
    prot: MmapProt,
    flags: MmapFlags,
    fd: c_int,
    offset: i64,
) -> Result<*mut u8, LxError> {
    unsafe {
        // `MAP_SYNC` applies to DAX only, but macOS has no DAX support.
        if flags.contains(MmapFlags::MAP_SYNC) {
            return Err(LxError::EOPNOTSUPP);
        }

        let addr: *mut u8 = match libc::mmap(
            addr.cast(),
            len,
            prot.to_apple()?,
            flags.to_apple()?,
            fd,
            offset,
        ) {
            libc::MAP_FAILED => Err(LxError::last_apple_error()),
            addr => Ok(addr.cast()),
        }?;

        // Lock memory if required. The man pages say errors are ignored silently.
        if flags.contains(MmapFlags::MAP_LOCKED) {
            libc::mlock(addr.cast(), len);
        }

        Ok(addr)
    }
}

pub unsafe fn unmap(addr: *mut u8, len: usize) -> Result<(), LxError> {
    unsafe { posix_result(libc::munmap(addr.cast(), len)) }
}

pub unsafe fn remap(
    old_addr: *mut u8,
    old_size: usize,
    new_addr: *mut u8,
    new_size: usize,
    flags: MremapFlags,
) -> Result<*mut u8, LxError> {
    unsafe {
        // TODO: this implementation is very incomplete
        let mut mmap_flags = MmapFlags::MAP_PRIVATE | MmapFlags::MAP_ANON;
        if flags.contains(MremapFlags::MREMAP_FIXED) {
            mmap_flags |= MmapFlags::MAP_FIXED;
        }
        if !flags.contains(MremapFlags::MREMAP_MAYMOVE) {
            return Err(LxError::ENOMEM);
        }
        if !flags.contains(MremapFlags::MREMAP_DONTUNMAP) {
            return Err(LxError::EINVAL);
        }
        let new_addr = match libc::mmap(
            new_addr.cast(),
            new_size,
            (MmapProt::PROT_READ | MmapProt::PROT_WRITE).to_apple()?,
            mmap_flags.to_apple()?,
            -1,
            0,
        ) {
            libc::MAP_FAILED => Err(std::io::Error::last_os_error()),
            addr => Ok(addr),
        }?;
        new_addr.copy_from(old_addr.cast(), old_size.min(new_size));
        if new_addr != old_addr.cast() {
            libc::munmap(old_addr.cast(), old_size);
        }

        Ok(new_addr.cast())
    }
}

pub unsafe fn advise(start: *mut u8, len: usize, advice: Madvice) -> Result<(), LxError> {
    if let Ok(apple_advice) = advice.to_apple() {
        unsafe {
            return posix_result(libc::madvise(start.cast(), len, apple_advice));
        }
    }
    match advice {
        Madvice::MADV_MERGEABLE => Ok(()),
        Madvice::MADV_UNMERGEABLE => Ok(()),
        Madvice::MADV_COLD => Ok(()),
        Madvice::MADV_PAGEOUT => Ok(()),
        _ => Err(LxError::EINVAL),
    }
}

pub fn incore(addr: *const u8, size: usize, vec: *mut u8) -> Result<(), LxError> {
    // Linux man pages says `-ENOMEM` is returned if the region contains pages that are not mapped, and
    // certain applications (e.g. GNU grep running on glibc) depends on the behavior, so check for
    // the memory map first.
    for i in (0..size.next_multiple_of(0x1000)).step_by(0x1000) {
        let start = (addr as usize + i) as *const u8;
        let region = mach_vm_region(start);
        if region.is_none() {
            return Err(LxError::ENOMEM);
        }
        let region = region.unwrap();
        if region.info.max_protection == 0 {
            return Err(LxError::ENOMEM);
        }
        if region.addr as usize + region.size > addr as usize + size {
            break;
        }
    }

    unsafe { posix_result(libc::mincore(addr.cast(), size, vec.cast())) }
}

#[derive(Debug)]
struct Region {
    addr: *const u8,
    size: usize,
    info: vm_region_basic_info_data_t,
}

fn mach_vm_region(addr: *const u8) -> Option<Region> {
    let mut addr = addr as u64;

    let mut count = size_of::<vm_region_basic_info_data_64_t>() as mach_msg_type_number_t;
    let mut object_name: mach_port_t = 0;

    let mut size = unsafe { std::mem::zeroed::<mach_vm_size_t>() };
    let mut info = unsafe { std::mem::zeroed::<vm_region_basic_info_data_t>() };
    let result = unsafe {
        mach2::vm::mach_vm_region(
            mach2::traps::mach_task_self(),
            &mut addr,
            &mut size,
            mach2::vm_region::VM_REGION_BASIC_INFO,
            &mut info as *mut vm_region_basic_info_data_t as vm_region_info_t,
            &mut count,
            &mut object_name,
        )
    };
    if result != libc::KERN_SUCCESS {
        return None;
    }
    Some(Region {
        size: size as _,
        info: info,
        addr: addr as _,
    })
}
