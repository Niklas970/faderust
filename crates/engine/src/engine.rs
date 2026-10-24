//! Device slots, hot-plug and the lanes between them.
//!
//! The configuration names one slot per wanted device. A slot is connected
//! whenever its device is present; [`Engine::maintain`] checks this and
//! (re)connects or removes single devices while all others keep playing.

use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::backend::{AudioBackend, BackendError, DeviceInfo, StreamFormat};
use crate::fx::InputFx;
use crate::lane::{lane, LaneSpec, LaneStats};
use crate::matrix::MixMatrix;
use crate::meter::PeakMeter;
use crate::node::{InputCommand, InputNode, NodeHandle, OutputCommand, OutputNode};

#[derive(Debug, Clone)]
pub struct InputConfig {
    /// Device id, or part of the device name (case-insensitive).
    pub device: String,
}

#[derive(Debug, Clone)]
pub struct OutputConfig {
    /// Device id, or part of the device name (case-insensitive).
    pub device: String,
    /// Buffer each input keeps for this output. Bluetooth needs more than USB.
    pub buffer_ms: u32,
    /// Close the device after this long without signal for it, and open it
    /// again when signal returns. Lets Bluetooth headphones switch to
    /// another device (multipoint) while Faderust has nothing to play.
    pub release_after: Option<Duration>,
}

/// Below this peak (−60 dBFS) an input counts as silent.
const SILENCE: f32 = 0.001;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Input,
    Output,
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Direction::Input => "in",
            Direction::Output => "out",
        })
    }
}

/// What [`Engine::maintain`] changed.
#[derive(Debug, Clone, PartialEq)]
pub enum EngineEvent {
    Connected {
        dir: Direction,
        slot: usize,
        device: DeviceInfo,
        format: StreamFormat,
    },
    Lost {
        dir: Direction,
        slot: usize,
        reason: LostReason,
    },
    /// An output closed after silence; it opens again with signal.
    Released { slot: usize },
    ConnectFailed {
        dir: Direction,
        slot: usize,
        error: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LostReason {
    /// The stream reported an error (unplugged, format change, ...).
    StreamError,
    /// The device is no longer listed.
    Removed,
}

impl fmt::Display for EngineEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineEvent::Connected {
                dir,
                slot,
                device,
                format,
            } => write!(
                f,
                "{dir}{slot} connected: {} ({} Hz, {} ch)",
                device.name, format.sample_rate, format.channels
            ),
            EngineEvent::Lost { dir, slot, reason } => {
                let why = match reason {
                    LostReason::StreamError => "stream error",
                    LostReason::Removed => "device removed",
                };
                write!(f, "{dir}{slot} lost ({why})")
            }
            EngineEvent::Released { slot } => write!(f, "out{slot} released (silent)"),
            EngineEvent::ConnectFailed { dir, slot, error } => {
                write!(f, "{dir}{slot} could not connect: {error}")
            }
        }
    }
}

/// Public state of one device slot.
#[derive(Debug)]
pub struct SlotState {
    /// Device id or name pattern from the configuration.
    pub pattern: String,
    /// The connected device and its format, if any.
    pub device: Option<(DeviceInfo, StreamFormat)>,
    pub meter: Arc<PeakMeter>,
    /// Stream errors since the engine started.
    pub stream_errors: Arc<AtomicU64>,
}

/// A connected device: its stream and the control end of its node.
struct Live<S, C> {
    stream: S,
    handle: NodeHandle<C>,
    failed: Arc<AtomicBool>,
}

impl<S, C> Live<S, C> {
    /// Stops the stream first, so its callback no longer runs, then frees
    /// whatever the node handed back.
    fn close(self) {
        drop(self.stream);
        let mut handle = self.handle;
        handle.collect_garbage();
    }
}

