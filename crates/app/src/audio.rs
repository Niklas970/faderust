//! Runs the engine on its own thread and publishes its state to the UI.

use std::path::PathBuf;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use faderust_engine::backend::cpal::CpalBackend;
use faderust_engine::meter::to_db;
use faderust_engine::{Engine, EngineConfig, InputConfig, OutputConfig, SlotState};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::config::Config;

const EMIT_EVERY: Duration = Duration::from_millis(50);
const MAINTAIN_EVERY: Duration = Duration::from_secs(1);
/// Fader moves are saved at most this often.
const SAVE_EVERY: Duration = Duration::from_secs(2);

pub enum Command {
    SetGain(usize, usize, f32),
    SetMuted(usize, usize, bool),
    Reconfigure,
}

/// Snapshot sent to the UI as the `state` event.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct UiState {
    inputs: Vec<UiDevice>,
    outputs: Vec<UiDevice>,
    gains: Vec<Vec<crate::config::Gain>>,
    lanes: Vec<Vec<UiLane>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct UiDevice {
    label: String,
    device: String,
    connected: bool,
    device_name: Option<String>,
    level_db: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    buffer_ms: Option<u32>,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct UiLane {
    fill_ms: f32,
    correction_ppm: i32,
    underruns: u64,
}

/// Starts the audio thread. `config` is shared with the UI commands; the
/// thread rebuilds the engine on [`Command::Reconfigure`].
pub fn spawn(app: AppHandle, config: Arc<Mutex<Config>>, path: PathBuf) -> Sender<Command> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("faderust-engine".into())
        .spawn(move || run(app, config, path, rx))
        .expect("spawn engine thread");
    tx
}

fn build(config: &Config) -> Option<Engine<CpalBackend>> {
    let engine_config = EngineConfig {
        inputs: config
            .inputs
            .iter()
            .map(|i| InputConfig {
                device: i.device.clone(),
            })
            .collect(),
        outputs: config
            .outputs
            .iter()
            .map(|o| OutputConfig {
                device: o.device.clone(),
                buffer_ms: o.buffer_ms,
            })
            .collect(),
    };
    // Needs at least one input and one output; the UI then offers setup.
    let engine = Engine::new(CpalBackend::new(), engine_config).ok()?;
    for (i, row) in config.gains.iter().enumerate() {
        for (o, g) in row.iter().enumerate() {
            engine.matrix.set_gain_db(i, o, g.db);
            engine.matrix.set_muted(i, o, g.muted);
        }
    }
    Some(engine)
}

fn run(app: AppHandle, config: Arc<Mutex<Config>>, path: PathBuf, rx: Receiver<Command>) {
    let mut engine = build(&config.lock().unwrap());
    let mut last_maintain: Option<Instant> = None;
    let mut last_emit = Instant::now();
    let mut dirty_since: Option<Instant> = None;

    loop {
        match rx.recv_timeout(EMIT_EVERY) {
            Ok(cmd) => {
                let mut pending = vec![cmd];
                pending.extend(rx.try_iter());
                for cmd in pending {
                    match cmd {
                        Command::Reconfigure => {
                            // Drop the old engine first so devices are free.
                            drop(engine.take());
                            engine = build(&config.lock().unwrap());
                            last_maintain = None;
                        }
                        Command::SetGain(i, o, db) => {
                            if let Some(g) = gain_mut(&config, i, o, |g| g.db = db) {
                                if let Some(e) = &engine {
                                    e.matrix.set_gain_db(i, o, g.db);
                                }
                                dirty_since.get_or_insert_with(Instant::now);
                            }
                        }
                        Command::SetMuted(i, o, muted) => {
                            if gain_mut(&config, i, o, |g| g.muted = muted).is_some() {
                                if let Some(e) = &engine {
                                    e.matrix.set_muted(i, o, muted);
                                }
                                dirty_since.get_or_insert_with(Instant::now);
                            }
                        }
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        if let Some(e) = &mut engine {
            if last_maintain.is_none_or(|t| t.elapsed() >= MAINTAIN_EVERY) {
                for event in e.maintain() {
                    eprintln!("{event}");
                }
                last_maintain = Some(Instant::now());
            }
        }

        if last_emit.elapsed() >= EMIT_EVERY {
            let state = snapshot(&config.lock().unwrap(), engine.as_ref());
            let _ = app.emit("state", state);
            last_emit = Instant::now();
        }

        if dirty_since.is_some_and(|t| t.elapsed() >= SAVE_EVERY) {
            if let Err(e) = config.lock().unwrap().save(&path) {
                eprintln!("saving {}: {e}", path.display());
            }
            dirty_since = None;
        }
    }
}

/// Applies `f` to one gain entry if it exists and returns the new value.
fn gain_mut(
    config: &Mutex<Config>,
    i: usize,
    o: usize,
    f: impl FnOnce(&mut crate::config::Gain),
) -> Option<crate::config::Gain> {
    let mut c = config.lock().unwrap();
    let g = c.gains.get_mut(i)?.get_mut(o)?;
    f(g);
    g.db = if g.db.is_finite() {
        g.db.clamp(-120.0, 12.0)
    } else {
        -120.0
    };
    Some(*g)
}

fn device(
    label: &str,
    pattern: &str,
    slot: Option<&SlotState>,
    buffer_ms: Option<u32>,
) -> UiDevice {
    let connected = slot.and_then(|s| s.device.as_ref());
    let level_db = slot
        .filter(|_| connected.is_some())
        .map(|s| {
            let [l, r] = s.meter.take();
            to_db(l.max(r))
        })
        .unwrap_or(-120.0);
    UiDevice {
        label: label.to_string(),
        device: pattern.to_string(),
        connected: connected.is_some(),
        device_name: connected.map(|(d, _)| d.name.clone()),
        level_db,
        buffer_ms,
    }
}

fn snapshot(config: &Config, engine: Option<&Engine<CpalBackend>>) -> UiState {
    let inputs = config
        .inputs
        .iter()
        .enumerate()
        .map(|(i, c)| {
            device(
                &c.label,
                &c.device,
                engine.and_then(|e| e.inputs().get(i)),
                None,
            )
        })
        .collect();
    let outputs = config
        .outputs
        .iter()
        .enumerate()
        .map(|(o, c)| {
            device(
                &c.label,
                &c.device,
                engine.and_then(|e| e.outputs().get(o)),
                Some(c.buffer_ms),
            )
        })
        .collect();
    let lanes = (0..config.inputs.len())
        .map(|i| {
            (0..config.outputs.len())
                .map(|o| {
                    let Some(e) = engine else {
                        return UiLane::default();
                    };
                    let rate = e.inputs()[i]
                        .device
                        .as_ref()
                        .map_or(48_000, |(_, f)| f.sample_rate);
                    let s = e.lane_stats(i, o);
                    UiLane {
                        fill_ms: s.fill_frames.load(Relaxed) as f32 * 1000.0 / rate as f32,
                        correction_ppm: s.correction_ppm.load(Relaxed),
                        underruns: s.underruns.load(Relaxed),
                    }
                })
                .collect()
        })
        .collect();
    UiState {
        inputs,
        outputs,
        gains: config.gains.clone(),
        lanes,
    }
}
