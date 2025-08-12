use std::sync::atomic::{self, AtomicU32};
use structures::{
    error::LxError,
    sync::{FutexOpts, FutexWakeOpVal3},
};

pub unsafe fn wait(
    uaddr: *mut u32,
    val: u32,
    utime: *mut libc::timespec,
    opts: FutexOpts,
) -> Result<(), LxError> {
    let flags = if opts.contains(FutexOpts::FUTEX_PRIVATE_FLAGS) {
        0
    } else {
        libc::OS_SYNC_WAIT_ON_ADDRESS_SHARED
    };
    unsafe {
        let result = if utime.is_null() {
            libc::os_sync_wait_on_address(uaddr.cast(), val as _, 4, flags)
        } else {
            let timeout = utime.read();
            let timeout = std::time::Duration::new(timeout.tv_sec as _, timeout.tv_nsec as _);
            libc::os_sync_wait_on_address_with_timeout(
                uaddr.cast(),
                val as _,
                4,
                flags,
                0,
                timeout.as_nanos() as _,
            )
        };

        match result {
            -1 => Err(LxError::last_apple_error()),
            _ => Ok(()),
        }
    }
}

pub unsafe fn wake(uaddr: *mut u32, val: u32, opts: FutexOpts) -> Result<usize, LxError> {
    let flags = if opts.contains(FutexOpts::FUTEX_PRIVATE_FLAGS) {
        0
    } else {
        libc::OS_SYNC_WAKE_BY_ADDRESS_SHARED
    };
    unsafe {
        for n in 0..val {
            match libc::os_sync_wake_by_address_any(uaddr.cast(), 4, flags) {
                -1 => {
                    if *libc::__error() == libc::ENOENT {
                        return Ok(n as usize + 1);
                    }
                    return Err(LxError::last_apple_error());
                }
                _ => continue,
            }
        }
        Ok(0)
    }
}

pub unsafe fn wake_op(
    uaddr: *mut u32,
    val: u32,
    val2: u32,
    uaddr2: *mut u32,
    val3: u32,
    opts: FutexOpts,
) -> Result<usize, LxError> {
    // TODO: This implementation is non-atomic.
    unsafe {
        let mut count = 0;
        let val3 = FutexWakeOpVal3(val3);
        let mut oldval = (*uaddr2.cast::<AtomicU32>()).load(atomic::Ordering::SeqCst);
        val3.op().perform(&mut oldval, val3.oparg())?;
        let newval = oldval;
        uaddr2.write(newval);
        count += wake(uaddr, val, opts)?;
        if val3.cmp().perform(oldval, val3.cmparg())? {
            count += wake(uaddr2, val2, opts)?;
        }

        Ok(count)
    }
}
