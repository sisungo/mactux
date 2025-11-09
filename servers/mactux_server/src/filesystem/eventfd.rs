use crate::{
    poll::PollToken,
    util::Watch,
    vfd::{Stream, Vfd, VfdContent},
};
use std::sync::Arc;
use structures::{
    error::LxError,
    fs::OpenFlags,
    io::{EventFdFlags, PollEvents, Whence},
};

pub fn open(count: u64, flags: EventFdFlags) -> Result<Vfd, LxError> {
    let mut open_flags = OpenFlags::empty();
    if flags.contains(EventFdFlags::EFD_CLOEXEC) {
        open_flags |= OpenFlags::O_CLOEXEC;
    }
    if flags.contains(EventFdFlags::EFD_NONBLOCK) {
        open_flags |= OpenFlags::O_NONBLOCK;
    }
    Ok(Vfd::new(
        Arc::new(EventFd {
            inner: Watch::new(count),
            flags,
        }),
        open_flags,
    ))
}

#[derive(Debug)]
struct EventFd {
    inner: Watch<u64>,
    flags: EventFdFlags,
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
        if buf.len() != size_of::<u64>() {
            return Err(LxError::EINVAL);
        }
        let mut val = [0; size_of::<u64>()];
        val.copy_from_slice(buf);
        let val = u64::from_ne_bytes(val);
        self.inner.update(|x| *x += val);
        Ok(size_of::<u64>())
    }

    fn poll(&self, _interest: PollEvents) -> Result<PollToken, LxError> {
        todo!()
    }

    fn seek(&self, _: Whence, _: i64) -> Result<i64, LxError> {
        Ok(0)
    }
}
impl VfdContent for EventFd {}