/// The audio engine. Dropping it stops all streams.
///
/// Not `Send`: keep it on the thread that calls [`Engine::maintain`].
pub struct Engine<B: AudioBackend> {
    backend: B,
    config: EngineConfig,
    pub matrix: Arc<MixMatrix>,
    inputs: Vec<SlotState>,
    outputs: Vec<SlotState>,
    live_in: Vec<Option<Live<B::Stream, InputCommand>>>,
    live_out: Vec<Option<Live<B::Stream, OutputCommand>>>,
    /// `lanes[input][output]`, stable across reconnections.
    lanes: Vec<Vec<Arc<LaneStats>>>,
    /// Effect settings per input, stable across reconnections.
    fx: Vec<Arc<InputFx>>,
    idle: Vec<Idle>,
}

/// Silence tracking of one output, see [`OutputConfig::release_after`].
#[derive(Debug, Default)]
struct Idle {
    quiet_since: Option<Instant>,
    released: bool,
}

fn slot(pattern: &str) -> SlotState {
    SlotState {
        pattern: pattern.to_string(),
        device: None,
        meter: Arc::new(PeakMeter::new()),
        stream_errors: Arc::new(AtomicU64::new(0)),
    }
}

fn error_flag(state: &SlotState) -> (Arc<AtomicBool>, Box<dyn FnMut() + Send>) {
    let failed = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&failed);
    let errors = Arc::clone(&state.stream_errors);
    let callback = Box::new(move || {
        flag.store(true, Relaxed);
        errors.fetch_add(1, Relaxed);
    });
    (failed, callback)
}

