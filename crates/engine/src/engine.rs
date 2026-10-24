//! Builds the lanes for a fixed set of devices and runs them.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::Arc;

use crate::backend::{AudioBackend, BackendError, DeviceInfo, StreamFormat};
use crate::lane::{lane, LaneSpec, LaneStats};
use crate::matrix::MixMatrix;
use crate::meter::PeakMeter;
use crate::node::{InputNode, OutputNode};

#[derive(Debug, Clone)]
pub struct InputConfig {
    pub device_id: String,
}

#[derive(Debug, Clone)]
pub struct OutputConfig {
    pub device_id: String,
    /// Buffer each input keeps for this output. Bluetooth needs more than USB.
    pub buffer_ms: u32,
}

#[derive(Debug, Clone, Default)]
pub struct EngineConfig {
    pub inputs: Vec<InputConfig>,
    pub outputs: Vec<OutputConfig>,
}

#[derive(Debug)]
pub enum EngineError {
    Backend(BackendError),
    NoDevices,
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::Backend(e) => write!(f, "audio backend: {e}"),
            EngineError::NoDevices => f.write_str("at least one input and one output are required"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<BackendError> for EngineError {
    fn from(e: BackendError) -> Self {
        EngineError::Backend(e)
    }
}

/// Live state of one device.
#[derive(Debug)]
pub struct DeviceState {
    pub id: String,
    pub format: StreamFormat,
    pub meter: Arc<PeakMeter>,
    pub stream_errors: Arc<AtomicU64>,
}

/// A running engine. Dropping it stops all streams.
///
/// The topology is fixed for now; hot-plug (E2) will add and remove devices
/// without stopping the others.
pub struct Engine<B: AudioBackend> {
    pub matrix: Arc<MixMatrix>,
    pub inputs: Vec<DeviceState>,
    pub outputs: Vec<DeviceState>,
    /// `lanes[input][output]`.
    pub lanes: Vec<Vec<Arc<LaneStats>>>,
    _streams: Vec<B::Stream>,
}

impl<B: AudioBackend> Engine<B> {
    pub fn start(backend: &B, config: &EngineConfig) -> Result<Self, EngineError> {
        if config.inputs.is_empty() || config.outputs.is_empty() {
            return Err(EngineError::NoDevices);
        }
        let n_in = config.inputs.len();
        let n_out = config.outputs.len();
        let matrix = Arc::new(MixMatrix::new(n_in, n_out));

        let state = |id: &str, format| DeviceState {
            id: id.to_string(),
            format,
            meter: Arc::new(PeakMeter::new()),
            stream_errors: Arc::new(AtomicU64::new(0)),
        };
        let inputs = config
            .inputs
            .iter()
            .map(|c| Ok(state(&c.device_id, backend.input_format(&c.device_id)?)))
            .collect::<Result<Vec<_>, BackendError>>()?;
        let outputs = config
            .outputs
            .iter()
            .map(|c| Ok(state(&c.device_id, backend.output_format(&c.device_id)?)))
            .collect::<Result<Vec<_>, BackendError>>()?;

        // Build every lane, then hand the ends to their device callbacks.
        let mut producers: Vec<Vec<_>> = (0..n_in).map(|_| Vec::new()).collect();
        let mut consumers: Vec<Vec<_>> = (0..n_out).map(|_| Vec::new()).collect();
        let mut lanes: Vec<Vec<_>> = (0..n_in).map(|_| Vec::new()).collect();
        for (i, input) in inputs.iter().enumerate() {
            for (o, (output, cfg)) in outputs.iter().zip(&config.outputs).enumerate() {
                let (producer, lane) = lane(LaneSpec {
                    input_rate: input.format.sample_rate,
                    output_rate: output.format.sample_rate,
                    target_ms: cfg.buffer_ms,
                });
                let stats = Arc::clone(lane.stats());
                producers[i].push((producer, Arc::clone(&stats)));
                consumers[o].push((i, lane));
                lanes[i].push(stats);
            }
        }

        let mut streams = Vec::with_capacity(n_in + n_out);
        // Outputs first, so inputs never fill buffers nobody drains yet.
        for ((o, output), lanes_o) in outputs.iter().enumerate().zip(consumers) {
            let mut node =
                OutputNode::new(o, lanes_o, Arc::clone(&matrix), Arc::clone(&output.meter));
            let channels = output.format.channels as usize;
            let errors = Arc::clone(&output.stream_errors);
            streams.push(backend.open_output(
                &output.id,
                output.format,
                Box::new(move |data| node.render(data, channels)),
                Box::new(move || {
                    errors.fetch_add(1, Relaxed);
                }),
            )?);
        }
        for (input, producers_i) in inputs.iter().zip(producers) {
            let mut node = InputNode::new(producers_i, Arc::clone(&input.meter));
            let channels = input.format.channels as usize;
            let errors = Arc::clone(&input.stream_errors);
            streams.push(backend.open_input(
                &input.id,
                input.format,
                Box::new(move |data| node.process(data, channels)),
                Box::new(move || {
                    errors.fetch_add(1, Relaxed);
                }),
            )?);
        }

        Ok(Self {
            matrix,
            inputs,
            outputs,
            lanes,
            _streams: streams,
        })
    }
}

/// Finds a device whose name contains `pattern` (case-insensitive), or
/// whose id equals it.
pub fn find_device(devices: &[DeviceInfo], pattern: &str) -> Option<DeviceInfo> {
    let needle = pattern.to_lowercase();
    devices
        .iter()
        .find(|d| d.id == pattern || d.name.to_lowercase().contains(&needle))
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{ErrorCallback, InputCallback, OutputCallback};
    use std::cell::RefCell;

    /// Backend that records callbacks so a test can drive them by hand.
    #[derive(Default)]
    struct FakeBackend {
        inputs: RefCell<Vec<InputCallback>>,
        outputs: RefCell<Vec<OutputCallback>>,
    }

    const FORMAT: StreamFormat = StreamFormat {
        sample_rate: 48_000,
        channels: 2,
    };

    impl AudioBackend for FakeBackend {
        type Stream = ();
        fn input_devices(&self) -> Result<Vec<DeviceInfo>, BackendError> {
            Ok(vec![])
        }
        fn output_devices(&self) -> Result<Vec<DeviceInfo>, BackendError> {
            Ok(vec![])
        }
        fn default_input(&self) -> Option<DeviceInfo> {
            None
        }
        fn default_output(&self) -> Option<DeviceInfo> {
            None
        }
        fn input_format(&self, _: &str) -> Result<StreamFormat, BackendError> {
            Ok(StreamFormat {
                channels: 1,
                ..FORMAT
            })
        }
        fn output_format(&self, _: &str) -> Result<StreamFormat, BackendError> {
            Ok(FORMAT)
        }
        fn open_input(
            &self,
            _: &str,
            _: StreamFormat,
            data: InputCallback,
            _: ErrorCallback,
        ) -> Result<(), BackendError> {
            self.inputs.borrow_mut().push(data);
            Ok(())
        }
        fn open_output(
            &self,
            _: &str,
            _: StreamFormat,
            data: OutputCallback,
            _: ErrorCallback,
        ) -> Result<(), BackendError> {
            self.outputs.borrow_mut().push(data);
            Ok(())
        }
    }

    #[test]
    fn routes_two_inputs_to_two_outputs() {
        let backend = FakeBackend::default();
        let config = EngineConfig {
            inputs: vec![
                InputConfig {
                    device_id: "mic".into(),
                },
                InputConfig {
                    device_id: "game".into(),
                },
            ],
            outputs: vec![
                OutputConfig {
                    device_id: "usb".into(),
                    buffer_ms: 20,
                },
                OutputConfig {
                    device_id: "bt".into(),
                    buffer_ms: 60,
                },
            ],
        };
        let engine = Engine::start(&backend, &config).unwrap();
        // Output 0 hears only the mic, output 1 hears both at -6 dB.
        engine.matrix.set_muted(1, 0, true);
        engine.matrix.set_gain_db(0, 1, -6.0);
        engine.matrix.set_gain_db(1, 1, -6.0);

        let mic = vec![0.2f32; 480];
        let game = vec![0.4f32; 480];
        let mut out = [vec![0.0f32; 960], vec![0.0f32; 960]];
        for _ in 0..300 {
            let mut ins = backend.inputs.borrow_mut();
            ins[0](&mic);
            ins[1](&game);
            for (cb, buf) in backend.outputs.borrow_mut().iter_mut().zip(&mut out) {
                cb(buf);
            }
        }
        assert!(
            out[0].iter().all(|v| (v - 0.2).abs() < 1e-3),
            "{:?}",
            &out[0][..4]
        );
        let both = (0.2 + 0.4) * 0.501;
        assert!(
            out[1].iter().all(|v| (v - both).abs() < 2e-3),
            "{:?}",
            &out[1][..4]
        );
        assert!(engine.inputs[1].meter.take()[0] > 0.39);
        for lanes in &engine.lanes {
            for l in lanes {
                assert_eq!(l.underruns.load(Relaxed), 0);
            }
        }
    }

    #[test]
    fn rejects_empty_config() {
        let backend = FakeBackend::default();
        assert!(matches!(
            Engine::start(&backend, &EngineConfig::default()),
            Err(EngineError::NoDevices)
        ));
    }

    #[test]
    fn finds_devices_by_name_or_id() {
        let devices = vec![
            DeviceInfo {
                id: "{1}".into(),
                name: "USB Microphone".into(),
            },
            DeviceInfo {
                id: "{2}".into(),
                name: "Galaxy Buds".into(),
            },
        ];
        assert_eq!(find_device(&devices, "buds").unwrap().id, "{2}");
        assert_eq!(find_device(&devices, "{1}").unwrap().name, "USB Microphone");
        assert!(find_device(&devices, "ps5").is_none());
    }
}
