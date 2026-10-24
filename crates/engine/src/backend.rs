//! Device access, abstracted so the engine does not depend on WASAPI.

use std::fmt;

#[cfg(feature = "cpal")]
pub mod cpal;

/// A device as listed by a backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    /// Stable identifier to open the device with.
    pub id: String,
    /// Human-readable name.
    pub name: String,
}

/// Sample rate and channel count a stream runs with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

/// Called with interleaved `f32` samples captured from an input device.
pub type InputCallback = Box<dyn FnMut(&[f32]) + Send + 'static>;
/// Called to fill interleaved `f32` samples for an output device.
pub type OutputCallback = Box<dyn FnMut(&mut [f32]) + Send + 'static>;
/// Called when a stream reports an error (e.g. the device disappeared).
/// May run on the audio thread: keep it allocation-free.
pub type ErrorCallback = Box<dyn FnMut() + Send + 'static>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendError(pub String);

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for BackendError {}

/// Platform audio API. Opened streams run until they are dropped.
pub trait AudioBackend {
    type Stream;

    fn input_devices(&self) -> Result<Vec<DeviceInfo>, BackendError>;
    fn output_devices(&self) -> Result<Vec<DeviceInfo>, BackendError>;
    fn default_input(&self) -> Option<DeviceInfo>;
    fn default_output(&self) -> Option<DeviceInfo>;

    /// Format the device runs with natively (shared mode mix format).
    fn input_format(&self, id: &str) -> Result<StreamFormat, BackendError>;
    fn output_format(&self, id: &str) -> Result<StreamFormat, BackendError>;

    fn open_input(
        &self,
        id: &str,
        format: StreamFormat,
        data: InputCallback,
        error: ErrorCallback,
    ) -> Result<Self::Stream, BackendError>;

    fn open_output(
        &self,
        id: &str,
        format: StreamFormat,
        data: OutputCallback,
        error: ErrorCallback,
    ) -> Result<Self::Stream, BackendError>;
}
