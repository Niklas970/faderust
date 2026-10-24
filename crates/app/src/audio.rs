//! Runs the engine on its own thread and publishes its state to the UI.

use std::path::PathBuf;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use faderust_engine::backend::cpal::CpalBackend;
use faderust_engine::meter::to_db;
use faderust_engine::{
    Direction, Engine, EngineConfig, EngineEvent, InputConfig, LostReason, OutputConfig, SlotState,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::config::{Config, Fx};

const EMIT_EVERY: Duration = Duration::from_millis(50);
const MAINTAIN_EVERY: Duration = Duration::from_secs(1);
/// Fader moves are saved at most this often.
const SAVE_EVERY: Duration = Duration::from_secs(2);

pub enum Command {
    SetGain(usize, usize, f32),
    SetMuted(usize, usize, bool),
    /// Moves a fader by a dB step (Stream Deck dials and keys).
    AddGain(usize, usize, f32),
    ToggleMuted(usize, usize),
    SetFx(usize, Fx),
    /// The gains in the config changed as a whole (preset loaded).
    ApplyGains,
    Reconfigure,
}

/// A message for the user, sent as the `notice` event.
#[derive(Serialize, Clone)]
struct Notice {
    /// "info", "warn" or "error".
    level: &'static str,
    text: String,
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
    /// Inputs only.
    #[serde(skip_serializing_if = "Option::is_none")]
    fx: Option<Fx>,
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
    apply_gains(&engine, config);
    for (i, input) in config.inputs.iter().enumerate() {
        engine.input_fx(i).set(input.fx.into());
    }
    Some(engine)
}

fn apply_gains(engine: &Engine<CpalBackend>, config: &Config) {
    for (i, row) in config.gains.iter().enumerate() {
        for (o, g) in row.iter().enumerate() {
            engine.matrix.set_gain_db(i, o, g.db);
            engine.matrix.set_muted(i, o, g.muted);
        }
    }
}

/// Turns engine events into messages, without repeating the same failure
/// every second while a device stays unavailable.
#[derive(Default)]
struct Notices {
    /// Last failure text per (is_output, slot).
    failing: std::collections::HashMap<(bool, usize), String>,
}

impl Notices {
    /// `quiet` skips "connected" messages, for the start-up connections.
    fn notice(&mut self, config: &Config, event: &EngineEvent, quiet: bool) -> Option<Notice> {
        let label = |dir: &Direction, slot: usize| {
            let label = match dir {
                Direction::Input => config.inputs.get(slot).map(|e| &e.label),
                Direction::Output => config.outputs.get(slot).map(|e| &e.label),
            };
            label.cloned().unwrap_or_default()
        };
        let key = |dir: &Direction, slot: usize| (*dir == Direction::Output, slot);
        match event {
            EngineEvent::Connected {
                dir, slot, device, ..
            } => {
                let was_failing = self.failing.remove(&key(dir, *slot)).is_some();
                (!quiet || was_failing).then(|| Notice {
                    level: "info",
                    text: format!("„{}“ verbunden: {}", label(dir, *slot), device.name),
                })
            }
            EngineEvent::Lost { dir, slot, reason } => Some(Notice {
                level: "warn",
                text: match reason {
                    LostReason::Removed => {
                        format!("„{}“ getrennt. Faderust verbindet neu, sobald das Gerät wieder da ist.", label(dir, *slot))
                    }
                    LostReason::StreamError => {
                        format!(
                            "„{}“ meldet einen Fehler und wird neu verbunden.",
                            label(dir, *slot)
                        )
                    }
                },
            }),
            EngineEvent::ConnectFailed { dir, slot, error } => {
                let previous = self.failing.insert(key(dir, *slot), error.clone());
                (previous.as_ref() != Some(error)).then(|| Notice {
                    level: "error",
                    text: format!(
                        "„{}“ lässt sich nicht öffnen: {error}. Nutzt ein anderes Programm das Gerät im Exklusivmodus?",
                        label(dir, *slot)
                    ),
                })
            }
        }
    }
}

fn run(app: AppHandle, config: Arc<Mutex<Config>>, path: PathBuf, rx: Receiver<Command>) {
    let mut engine = build(&config.lock().unwrap());
    let mut last_maintain: Option<Instant> = None;
    let mut last_emit = Instant::now();
    let mut dirty_since: Option<Instant> = None;
    let mut notices = Notices::default();
    // The first connections after a (re)build are expected, not news.
    let mut quiet = true;

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
                            notices = Notices::default();
                            quiet = true;
                        }
                        Command::ApplyGains => {
                            if let Some(e) = &engine {
                                apply_gains(e, &config.lock().unwrap());
                            }
                        }
                        Command::SetFx(i, fx) => {
                            let mut c = config.lock().unwrap();
                            if let Some(input) = c.inputs.get_mut(i) {
                                let settings = faderust_engine::FxSettings::from(fx).sanitized();
                                input.fx = settings.into();
                                if let Some(e) = &engine {
                                    e.input_fx(i).set(settings);
                                }
                                dirty_since.get_or_insert_with(Instant::now);
                            }
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
                        Command::AddGain(i, o, delta) => {
                            let step = |g: &mut crate::config::Gain| g.db = step_db(g.db, delta);
                            if let Some(g) = gain_mut(&config, i, o, step) {
                                if let Some(e) = &engine {
                                    e.matrix.set_gain_db(i, o, g.db);
                                }
                                dirty_since.get_or_insert_with(Instant::now);
                            }
                        }
                        Command::ToggleMuted(i, o) => {
                            if let Some(g) = gain_mut(&config, i, o, |g| g.muted = !g.muted) {
                                if let Some(e) = &engine {
                                    e.matrix.set_muted(i, o, g.muted);
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
                let events = e.maintain();
                let c = config.lock().unwrap();
                for event in &events {
                    eprintln!("{event}");
                    if let Some(n) = notices.notice(&c, event, quiet) {
                        let _ = app.emit("notice", n);
                    }
                }
                quiet = false;
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

/// One dial click or key press on a fader, within the UI's range: below
/// -60 dB is off, and off comes back at -60 dB.
fn step_db(db: f32, delta: f32) -> f32 {
    let next = db.max(-60.0) + delta;
    if next < -60.0 {
        -120.0
    } else {
        next.min(12.0)
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
        fx: None,
    }
}

fn snapshot(config: &Config, engine: Option<&Engine<CpalBackend>>) -> UiState {
    let inputs = config
        .inputs
        .iter()
        .enumerate()
        .map(|(i, c)| UiDevice {
            fx: Some(c.fx),
            ..device(
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

#[cfg(test)]
mod tests {
    use super::step_db;

    #[test]
    fn steps_stay_in_fader_range() {
        assert_eq!(step_db(0.0, 1.0), 1.0);
        assert_eq!(step_db(11.5, 1.0), 12.0);
        assert_eq!(step_db(-59.0, -2.0), -120.0);
        assert_eq!(step_db(-120.0, 1.0), -59.0);
        assert_eq!(step_db(-120.0, -1.0), -120.0);
    }
}
