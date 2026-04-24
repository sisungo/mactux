use std::sync::atomic::{AtomicU32, Ordering};
use structures::{error::LxError, sync::FUTEX_WAITERS};

pub unsafe fn lock(word: &mut AtomicU32) {
    let tid = crate::thread::id() as u32;
    loop {
        if word
            .compare_exchange(0, tid, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            break;
        }
        word.fetch_or(FUTEX_WAITERS, Ordering::SeqCst);
    }
}

pub unsafe fn try_lock(word: &mut AtomicU32) -> Result<(), LxError> {
    let tid = crate::thread::id() as u32;
    if word
        .compare_exchange(0, tid, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        Ok(())
    } else {
        Err(LxError::EAGAIN)
    }
}

pub unsafe fn unlock(word: &mut AtomicU32) -> Result<(), LxError> {
    let tid = crate::thread::id() as u32;
    let result = word.try_update(Ordering::SeqCst, Ordering::SeqCst, |x| {
        if (x & !FUTEX_WAITERS) == tid {
            Some(0)
        } else {
            None
        }
    });
    if result.is_ok() {
        Ok(())
    } else {
        Err(LxError::EPERM)
    }
}
