use crate::{
    util::Watch,
    vfd::{PollToken, Stream, Vfd, VfdContent},
};
use crossbeam::channel::Sender;
use rustc_hash::FxHashSet;
use std::sync::{Arc, Mutex};
use structures::{
    error::LxError,
    io::{EventFdFlags, PollEvents, Whence},
};

pub fn open(count: u64, flags: EventFdFlags) -> Result<Vfd, LxError> {
    Ok(Vfd::new(
        Arc::new(EventFd {
            inner: Watch::new(count),
            flags,
            senders: Mutex::new(Vec::new()),
        }),
        flags.open_flags(),
    ))
}

#[derive(Debug)]
struct EventFd {
    inner: Watch<u64>,
    flags: EventFdFlags,
    senders: Mutex<Vec<Sender<PollEvents>>>,
}
impl Stream for EventFd {
    fn read(&self, buf: &mut [u8], _: &mut i64) -> Result<usize, LxError> {
        if buf.len() != size_of::<u64>() {
            return Err(LxError::EINVAL);
        }
        let mut val = 0;
        self.inner.wait_until(|cur| match cur {
            0 => false,
            other => {
                val = *other;
                if self.flags.contains(EventFdFlags::EFD_SEMAPHORE) {
                    *other -= 1;
                } else {
                    *other = 0;
                }
                true
            }
        });
        buf.copy_from_slice(&val.to_ne_bytes());

        Ok(size_of::<u64>())
    }

    fn write(&self, buf: &[u8], _: &mut i64) -> Result<usize, LxError> {
        // Ensure that the argument is valid
        if buf.len() != size_of::<u64>() {
            return Err(LxError::EINVAL);
        }

        // Get added value
        let mut val = [0; size_of::<u64>()];
        val.copy_from_slice(buf);
        let val = u64::from_ne_bytes(val);

        // Update the internal value
        self.inner.update(|x| *x += val);

        // Notify polling clients
        let mut senders = self.senders.lock().unwrap();
        let mut invalid = FxHashSet::default();
        for (id, sender) in senders.iter().enumerate() {
            if sender.send(PollEvents::POLLIN).is_err() {
                invalid.insert(id);
            }
        }

        // Remove invalidated polling clients
        let mut id = 0;
        senders.retain(|_| {
            id += 1;
            !invalid.contains(&(id - 1))
        });

        Ok(size_of::<u64>())
    }

    fn poll(&self, interest: PollEvents) -> Result<PollToken, LxError> {
        let (tx, rx) = crossbeam::channel::unbounded();
        self.senders.lock().unwrap().push(tx);
        Ok(PollToken {
            vfd: 0,
            interest,
            receiver: rx,
        })
    }

    fn seek(&self, _: i64, _: Whence, _: i64) -> Result<i64, LxError> {
        Ok(0)
    }
}
impl VfdContent for EventFd {}
