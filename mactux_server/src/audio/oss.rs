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
    fs::{OpenFlags, Statx},
    io::IoctlCmd,
};

/// An OSS device.
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

/// An open file descriptor of the OSS device.
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
    async fn stat(&self) -> Result<Statx, LxError> {
        Ok(Statx {
            stx_mask: 0,
            stx_dev_major: 0,
            stx_dev_minor: 0,
            stx_ino: 0,
            stx_nlink: 0,
            stx_uid: self.attrs.uid,
            stx_gid: self.attrs.gid,
            stx_mode: self.attrs.mode as u16 | 0o20000,
            stx_attributes: 0,
            stx_attributes_mask: 0,
            stx_rdev_major: 0,
            stx_rdev_minor: 0,
            stx_size: 0,
            stx_blksize: 0,
            stx_blocks: 0,
            stx_atime: self.attrs.atime.into(),
            stx_btime: self.attrs.btime.into(),
            stx_ctime: self.attrs.ctime.into(),
            stx_mtime: self.attrs.mtime.into(),
            stx_mnt_id: 0,
            stx_dio_mem_align: 0,
            stx_dio_offset_align: 0,
            stx_dio_read_offset_align: 0,
            stx_atomic_write_segments_max: 0,
            stx_atomic_write_unit_min: 0,
            stx_atomic_write_unit_max: 0,
            stx_subvol: 0,
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