impl<B: AudioBackend> Engine<B> {
    /// Creates the engine with every slot disconnected. Call
    /// [`maintain`](Self::maintain) to connect the devices that are present.
    pub fn new(backend: B, config: EngineConfig) -> Result<Self, EngineError> {
        if config.inputs.is_empty() || config.outputs.is_empty() {
            return Err(EngineError::NoDevices);
        }
        let (n_in, n_out) = (config.inputs.len(), config.outputs.len());
        Ok(Self {
            backend,
            matrix: Arc::new(MixMatrix::new(n_in, n_out)),
            inputs: config.inputs.iter().map(|c| slot(&c.device)).collect(),
            outputs: config.outputs.iter().map(|c| slot(&c.device)).collect(),
            live_in: (0..n_in).map(|_| None).collect(),
            live_out: (0..n_out).map(|_| None).collect(),
            lanes: (0..n_in)
                .map(|_| (0..n_out).map(|_| Arc::default()).collect())
                .collect(),
            fx: (0..n_in).map(|_| Arc::default()).collect(),
            idle: (0..n_out).map(|_| Idle::default()).collect(),
            config,
        })
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn inputs(&self) -> &[SlotState] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[SlotState] {
        &self.outputs
    }

    /// Effect settings of `input`; changes apply while it plays.
    pub fn input_fx(&self, input: usize) -> &Arc<InputFx> {
        &self.fx[input]
    }

    /// Whether `output` is closed because nothing played for it.
    pub fn is_released(&self, output: usize) -> bool {
        self.idle[output].released
    }

    /// Statistics of the lane from `input` to `output`.
    pub fn lane_stats(&self, input: usize, output: usize) -> &Arc<LaneStats> {
        &self.lanes[input][output]
    }

    /// Removes lost devices and connects the ones that (re)appeared. Call it
    /// regularly, e.g. twice a second, and after device notifications.
    pub fn maintain(&mut self) -> Vec<EngineEvent> {
        self.maintain_at(Instant::now())
    }

    /// [`maintain`](Self::maintain) with the current time given, for tests.
    pub fn maintain_at(&mut self, now: Instant) -> Vec<EngineEvent> {
        let mut events = Vec::new();
        for live in self.live_in.iter_mut().flatten() {
            live.handle.collect_garbage();
        }
        for live in self.live_out.iter_mut().flatten() {
            live.handle.collect_garbage();
        }

        // A failed listing must not tear down devices that still work.
        let in_list = self.backend.input_devices().ok();
        let out_list = self.backend.output_devices().ok();

        for o in 0..self.outputs.len() {
            if let Some(reason) = self.lost(Direction::Output, o, out_list.as_deref()) {
                self.disconnect_output(o);
                events.push(EngineEvent::Lost {
                    dir: Direction::Output,
                    slot: o,
                    reason,
                });
            }
        }
        for i in 0..self.inputs.len() {
            if let Some(reason) = self.lost(Direction::Input, i, in_list.as_deref()) {
                self.disconnect_input(i);
                events.push(EngineEvent::Lost {
                    dir: Direction::Input,
                    slot: i,
                    reason,
                });
            }
        }

        self.release_silent(now, &mut events);

        // Outputs first, so a new input gets lanes to every output at once.
        if let Some(list) = &out_list {
            for o in 0..self.outputs.len() {
                if self.live_out[o].is_none() && !self.idle[o].released {
                    if let Some(device) = find_device(list, &self.outputs[o].pattern) {
                        events.push(self.connect_output(o, device));
                    }
                }
            }
        }
        if let Some(list) = &in_list {
            for i in 0..self.inputs.len() {
                if self.live_in[i].is_none() {
                    if let Some(device) = find_device(list, &self.inputs[i].pattern) {
                        events.push(self.connect_input(i, device));
                    }
                }
            }
        }
        self.block_loops();
        events
    }

    /// Silences every pair whose output feeds straight back into the input,
    /// e.g. "CABLE Input" to "CABLE Output": it would howl.
    fn block_loops(&self) {
        for (i, input) in self.inputs.iter().enumerate() {
            for (o, output) in self.outputs.iter().enumerate() {
                let looped = match (&input.device, &output.device) {
                    (Some((i_dev, _)), Some((o_dev, _))) => feeds_back(&o_dev.name, &i_dev.name),
                    _ => false,
                };
                self.matrix.set_blocked(i, o, looped);
            }
        }
    }

    /// Whether the pair is silenced because it would feed back.
    pub fn is_loop(&self, input: usize, output: usize) -> bool {
        self.matrix.is_blocked(input, output)
    }

    /// Closes outputs that had no signal for their `release_after` and marks
    /// released ones with signal again for reconnection.
    fn release_silent(&mut self, now: Instant, events: &mut Vec<EngineEvent>) {
        let activity: Vec<f32> = self
            .inputs
            .iter()
            .map(|s| s.meter.take_activity())
            .collect();
        for o in 0..self.outputs.len() {
            let Some(after) = self.config.outputs[o].release_after else {
                continue;
            };
            let signal = activity
                .iter()
                .enumerate()
                .any(|(i, &peak)| peak > SILENCE && self.matrix.effective_gain(i, o) > 0.0);
            let idle = &mut self.idle[o];
            if signal {
                idle.quiet_since = None;
                idle.released = false;
            } else if self.live_out[o].is_some() {
                let since = *idle.quiet_since.get_or_insert(now);
                if now.duration_since(since) >= after {
                    idle.quiet_since = None;
                    idle.released = true;
                    self.disconnect_output(o);
                    events.push(EngineEvent::Released { slot: o });
                }
            }
        }
    }

    fn lost(
        &self,
        dir: Direction,
        index: usize,
        list: Option<&[DeviceInfo]>,
    ) -> Option<LostReason> {
        let (failed, state) = match dir {
            Direction::Input => (
                self.live_in[index].as_ref()?.failed.load(Relaxed),
                &self.inputs[index],
            ),
            Direction::Output => (
                self.live_out[index].as_ref()?.failed.load(Relaxed),
                &self.outputs[index],
            ),
        };
        if failed {
            return Some(LostReason::StreamError);
        }
        let (device, _) = state.device.as_ref()?;
        match list {
            Some(list) if !list.iter().any(|d| d.id == device.id) => Some(LostReason::Removed),
            _ => None,
        }
    }

    fn connect_output(&mut self, o: usize, device: DeviceInfo) -> EngineEvent {
        let failed = |error: BackendError| EngineEvent::ConnectFailed {
            dir: Direction::Output,
            slot: o,
            error: error.0,
        };
        let format = match self.backend.output_format(&device.id) {
            Ok(f) => f,
            Err(e) => return failed(e),
        };
        let buffer_ms = self.config.outputs[o].buffer_ms;

        // One lane from every connected input.
        let mut lanes = Vec::with_capacity(self.inputs.len());
        let mut producers = Vec::new();
        for (i, state) in self.inputs.iter().enumerate() {
            match (&self.live_in[i], &state.device) {
                (Some(_), Some((_, in_format))) => {
                    let stats = Arc::clone(&self.lanes[i][o]);
                    let (producer, lane) = lane(
                        LaneSpec {
                            input_rate: in_format.sample_rate,
                            output_rate: format.sample_rate,
                            target_ms: buffer_ms,
                        },
                        Arc::clone(&stats),
                    );
                    lanes.push(Some(Box::new(lane)));
                    producers.push((i, (producer, stats)));
                }
                _ => lanes.push(None),
            }
        }

        let state = &self.outputs[o];
        let (mut node, handle) =
            OutputNode::new(o, lanes, Arc::clone(&self.matrix), Arc::clone(&state.meter));
        let (flag, on_error) = error_flag(state);
        let channels = format.channels as usize;
        let stream = match self.backend.open_output(
            &device.id,
            format,
            Box::new(move |data| node.render(data, channels)),
            on_error,
        ) {
            Ok(s) => s,
            Err(e) => return failed(e),
        };

        for (i, lane_input) in producers {
            if let Some(live) = &mut self.live_in[i] {
                // A full queue means the input stalled; it will be rebuilt.
                let _ = live.handle.send(InputCommand::SetLane(o, Some(lane_input)));
            }
        }
        self.live_out[o] = Some(Live {
            stream,
            handle,
            failed: flag,
        });
        self.outputs[o].device = Some((device.clone(), format));
        EngineEvent::Connected {
            dir: Direction::Output,
            slot: o,
            device,
            format,
        }
    }

    fn connect_input(&mut self, i: usize, device: DeviceInfo) -> EngineEvent {
        let failed = |error: BackendError| EngineEvent::ConnectFailed {
            dir: Direction::Input,
            slot: i,
            error: error.0,
        };
        let format = match self.backend.input_format(&device.id) {
            Ok(f) => f,
            Err(e) => return failed(e),
        };

        // One lane to every connected output.
        let mut producers = Vec::with_capacity(self.outputs.len());
        let mut lanes = Vec::new();
        for (o, state) in self.outputs.iter().enumerate() {
            match (&self.live_out[o], &state.device) {
                (Some(_), Some((_, out_format))) => {
                    let stats = Arc::clone(&self.lanes[i][o]);
                    let (producer, lane) = lane(
                        LaneSpec {
                            input_rate: format.sample_rate,
                            output_rate: out_format.sample_rate,
                            target_ms: self.config.outputs[o].buffer_ms,
                        },
                        Arc::clone(&stats),
                    );
                    producers.push(Some((producer, stats)));
                    lanes.push((o, Box::new(lane)));
                }
                _ => producers.push(None),
            }
        }

        let state = &self.inputs[i];
        let (mut node, handle) = InputNode::new(
            producers,
            Arc::clone(&state.meter),
            Arc::clone(&self.fx[i]),
            format.sample_rate,
        );
        let (flag, on_error) = error_flag(state);
        let channels = format.channels as usize;
        let stream = match self.backend.open_input(
            &device.id,
            format,
            Box::new(move |data| node.process(data, channels)),
            on_error,
        ) {
            Ok(s) => s,
            Err(e) => return failed(e),
        };

        for (o, lane) in lanes {
            if let Some(live) = &mut self.live_out[o] {
                let _ = live.handle.send(OutputCommand::SetLane(i, Some(lane)));
            }
        }
        self.live_in[i] = Some(Live {
            stream,
            handle,
            failed: flag,
        });
        self.inputs[i].device = Some((device.clone(), format));
        EngineEvent::Connected {
            dir: Direction::Input,
            slot: i,
            device,
            format,
        }
    }

    fn disconnect_output(&mut self, o: usize) {
        if let Some(live) = self.live_out[o].take() {
            live.close();
        }
        self.outputs[o].device = None;
        for live in self.live_in.iter_mut().flatten() {
            let _ = live.handle.send(InputCommand::SetLane(o, None));
        }
    }

    fn disconnect_input(&mut self, i: usize) {
        if let Some(live) = self.live_in[i].take() {
            live.close();
        }
        self.inputs[i].device = None;
        for live in self.live_out.iter_mut().flatten() {
            let _ = live.handle.send(OutputCommand::SetLane(i, None));
        }
    }
}

/// Whether playing on `output` reaches the recording device `input`: the
/// two ends of one virtual cable, named alike except for "Input"/"Output"
/// ("CABLE Input (VB-Audio Virtual Cable)", "VoiceMeeter Input" …).
pub fn feeds_back(output: &str, input: &str) -> bool {
    let output = output.to_lowercase();
    output.contains("input") && output.replacen("input", "output", 1) == input.to_lowercase()
}

/// Finds a device whose id equals `pattern` or whose name contains it
/// (case-insensitive).
pub fn find_device(devices: &[DeviceInfo], pattern: &str) -> Option<DeviceInfo> {
    let needle = pattern.to_lowercase();
    devices
        .iter()
        .find(|d| d.id == pattern)
        .or_else(|| {
            devices
                .iter()
                .find(|d| d.name.to_lowercase().contains(&needle))
        })
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{ErrorCallback, InputCallback, OutputCallback};
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    #[derive(Default)]
    struct FakeState {
        inputs: Vec<(DeviceInfo, StreamFormat)>,
        outputs: Vec<(DeviceInfo, StreamFormat)>,
        in_streams: BTreeMap<String, (InputCallback, ErrorCallback, StreamFormat)>,
        out_streams: BTreeMap<String, (OutputCallback, ErrorCallback, StreamFormat)>,
    }

    /// Backend whose devices a test plugs in and out and whose callbacks it
    /// drives by hand.
    #[derive(Clone, Default)]
    struct FakeBackend(Rc<RefCell<FakeState>>);

    struct FakeStream {
        id: String,
        state: Rc<RefCell<FakeState>>,
    }

    impl Drop for FakeStream {
        fn drop(&mut self) {
            let mut s = self.state.borrow_mut();
            s.in_streams.remove(&self.id);
            s.out_streams.remove(&self.id);
        }
    }

    fn dev(id: &str, rate: u32, channels: u16) -> (DeviceInfo, StreamFormat) {
        (
            DeviceInfo {
                id: format!("{{{id}}}"),
                name: id.to_string(),
            },
            StreamFormat {
                sample_rate: rate,
                channels,
            },
        )
    }

    impl FakeBackend {
        fn plug_input(&self, id: &str, rate: u32, channels: u16) {
            self.0.borrow_mut().inputs.push(dev(id, rate, channels));
        }

        fn plug_output(&self, id: &str, rate: u32, channels: u16) {
            self.0.borrow_mut().outputs.push(dev(id, rate, channels));
        }

        /// Removes the device and makes its stream report an error.
        fn unplug(&self, id: &str) {
            let id = format!("{{{id}}}");
            let mut s = self.0.borrow_mut();
            s.inputs.retain(|(d, _)| d.id != id);
            s.outputs.retain(|(d, _)| d.id != id);
            if let Some((_, err, _)) = s.in_streams.get_mut(&id) {
                err();
            }
            if let Some((_, err, _)) = s.out_streams.get_mut(&id) {
                err();
            }
        }

        fn fail_stream(&self, id: &str) {
            let id = format!("{{{id}}}");
            let mut s = self.0.borrow_mut();
            if let Some((_, err, _)) = s.out_streams.get_mut(&id) {
                err();
            }
        }

        /// Runs every stream for `ms` in 10 ms periods. Every input
        /// delivers the constant `level(id)`. Returns the last block of
        /// each output by name.
        fn pump(&self, ms: u32, level: impl Fn(&str) -> f32) -> BTreeMap<String, Vec<f32>> {
            let mut last = BTreeMap::new();
            for _ in 0..ms / 10 {
                let mut s = self.0.borrow_mut();
                for (id, (cb, _, f)) in s.in_streams.iter_mut() {
                    let name = id.trim_matches(|c| c == '{' || c == '}');
                    let frames = f.sample_rate as usize / 100;
                    cb(&vec![level(name); frames * f.channels as usize]);
                }
                for (id, (cb, _, f)) in s.out_streams.iter_mut() {
                    let frames = f.sample_rate as usize / 100;
                    let mut buf = vec![0.0; frames * f.channels as usize];
                    cb(&mut buf);
                    last.insert(id.trim_matches(|c| c == '{' || c == '}').to_string(), buf);
                }
            }
            last
        }
    }

    fn find(list: &[(DeviceInfo, StreamFormat)], id: &str) -> Result<StreamFormat, BackendError> {
        list.iter()
            .find(|(d, _)| d.id == id)
            .map(|(_, f)| *f)
            .ok_or_else(|| BackendError(format!("{id} not found")))
    }

    impl AudioBackend for FakeBackend {
        type Stream = FakeStream;
        fn input_devices(&self) -> Result<Vec<DeviceInfo>, BackendError> {
            Ok(self
                .0
                .borrow()
                .inputs
                .iter()
                .map(|(d, _)| d.clone())
                .collect())
        }
        fn output_devices(&self) -> Result<Vec<DeviceInfo>, BackendError> {
            Ok(self
                .0
                .borrow()
                .outputs
                .iter()
                .map(|(d, _)| d.clone())
                .collect())
        }
        fn default_input(&self) -> Option<DeviceInfo> {
            None
        }
        fn default_output(&self) -> Option<DeviceInfo> {
            None
        }
        fn input_format(&self, id: &str) -> Result<StreamFormat, BackendError> {
            find(&self.0.borrow().inputs, id)
        }
        fn output_format(&self, id: &str) -> Result<StreamFormat, BackendError> {
            find(&self.0.borrow().outputs, id)
        }
        fn open_input(
            &self,
            id: &str,
            format: StreamFormat,
            data: InputCallback,
            error: ErrorCallback,
        ) -> Result<FakeStream, BackendError> {
            let mut s = self.0.borrow_mut();
            s.in_streams.insert(id.to_string(), (data, error, format));
            Ok(FakeStream {
                id: id.to_string(),
                state: Rc::clone(&self.0),
            })
        }
        fn open_output(
            &self,
            id: &str,
            format: StreamFormat,
            data: OutputCallback,
            error: ErrorCallback,
        ) -> Result<FakeStream, BackendError> {
            let mut s = self.0.borrow_mut();
            s.out_streams.insert(id.to_string(), (data, error, format));
            Ok(FakeStream {
                id: id.to_string(),
                state: Rc::clone(&self.0),
            })
        }
    }

    fn config(inputs: &[&str], outputs: &[(&str, u32)]) -> EngineConfig {
        EngineConfig {
            inputs: inputs
                .iter()
                .map(|d| InputConfig {
                    device: d.to_string(),
                })
                .collect(),
            outputs: outputs
                .iter()
                .map(|(d, ms)| OutputConfig {
                    device: d.to_string(),
                    buffer_ms: *ms,
                    release_after: None,
                })
                .collect(),
        }
    }

    fn level(name: &str) -> f32 {
        match name {
            "mic" => 0.2,
            "game" => 0.4,
            _ => 0.0,
        }
    }

    fn assert_level(block: &[f32], expected: f32, tol: f32) {
        assert!(
            block.iter().all(|v| (v - expected).abs() < tol),
            "expected {expected}, got {:?}",
            &block[..4.min(block.len())]
        );
    }

    fn connected(events: &[EngineEvent]) -> usize {
        events
            .iter()
            .filter(|e| matches!(e, EngineEvent::Connected { .. }))
            .count()
    }

    #[test]
    fn routes_two_inputs_to_two_outputs() {
        let b = FakeBackend::default();
        b.plug_input("mic", 48_000, 1);
        b.plug_input("game", 48_000, 2);
        b.plug_output("usb", 48_000, 2);
        b.plug_output("bt", 48_000, 2);
        let mut engine = Engine::new(
            b.clone(),
            config(&["mic", "game"], &[("usb", 20), ("bt", 60)]),
        )
        .unwrap();
        assert_eq!(connected(&engine.maintain()), 4);

        // Output 0 hears only the mic, output 1 hears both at -6 dB.
        engine.matrix.set_muted(1, 0, true);
        engine.matrix.set_gain_db(0, 1, -6.0);
        engine.matrix.set_gain_db(1, 1, -6.0);
        let out = b.pump(3000, level);
        assert_level(&out["usb"], 0.2, 1e-3);
        assert_level(&out["bt"], 0.6 * 0.501, 2e-3);
        assert!(engine.inputs()[1].meter.take()[0] > 0.39);
        for i in 0..2 {
            for o in 0..2 {
                assert_eq!(engine.lane_stats(i, o).underruns.load(Relaxed), 0);
            }
        }
    }

    #[test]
    fn output_comes_and_goes_without_disturbing_others() {
        let b = FakeBackend::default();
        b.plug_input("mic", 48_000, 1);
        b.plug_output("usb", 48_000, 2);
        b.plug_output("bt", 48_000, 2);
        let mut engine =
            Engine::new(b.clone(), config(&["mic"], &[("usb", 20), ("bt", 80)])).unwrap();
        engine.maintain();
        b.pump(2000, level);

        b.unplug("bt");
        let events = engine.maintain();
        assert_eq!(
            events,
            vec![EngineEvent::Lost {
                dir: Direction::Output,
                slot: 1,
                reason: LostReason::StreamError
            }]
        );
        assert!(engine.outputs()[1].device.is_none());
        let out = b.pump(2000, level);
        assert_level(&out["usb"], 0.2, 1e-3);
        assert!(!out.contains_key("bt"));
        assert_eq!(engine.lane_stats(0, 0).underruns.load(Relaxed), 0);

        // Back in hands-free mode at 16 kHz mono: rebuilt with the new format.
        b.plug_output("bt", 16_000, 1);
        assert_eq!(connected(&engine.maintain()), 1);
        let out = b.pump(3000, level);
        assert_level(&out["usb"], 0.2, 1e-3);
        assert_level(&out["bt"], 0.2, 1e-2);
        assert_eq!(engine.lane_stats(0, 0).underruns.load(Relaxed), 0);
        assert_eq!(
            engine.outputs()[1].device.as_ref().unwrap().1.sample_rate,
            16_000
        );
    }

    #[test]
    fn silent_output_is_released_and_comes_back_with_signal() {
        let b = FakeBackend::default();
        b.plug_input("mic", 48_000, 1);
        b.plug_input("game", 48_000, 2);
        b.plug_output("usb", 48_000, 2);
        b.plug_output("bt", 48_000, 2);
        let mut cfg = config(&["mic", "game"], &[("usb", 20), ("bt", 80)]);
        cfg.outputs[1].release_after = Some(Duration::from_secs(5));
        let mut engine = Engine::new(b.clone(), cfg).unwrap();
        // The mic goes only to usb, so bt hears nothing while the game is quiet.
        engine.matrix.set_muted(0, 1, true);
        let t0 = Instant::now();
        let at = |s: u64| t0 + Duration::from_secs(s);
        assert_eq!(connected(&engine.maintain_at(at(0))), 4);
        let quiet_game = |n: &str| if n == "game" { 0.0 } else { level(n) };

        b.pump(100, quiet_game);
        assert!(engine.maintain_at(at(1)).is_empty());
        b.pump(100, quiet_game);
        assert_eq!(
            engine.maintain_at(at(6)),
            vec![EngineEvent::Released { slot: 1 }]
        );
        assert!(engine.is_released(1));
        assert!(engine.outputs()[1].device.is_none());
        let out = b.pump(100, quiet_game);
        assert!(!out.contains_key("bt"));
        assert_level(&out["usb"], 0.2, 1e-3);
        assert!(engine.maintain_at(at(20)).is_empty());

        // The game plays again: bt opens and hears it.
        b.pump(100, level);
        assert_eq!(connected(&engine.maintain_at(at(21))), 1);
        assert!(!engine.is_released(1));
        let out = b.pump(3000, level);
        assert_level(&out["bt"], 0.4, 1e-2);
        assert!(engine.maintain_at(at(30)).is_empty());
    }

    #[test]
    fn cable_ends_feed_back() {
        assert!(feeds_back(
            "CABLE Input (VB-Audio Virtual Cable)",
            "CABLE Output (VB-Audio Virtual Cable)"
        ));
        assert!(feeds_back(
            "CABLE-A Input (VB-Audio Cable A)",
            "CABLE-A Output (VB-Audio Cable A)"
        ));
        assert!(feeds_back(
            "VoiceMeeter Input (VB-Audio VoiceMeeter VAIO)",
            "VoiceMeeter Output (VB-Audio VoiceMeeter VAIO)"
        ));
        assert!(!feeds_back(
            "CABLE-A Input (VB-Audio Cable A)",
            "CABLE-B Output (VB-Audio Cable B)"
        ));
        assert!(!feeds_back(
            "Lautsprecher (USB Audio)",
            "Mikrofon (USB Audio)"
        ));
    }

    #[test]
    fn loop_through_a_cable_stays_silent() {
        let b = FakeBackend::default();
        b.plug_input("CABLE Output", 48_000, 2);
        b.plug_input("mic", 48_000, 1);
        b.plug_output("CABLE Input", 48_000, 2);
        b.plug_output("usb", 48_000, 2);
        let mut engine = Engine::new(
            b.clone(),
            config(
                &["CABLE Output", "mic"],
                &[("CABLE Input", 20), ("usb", 20)],
            ),
        )
        .unwrap();
        engine.maintain();
        assert!(engine.is_loop(0, 0));
        assert!(!engine.is_loop(0, 1) && !engine.is_loop(1, 0));
        let out = b.pump(3000, |n| if n == "CABLE Output" { 0.4 } else { level(n) });
        assert_level(&out["CABLE Input"], 0.2, 1e-3);
        assert_level(&out["usb"], 0.6, 2e-3);

        // Gone with the device: a fresh one with another name is no loop.
        b.unplug("CABLE Input");
        engine.maintain();
        assert!(!engine.is_loop(0, 0));
    }

    #[test]
    fn input_plugged_in_later() {
        let b = FakeBackend::default();
        b.plug_output("usb", 48_000, 2);
        let mut engine = Engine::new(b.clone(), config(&["mic"], &[("usb", 20)])).unwrap();
        assert_eq!(connected(&engine.maintain()), 1);
        assert_level(&b.pump(500, level)["usb"], 0.0, 1e-6);

        b.plug_input("mic", 44_100, 1);
        assert_eq!(connected(&engine.maintain()), 1);
        assert_level(&b.pump(2000, level)["usb"], 0.2, 1e-2);

        b.unplug("mic");
        engine.maintain();
        b.pump(200, level);
        assert_level(&b.pump(100, level)["usb"], 0.0, 1e-3);
    }

    #[test]
    fn stream_error_reconnects_present_device() {
        let b = FakeBackend::default();
        b.plug_input("mic", 48_000, 1);
        b.plug_output("usb", 48_000, 2);
        let mut engine = Engine::new(b.clone(), config(&["mic"], &[("usb", 20)])).unwrap();
        engine.maintain();
        b.pump(1000, level);

        b.fail_stream("usb");
        let events = engine.maintain();
        assert_eq!(events.len(), 2, "{events:?}");
        assert!(matches!(
            events[0],
            EngineEvent::Lost {
                reason: LostReason::StreamError,
                ..
            }
        ));
        assert!(matches!(
            events[1],
            EngineEvent::Connected {
                dir: Direction::Output,
                ..
            }
        ));
        assert_level(&b.pump(1000, level)["usb"], 0.2, 1e-3);
        assert_eq!(engine.outputs()[0].stream_errors.load(Relaxed), 1);
    }

    #[test]
    fn missing_devices_stay_offline() {
        let b = FakeBackend::default();
        let mut engine = Engine::new(b.clone(), config(&["mic"], &[("usb", 20)])).unwrap();
        assert!(engine.maintain().is_empty());
        assert!(engine.inputs()[0].device.is_none());
    }

    #[test]
    fn rejects_empty_config() {
        assert!(matches!(
            Engine::new(FakeBackend::default(), EngineConfig::default()),
            Err(EngineError::NoDevices)
        ));
    }

    #[test]
    fn finds_devices_by_id_or_name() {
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
