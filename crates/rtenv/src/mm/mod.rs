use crate::posix_bi;
use structures::{
    ToApple,
    error::LxError,
    mm::{Madvice, MmapFlags, MmapProt, MremapFlags},
};

pub unsafe fn remap(
    old_addr: *mut u8,
    old_size: usize,
    new_addr: *mut u8,
    new_size: usize,
    flags: MremapFlags,
) -> Result<*mut u8, LxError> {
    unsafe {
        // TODO
        let new_addr = match libc::mmap(
            new_addr.cast(),
            new_size,
            (MmapProt::PROT_READ | MmapProt::PROT_WRITE).to_apple(),
            (MmapFlags::MAP_PRIVATE | MmapFlags::MAP_ANON).to_apple(),
            -1,
            0,
        ) {
            libc::MAP_FAILED => Err(std::io::Error::last_os_error()),
            addr => Ok(addr),
        }?;
        new_addr.copy_from(old_addr.cast(), old_size.min(new_size));
        Ok(new_addr.cast())
    }
}

pub unsafe fn advise(start: *mut u8, len: usize, advice: Madvice) -> Result<(), LxError> {
    if let Ok(apple_advice) = advice.to_apple() {
        unsafe {
            return posix_bi!(libc::madvise(start.cast(), len, apple_advice));
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
