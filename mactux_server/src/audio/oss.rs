//! Emulates the OSS interface.

use crate::{
    audio::AudioOutput,
    filesystem::{kernfs::KernFsFile, vfs::NewlyOpen},
    util::FileAttrs,
    vfd::{VirtualFd, VirtualFile},
};
use async_trait::async_trait;
use libc::c_int;
use mactux_ipc::response::{Response, VirtualFdAvailCtrl};
use rodio::cpal::SampleFormat;
use std::sync::atomic;
use structures::{
    error::LxError,
    fs::{OpenFlags, Stat},
    io::IoctlCmd,
};

#[derive(Debug)]
pub struct OssDevice {
    attrs: FileAttrs,
}
impl OssDevice {
    pub fn new(attrs: FileAttrs) -> Self {
        Self { attrs }
    }
}
#[async_trait]
impl KernFsFile for OssDevice {
    async fn open(&self, flags: OpenFlags) -> Result<NewlyOpen, LxError> {
        Ok(NewlyOpen::Virtual(VirtualFd::new(
            Box::new(OssFd::new(flags, self.attrs.clone()).await?),
            flags,
        )))
    }
}

pub struct OssFd {
    output: Option<AudioOutput>,
    attrs: FileAttrs,
}
impl OssFd {
    async fn new(flags: OpenFlags, attrs: FileAttrs) -> Result<Self, LxError> {
        let output = if flags.is_writable() {
            let output = AudioOutput::new().await?;
            output.sample_rate.store(8000, atomic::Ordering::Relaxed);
            output.channels.store(1, atomic::Ordering::Relaxed);
            output.sample_format.store(SampleFormat::U8);
            output.start()?;
            Some(output)
        } else {
            None
        };

        Ok(Self { output, attrs })
    }

    fn output(&self) -> Result<&AudioOutput, LxError> {
        self.output.as_ref().ok_or(LxError::EBADF)
    }
}
#[async_trait]
impl VirtualFile for OssFd {
    async fn stat(&self) -> Result<Stat, LxError> {
        Ok(Stat {
            st_dev: 0,
            st_ino: 0,
            st_nlink: 0,
            st_mode: self.attrs.mode | 0o20000,
            st_uid: self.attrs.uid,
            st_gid: self.attrs.gid,
            _pad0: 0,
            st_rdev: 0,
            st_size: 0,
            st_blksize: 0,
            st_blocks: 0,
            st_atime: self.attrs.atime.tv_sec,
            st_atimensec: self.attrs.atime.tv_nsec as _,
            st_mtime: self.attrs.mtime.tv_sec,
            st_mtimensec: self.attrs.mtime.tv_nsec as _,
            st_ctime: self.attrs.ctime.tv_sec,
            st_ctimensec: self.attrs.ctime.tv_nsec as _,
            _unused: [0; _],
        })
    }

    fn ioctl_query(&self, cmd: u32) -> Result<VirtualFdAvailCtrl, LxError> {
        const AVAIL_CTRL: VirtualFdAvailCtrl = VirtualFdAvailCtrl {
            in_size: size_of::<c_int>() as _,
            out_size: size_of::<c_int>(),
        };
        match IoctlCmd(cmd) {
            IoctlCmd::SNDCTL_DSP_CHANNELS => Ok(AVAIL_CTRL),
            IoctlCmd::SNDCTL_DSP_SETFMT => Ok(AVAIL_CTRL),
            IoctlCmd::SNDCTL_DSP_SPEED => Ok(AVAIL_CTRL),
            IoctlCmd::SNDCTL_DSP_STEREO => Ok(AVAIL_CTRL),
            _ => Err(LxError::EINVAL),
        }
    }

    async fn ioctl(&self, cmd: u32, data: Vec<u8>) -> Result<Response, LxError> {
        let mut buf = [0u8; size_of::<c_int>()];
        if data.len() != buf.len() {
            return Err(LxError::EINVAL);
        }
        buf.copy_from_slice(&data);
        let value = c_int::from_ne_bytes(buf);

        match IoctlCmd(cmd) {
            IoctlCmd::SNDCTL_DSP_CHANNELS => {
                self.output()?
                    .channels
                    .store(value as _, atomic::Ordering::Relaxed);
                Ok(Response::CtrlBlob(0, data))
            }
            IoctlCmd::SNDCTL_DSP_SETFMT => {
                let sample_format = match value {
                    0x8 => SampleFormat::U8,
                    0x10 => SampleFormat::I16,
                    _ => return Err(LxError::EINVAL),
                };
                self.output()?.sample_format.store(sample_format);
                Ok(Response::CtrlBlob(0, data))
            }
            IoctlCmd::SNDCTL_DSP_SPEED => {
                self.output()?
                    .sample_rate
                    .store(value as _, atomic::Ordering::Relaxed);
                Ok(Response::CtrlBlob(0, data))
            }
            IoctlCmd::SNDCTL_DSP_STEREO => {
                if value == 1 {
                    self.output()?.channels.store(2, atomic::Ordering::Relaxed);
                } else {
                    self.output()?.channels.store(1, atomic::Ordering::Relaxed);
                }
                Ok(Response::CtrlBlob(0, data))
            }
            _ => Err(LxError::EINVAL),
        }
    }

    async fn write(&self, buf: &[u8], _: &mut u64) -> Result<usize, LxError> {
        let output = self.output()?;
        let nbytes = output.write_samples(buf)?;
        let nsamples = nbytes / output.sample_format.load().sample_size();
        let time_to_sleep = std::time::Duration::from_secs(1)
            / output.sample_rate.load(atomic::Ordering::Relaxed)
            * (nsamples as u32)
            / output.channels.load(atomic::Ordering::Relaxed) as _;

        if output.sink.len() > 2 {
            tokio::time::sleep(time_to_sleep).await;
        }

        Ok(nbytes)
    }
}
impl Drop for OssFd {
    fn drop(&mut self) {
        if let Some(x) = &self.output {
            _ = x.stop();
        }
    }
}
