//! OSS audio devices.

use crate::{
    device::{Device, DeviceTable},
    multimedia::audio::AudioOutput,
    vfd::Stream,
};
use rodio::cpal::SampleFormat;
use std::{ffi::c_int, sync::Arc};
use structures::{
    error::LxError,
    fs::OpenFlags,
    internal::mactux_ipc::CtrlOutput,
    io::{IoctlCmd, VfdAvailCtrl},
};

/// The `/dev/dsp` device.
#[derive(Debug)]
struct Dsp;
impl Device for Dsp {
    fn open(&self, flags: OpenFlags) -> Result<Arc<dyn Stream + Send + Sync>, LxError> {
        Ok(DspFd::new(flags)?)
    }
}

struct DspFd {
    output: Option<Arc<AudioOutput>>,
}
impl DspFd {
    fn new(flags: OpenFlags) -> Result<Arc<Self>, LxError> {
        let output = if flags.is_writable() {
            let output = Arc::new(AudioOutput::new()?);
            _ = output.set_channels(1);
            _ = output.set_sample_rate(8000);
            output.sample_format.store(SampleFormat::U8);
            Some(output)
        } else {
            None
        };

        Ok(Arc::new(Self { output }))
    }

    fn output(&self) -> Result<Arc<AudioOutput>, LxError> {
        self.output.clone().ok_or(LxError::EBADF)
    }
}
impl Stream for DspFd {
    fn write(&self, buf: &[u8], _off: &mut i64) -> Result<usize, LxError> {
        let output = self.output()?;
        let nbytes = output.write_samples(buf)?;
        let nsamples = nbytes / output.sample_format.load().sample_size();
        let time_to_sleep = std::time::Duration::from_secs(1) / output.sample_rate()
            * (nsamples as u32)
            / output.channels() as _;

        if output.player.len() > 2 {
            std::thread::sleep(time_to_sleep);
        }

        Ok(nbytes)
    }

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
                if !self.output()?.set_channels(value as _) {
                    return Err(LxError::EINVAL);
                }
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
                if !self.output()?.set_sample_rate(value as _) {
                    return Err(LxError::EINVAL);
                }
                Ok(CtrlOutput {
                    status: 0,
                    blob: data.to_vec(),
                })
            }
            IoctlCmd::SNDCTL_DSP_STEREO => {
                if value == 1 {
                    _ = self.output()?.set_channels(2);
                } else {
                    _ = self.output()?.set_channels(1);
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

pub fn discover(devices: &DeviceTable) {
    devices.add_chr_fixed(14, 3, || Arc::new(Dsp));
}
