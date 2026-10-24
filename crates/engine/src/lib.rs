//! Faderust audio engine.
//!
//! Every input device feeds one lock-free ring buffer per output device
//! ([`lane`]). Each output callback pulls from its lanes through an adaptive
//! resampler that keeps the buffer fill at a target ([`drift`]), applies the
//! [`matrix`] gains and sums. Devices can come and go while the others keep
//! playing ([`engine`]). Device access sits behind [`backend::AudioBackend`]
//! so a PipeWire backend can be added later.

pub mod backend;
pub mod drift;
pub mod engine;
pub mod lane;
pub mod matrix;
pub mod meter;
pub mod node;

pub use engine::{
    Direction, Engine, EngineConfig, EngineError, EngineEvent, InputConfig, LostReason,
    OutputConfig, SlotState,
};
pub use matrix::MixMatrix;
pub use meter::PeakMeter;

/// Crate version, exposed for the app's about screen and logs.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The internal bus is stereo: mono inputs are duplicated, extra channels dropped.
pub const BUS_CHANNELS: usize = 2;
