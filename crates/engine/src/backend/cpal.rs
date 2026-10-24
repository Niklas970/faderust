//! [`AudioBackend`] on top of cpal (WASAPI shared mode on Windows).

use ::cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ::cpal::{
    Device, ErrorKind, FromSample, Host, Sample, SampleFormat, SizedSample, Stream, StreamConfig,
};

use super::{
    AudioBackend, BackendError, DeviceInfo, ErrorCallback, InputCallback, OutputCallback,
    StreamFormat,
};

/// Frames converted per step between device samples and `f32`.
const CONVERT_FRAMES: usize = 1024;

pub struct CpalBackend {
    host: Host,
}

impl Default for CpalBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CpalBackend {
    pub fn new() -> Self {
        Self {
            host: ::cpal::default_host(),
        }
    }

    fn device(&self, id: &str) -> Result<Device, BackendError> {
        let parsed = id
            .parse()
            .map_err(|e| BackendError(format!("invalid device id {id}: {e}")))?;
        self.host
            .device_by_id(&parsed)
            .ok_or_else(|| BackendError(format!("device {id} not found")))
    }
}

fn err(e: impl std::fmt::Display) -> BackendError {
    BackendError(e.to_string())
}

fn info(device: &Device) -> Option<DeviceInfo> {
    let id = device.id().ok()?.to_string();
    let name = device
        .description()
        .map(|d| d.name().to_string())
        .unwrap_or_else(|_| id.clone());
    Some(DeviceInfo { id, name })
}

fn config(format: StreamFormat) -> StreamConfig {
    StreamConfig {
        channels: format.channels,
        sample_rate: format.sample_rate,
        buffer_size: ::cpal::BufferSize::Default,
    }
}

impl AudioBackend for CpalBackend {
    type Stream = Stream;

    fn input_devices(&self) -> Result<Vec<DeviceInfo>, BackendError> {
        Ok(self
            .host
            .input_devices()
            .map_err(err)?
            .filter_map(|d| info(&d))
            .collect())
    }

    fn output_devices(&self) -> Result<Vec<DeviceInfo>, BackendError> {
        Ok(self
            .host
            .output_devices()
            .map_err(err)?
            .filter_map(|d| info(&d))
            .collect())
    }

    fn default_input(&self) -> Option<DeviceInfo> {
        info(&self.host.default_input_device()?)
    }

    fn default_output(&self) -> Option<DeviceInfo> {
        info(&self.host.default_output_device()?)
    }

    fn input_format(&self, id: &str) -> Result<StreamFormat, BackendError> {
        let c = self.device(id)?.default_input_config().map_err(err)?;
        Ok(StreamFormat {
            sample_rate: c.sample_rate(),
            channels: c.channels(),
        })
    }

    fn output_format(&self, id: &str) -> Result<StreamFormat, BackendError> {
        let c = self.device(id)?.default_output_config().map_err(err)?;
        Ok(StreamFormat {
            sample_rate: c.sample_rate(),
            channels: c.channels(),
        })
    }

    fn open_input(
        &self,
        id: &str,
        format: StreamFormat,
        data: InputCallback,
        error: ErrorCallback,
    ) -> Result<Stream, BackendError> {
        let device = self.device(id)?;
        let sample_format = device.default_input_config().map_err(err)?.sample_format();
        let stream = match sample_format {
            SampleFormat::F32 => build_input::<f32>(&device, format, data, error),
            SampleFormat::I16 => build_input::<i16>(&device, format, data, error),
            SampleFormat::I32 => build_input::<i32>(&device, format, data, error),
            SampleFormat::U16 => build_input::<u16>(&device, format, data, error),
            other => return Err(BackendError(format!("unsupported sample format {other:?}"))),
        }?;
        stream.play().map_err(err)?;
        Ok(stream)
    }

    fn open_output(
        &self,
        id: &str,
        format: StreamFormat,
        data: OutputCallback,
        error: ErrorCallback,
    ) -> Result<Stream, BackendError> {
        let device = self.device(id)?;
        let sample_format = device.default_output_config().map_err(err)?.sample_format();
        let stream = match sample_format {
            SampleFormat::F32 => build_output::<f32>(&device, format, data, error),
            SampleFormat::I16 => build_output::<i16>(&device, format, data, error),
            SampleFormat::I32 => build_output::<i32>(&device, format, data, error),
            SampleFormat::U16 => build_output::<u16>(&device, format, data, error),
            other => return Err(BackendError(format!("unsupported sample format {other:?}"))),
        }?;
        stream.play().map_err(err)?;
        Ok(stream)
    }
}

/// Whether a stream error means the stream is gone. Glitches (WASAPI reports
/// one for every capture discontinuity), a refused real-time boost and an
/// automatic reroute leave the stream running; reconnecting for them only
/// causes the next glitch.
fn is_fatal(kind: ErrorKind) -> bool {
    !matches!(
        kind,
        ErrorKind::Xrun | ErrorKind::RealtimeDenied | ErrorKind::DeviceChanged
    )
}

fn build_input<T>(
    device: &Device,
    format: StreamFormat,
    mut data: InputCallback,
    mut error: ErrorCallback,
) -> Result<Stream, BackendError>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = format.channels as usize;
    // Preallocated so the callback never allocates. Copying even f32 data is
    // negligible next to the mixing and keeps one code path.
    let mut scratch = vec![0.0f32; CONVERT_FRAMES * channels];
    device
        .build_input_stream::<T, _, _>(
            config(format),
            move |samples: &[T], _| {
                for chunk in samples.chunks(CONVERT_FRAMES * channels) {
                    let buf = &mut scratch[..chunk.len()];
                    for (d, s) in buf.iter_mut().zip(chunk) {
                        *d = f32::from_sample(*s);
                    }
                    data(buf);
                }
            },
            move |e: ::cpal::Error| {
                if is_fatal(e.kind()) {
                    error();
                }
            },
            None,
        )
        .map_err(err)
}

fn build_output<T>(
    device: &Device,
    format: StreamFormat,
    mut data: OutputCallback,
    mut error: ErrorCallback,
) -> Result<Stream, BackendError>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = format.channels as usize;
    let mut scratch = vec![0.0f32; CONVERT_FRAMES * channels];
    device
        .build_output_stream::<T, _, _>(
            config(format),
            move |samples: &mut [T], _| {
                for chunk in samples.chunks_mut(CONVERT_FRAMES * channels) {
                    let buf = &mut scratch[..chunk.len()];
                    data(buf);
                    for (d, s) in chunk.iter_mut().zip(buf.iter()) {
                        *d = T::from_sample(*s);
                    }
                }
            },
            move |e: ::cpal::Error| {
                if is_fatal(e.kind()) {
                    error();
                }
            },
            None,
        )
        .map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glitches_are_not_fatal() {
        assert!(!is_fatal(ErrorKind::Xrun));
        assert!(!is_fatal(ErrorKind::RealtimeDenied));
        assert!(!is_fatal(ErrorKind::DeviceChanged));
        assert!(is_fatal(ErrorKind::DeviceNotAvailable));
        assert!(is_fatal(ErrorKind::StreamInvalidated));
        assert!(is_fatal(ErrorKind::BackendError));
    }
}
