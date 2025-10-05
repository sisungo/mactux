use crate::vfd::{VirtualFd, VirtualFile};
use async_trait::async_trait;
use std::sync::Arc;
use structures::{
    error::LxError,
    io::{EventFdFlags, PollEvents},
};
use tokio::sync::watch;

/// An eventfd implementation.
#[derive(Debug, Clone)]
struct EventFd {
    tx: watch::Sender<u64>,
    rx: watch::Receiver<u64>,
}
impl EventFd {
    /// The maximum value the counter can hold.
    const MAX: u64 = u64::MAX - 1;
}
#[async_trait]
impl VirtualFile for EventFd {
    async fn read(&self, buf: &mut [u8], _: &mut u64) -> Result<usize, LxError> {
        if buf.len() != size_of::<u64>() {
            return Err(LxError::EINVAL);
        }
        buf.copy_from_slice(&self.rx.borrow().to_ne_bytes());
        Ok(size_of::<u64>())
    }

    async fn write(&self, buf: &[u8], _: &mut u64) -> Result<usize, LxError> {
        if buf.len() != size_of::<u64>() {
            return Err(LxError::EINVAL);
        }
        let mut number = [0; size_of::<u64>()];
        number.copy_from_slice(buf);
        let number = u64::from_ne_bytes(number);
        self.tx.send_modify(|x| *x += number);
        Ok(8)
    }

    async fn poll(&self, interest: PollEvents) -> Result<PollEvents, LxError> {
        let mut events = PollEvents::empty();
        let mut rx = self.rx.clone();
        let val = rx
            .wait_for(|val| {
                (interest.contains(PollEvents::POLLIN) && *val > 0)
                    || (interest.contains(PollEvents::POLLOUT) && *val < Self::MAX)
            })
            .await
            .map_err(|_| LxError::EIO)?;
        if *val > 0 {
            events |= PollEvents::POLLIN;
        }
        if *val < Self::MAX {
            events |= PollEvents::POLLOUT;
        }
        Ok(events)
    }
}

/// Creates a new eventfd with the given initial value and flags.
pub fn create(initval: u64, flags: EventFdFlags) -> Result<Arc<VirtualFd>, LxError> {
    if flags.contains(EventFdFlags::EFD_SEMAPHORE) {
        return Err(LxError::EINVAL);
    }
    let (tx, rx) = watch::channel(initval);
    Ok(VirtualFd::new(
        Box::new(EventFd { tx, rx }),
        flags.open_flags(),
    ))
}
