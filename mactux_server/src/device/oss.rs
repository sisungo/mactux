//! OSS audio devices.

use crate::{
    device::{Device, DeviceTable},
    file::{Ioctl, Stream},
    multimedia::audio::AudioOutput,
};
use mactux_ipc::response::{CtrlOutput, Response, VfdAvailCtrl};
use rodio::cpal::SampleFormat;
use std::{
    ffi::c_int,
    sync::{
        Arc, Mutex,
        atomic::{self, AtomicBool},
    },
};
use structures::{error::LxError, fs::OpenFlags, io::IoctlCmd};

/// The `/dev/dsp` device.
#[derive(Debug)]
struct Dsp {
    locked: AtomicBool,
    output: Mutex<Option<Arc<AudioOutput>>>,
}
impl Dsp {
    fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            output: Mutex::new(None),
        }
    }

    fn output(&self) -> Result<Arc<AudioOutput>, LxError> {
        self.output.lock().unwrap().clone().ok_or(LxError::EBADF)
    }
}
impl Stream for Dsp {
    fn write(&self, buf: &[u8], _off: &mut i64) -> Result<usize, LxError> {
        let output = self.output()?;
        let nbytes = output.write_samples(buf)?;
        let nsamples = nbytes / output.sample_format.load().sample_size();
        let time_to_sleep = std::time::Duration::from_secs(1)
            / output.sample_rate.load(atomic::Ordering::Relaxed)
            * (nsamples as u32)
            / output.channels.load(atomic::Ordering::Relaxed) as _;

        if output.sink.len() > 2 {
            std::thread::sleep(time_to_sleep);
        }

        Ok(nbytes)
    }
}
impl Ioctl for Dsp {
    fn ioctl_query(&self, cmd: IoctlCmd) -> Result<VfdAvailCtrl, LxError> {
        const AVAIL_CTRL: VfdAvailCtrl = VfdAvailCtrl {
            in_size: size_of::<c_int>() as _,
            out_size: size_of::<c_int>(),
        };
        match cmd {
            IoctlCmd::SNDCTL_DSP_CHANNELS => Ok(AVAIL_CTRL),
            IoctlCmd::SNDCTL_DSP_SETFMT => Ok(AVAIL_CTRL),
            IoctlCmd::SNDCTL_DSP_SPEED => Ok(AVAIL_CTRL),
            IoctlCmd::SNDCTL_DSP_STEREO => Ok(AVAIL_CTRL),
            _ => Err(LxError::EINVAL),
        }
    }

    fn ioctl(&self, cmd: IoctlCmd, data: &[u8]) -> Result<CtrlOutput, LxError> {
        let mut buf = [0u8; size_of::<c_int>()];
        if data.len() != buf.len() {
            return Err(LxError::EINVAL);
        }
        buf.copy_from_slice(&data);
        let value = c_int::from_ne_bytes(buf);

        match cmd {
            IoctlCmd::SNDCTL_DSP_CHANNELS => {
                self.output()?
                    .channels
                    .store(value as _, atomic::Ordering::Relaxed);
                Ok(CtrlOutput {
                    status: 0,
                    blob: data.to_vec(),
                })
            }
            IoctlCmd::SNDCTL_DSP_SETFMT => {
                let sample_format = match value {
                    0x8 => SampleFormat::U8,
                    0x10 => SampleFormat::I16,
                    _ => return Err(LxError::EINVAL),
                };
                self.output()?.sample_format.store(sample_format);
                Ok(CtrlOutput {
                    status: 0,
                    blob: data.to_vec(),
                })
            }
            IoctlCmd::SNDCTL_DSP_SPEED => {
                self.output()?
                    .sample_rate
                    .store(value as _, atomic::Ordering::Relaxed);
                Ok(CtrlOutput {
                    status: 0,
                    blob: data.to_vec(),
                })
            }
            IoctlCmd::SNDCTL_DSP_STEREO => {
                if value == 1 {
                    self.output()?.channels.store(2, atomic::Ordering::Relaxed);
                } else {
                    self.output()?.channels.store(1, atomic::Ordering::Relaxed);
                }
                Ok(CtrlOutput {
                    status: 0,
                    blob: data.to_vec(),
                })
            }
            _ => Err(LxError::EINVAL),
        }
    }
}
impl Device for Dsp {
    fn open(&self, flags: OpenFlags) -> Result<(), LxError> {
        if self
            .locked
            .compare_exchange(
                false,
                true,
                atomic::Ordering::Relaxed,
                atomic::Ordering::Relaxed,
            )
            .is_err()
        {
            return Err(LxError::EBUSY);
        }

        if flags.is_writable() {
            *self.output.lock().unwrap() = Some(Arc::new(AudioOutput::new()?));
        }

        Ok(())
    }

    fn close(&self) {
        *self.output.lock().unwrap() = None;
        self.locked.store(false, atomic::Ordering::Relaxed);
    }
}

pub fn discover(devices: &DeviceTable) {
    devices.add_chr_fixed(14, 3, || Arc::new(Dsp::new()));
}
