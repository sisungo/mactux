//! Linux audio emulation infrastructure.

use crossbeam::atomic::AtomicCell;
use rodio::{
    OutputStreamBuilder, Sample, Sink,
    buffer::SamplesBuffer,
    conversions::SampleTypeConverter,
    cpal::{FromSample, SampleFormat},
};
use std::{
    fmt::Debug,
    io::{Cursor, Read},
    sync::{
        atomic::{self, AtomicU16, AtomicU32},
        mpsc,
    },
};
use structures::error::LxError;

/// A common audio node for output.
///
/// This is shared across different audio interfaces, and wraps the actual macOS API.
pub struct AudioOutput {
    drop_notify: mpsc::SyncSender<()>,
    pub sink: Sink,
    pub sample_rate: AtomicU32,
    pub channels: AtomicU16,
    pub sample_format: AtomicCell<SampleFormat>,
}
impl AudioOutput {
    /// Creates a new audio output instance.
    pub fn new() -> Result<Self, LxError> {
        let (drop_notify, drop_rx) = mpsc::sync_channel(1);
        let (sink_tx, sink_rx) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name(String::from("AudioOutput"))
            .spawn(move || {
                let output_stream = OutputStreamBuilder::open_default_stream();
                let mut output_stream = match output_stream {
                    Ok(x) => x,
                    Err(err) => {
                        _ = sink_tx.send(Err(from_stream_error(err)));
                        return;
                    }
                };
                output_stream.log_on_drop(false);
                _ = sink_tx.send(Ok(Sink::connect_new(output_stream.mixer())));
                _ = drop_rx.recv();
            })
            .map_err(|_| LxError::EIO)?;
        let sink = sink_rx.recv().map_err(|_| LxError::EIO)??;

        Ok(Self {
            drop_notify,
            sink,
            sample_rate: 48000.into(),
            channels: 2.into(),
            sample_format: SampleFormat::I16.into(),
        })
    }

    /// Writes samples to the audio output, returning the number of bytes written.
    ///
    /// Currently, partial samples are not written. For example, if we set the audio output to accept 16-bit samples,
    /// trying to write 3 samples will always return 2.
    pub fn write_samples(&self, samples: &[u8]) -> Result<usize, LxError> {
        let (samples, bytes) = convert_samples(self.sample_format.load(), samples);
        let buffer = SamplesBuffer::new(
            self.channels.load(atomic::Ordering::Relaxed),
            self.sample_rate.load(atomic::Ordering::Relaxed),
            samples,
        );
        self.sink.append(buffer);
        Ok(bytes)
    }

    /// Starts the audio output.
    pub fn start(&self) -> Result<(), LxError> {
        self.sink.play();
        Ok(())
    }

    /// Stops the audio output.
    pub fn stop(&self) -> Result<(), LxError> {
        self.sink.pause();
        Ok(())
    }
}
impl Debug for AudioOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioOutput")
            .field("sample_rate", &self.sample_rate)
            .field("channels", &self.channels)
            .field("sample_format", &self.sample_format)
            .finish_non_exhaustive()
    }
}
impl Drop for AudioOutput {
    fn drop(&mut self) {
        _ = self.drop_notify.send(());
    }
}

fn from_stream_error(err: rodio::StreamError) -> LxError {
    match err {
        _ => LxError::EIO,
    }
}

/// Converts samples in given format to the F32 format, returning a tuple of samples and number of bytes written.
fn convert_samples(fmt: SampleFormat, samples: &[u8]) -> (Vec<Sample>, usize) {
    match fmt {
        SampleFormat::I8 => _convert_samples::<i8>(samples),
        SampleFormat::U8 => _convert_samples::<u8>(samples),
        SampleFormat::I16 => _convert_samples::<i16>(samples),
        SampleFormat::U16 => _convert_samples::<u16>(samples),
        SampleFormat::I32 => _convert_samples::<i32>(samples),
        SampleFormat::U32 => _convert_samples::<u32>(samples),
        SampleFormat::I64 => _convert_samples::<i64>(samples),
        SampleFormat::U64 => _convert_samples::<u64>(samples),
        SampleFormat::F32 => _convert_samples::<f32>(samples),
        SampleFormat::F64 => _convert_samples::<f64>(samples),
        _ => panic!("Unsupported sample format"),
    }
}

/// Converts samples to the F32 format, returning a tuple of samples and number of bytes written.
fn _convert_samples<I: rodio::cpal::Sample + FromBytes>(rsamples: &[u8]) -> (Vec<Sample>, usize)
where
    Sample: FromSample<I>,
{
    let mut stream = Cursor::new(rsamples);
    let mut sample_buf = vec![0u8; size_of::<I>()];

    let converter = SampleTypeConverter::new(std::iter::from_fn(|| {
        let Ok(n) = stream.read(&mut sample_buf) else {
            return None;
        };
        if n != size_of::<I>() {
            return None;
        }
        return Some(I::from_bytes(&sample_buf));
    }));

    (converter.collect(), stream.position() as _)
}

/// Converts from a byte slice to a certain type.
trait FromBytes {
    /// Converts from a byte slice to a certain type.
    fn from_bytes(bytes: &[u8]) -> Self;
}

macro_rules! int_from_bytes {
    ($t:ty) => {
        impl FromBytes for $t {
            fn from_bytes(bytes: &[u8]) -> Self {
                let mut data = [0u8; size_of::<Self>()];
                data.copy_from_slice(bytes);
                Self::from_le_bytes(data)
            }
        }
    };
    ($($t:ty),*) => {
        $(int_from_bytes!($t);)*
    }
}
macro_rules! float_from_bytes {
    ($t:ty:$b:ty) => {
        impl FromBytes for $t {
            fn from_bytes(bytes: &[u8]) -> Self {
                let mut data = [0u8; size_of::<Self>()];
                data.copy_from_slice(bytes);
                Self::from_bits(<$b>::from_ne_bytes(data))
            }
        }
    };
    ($($t:ty:$b:ty),*) => {
        $(float_from_bytes!($t:$b);)*
    }
}

int_from_bytes!(i8, u8, i16, u16, i32, u32, i64, u64);
float_from_bytes!(f32: u32, f64: u64);
