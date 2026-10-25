//! Runs the engine on its own thread and publishes its state to the UI.

use std::path::PathBuf;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use faderust_engine::backend::cpal::CpalBackend;
use faderust_engine::lang::tr;
use faderust_engine::meter::to_db;
use faderust_engine::{
    Direction, Engine, EngineConfig, EngineEvent, InputConfig, LostReason, OutputConfig, SlotState,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::config::{Config, Fx};
use crate::log;

const EMIT_EVERY: Duration = Duration::from_millis(50);
const MAINTAIN_EVERY: Duration = Duration::from_secs(1);
/// How often to check whether anyone sees the window.
const VISIBLE_EVERY: Duration = Duration::from_millis(250);
/// Fader moves are saved at most this often.
const SAVE_EVERY: Duration = Duration::from_secs(2);

pub enum Command {
    SetGain(usize, usize, f32),
    SetMuted(usize, usize, bool),
    /// Moves a fader by a dB step (Stream Deck dials and keys).
    AddGain(usize, usize, f32),
    ToggleMuted(usize, usize),
    /// Master fader of an output.
    SetMasterGain(usize, f32),
    SetMasterMuted(usize, bool),
    /// Mute of a whole input.
    SetInputMuted(usize, bool),
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
pub(crate) struct UiState {
    pub(crate) inputs: Vec<UiDevice>,
    pub(crate) outputs: Vec<UiDevice>,
    gains: Vec<Vec<crate::config::Gain>>,
    pub(crate) lanes: Vec<Vec<UiLane>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UiDevice {
    pub(crate) label: String,
    pub(crate) device: String,
    pub(crate) connected: bool,
    device_name: Option<String>,
    level_db: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) buffer_ms: Option<u32>,
    /// Outputs only: closed while nothing plays for them.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub(crate) released: bool,
    /// Inputs only.
    #[serde(skip_serializing_if = "Option::is_none")]
    fx: Option<Fx>,
    /// Outputs only.
    #[serde(skip_serializing_if = "Option::is_none")]
    master: Option<crate::config::Gain>,
    /// Inputs only.
    #[serde(skip_serializing_if = "Option::is_none")]
    muted: Option<bool>,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UiLane {
    fill_ms: f32,
    correction_ppm: i32,
    pub(crate) underruns: u64,
    /// Input frames lost because the output did not take them in time.
    pub(crate) dropped_frames: u64,
    /// Silenced because the output feeds back into the input.
    pub(crate) looped: bool,
}

/// Starts the audio thread. `config` is shared with the UI commands; the
/// thread rebuilds the engine on [`Command::Reconfigure`].
pub fn spawn(app: AppHandle, config: Arc<Mutex<Config>>, path: PathBuf) -> Sender<Command> {
    let (tx, rx) = mpsc::channel();
    // A program recording that took Faderust down last time stays off until
    // the next start, so the app comes up again.
    #[cfg(windows)]
    let skip = faderust_engine::apps::capture::crashed_last_time();
    #[cfg(not(windows))]
    let skip: Option<String> = None;
    if let Some(exe) = &skip {
        log::write(&format!(
            "Programm-Aufnahme {exe} beim letzten Start abgestürzt, übersprungen"
        ));
    }
    thread::Builder::new()
        .name("faderust-engine".into())
        .spawn(move || run(app, config, path, rx, skip))
        .expect("spawn engine thread");
    tx
}

/// Whether `device` is the program input that crashed Faderust last time
/// (`skip`, an exe name) and must stay off for this run.
fn skipped(device: &str, skip: Option<&str>) -> bool {
    skip.is_some_and(|exe| {
        device
            .strip_prefix("app:")
            .is_some_and(|d| d.eq_ignore_ascii_case(exe))
    })
}

fn build(config: &Config, skip: Option<&str>) -> Option<Engine<CpalBackend>> {
    let engine_config = EngineConfig {
        inputs: config
            .inputs
            .iter()
            .map(|i| InputConfig {
                device: if skipped(&i.device, skip) {
                    format!("{}#aus", i.device)
                } else {
                    i.device.clone()
                },
            })
            .collect(),
        outputs: config
            .outputs
            .iter()
            .map(|o| OutputConfig {
                device: o.device.clone(),
                buffer_ms: o.buffer_ms,
                release_after: (o.release_secs > 0)
                    .then(|| Duration::from_secs(o.release_secs.into())),
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
    for (i, input) in config.inputs.iter().enumerate() {
        engine.matrix.set_input_muted(i, input.muted);
    }
    for (o, out) in config.outputs.iter().enumerate() {
        engine.matrix.set_master_db(o, out.master.db);
        engine.matrix.set_master_muted(o, out.master.muted);
    }
}

/// WASAPI's "device in use" (AUDCLNT_E_DEVICE_IN_USE), as cpal prints it.
const DEVICE_IN_USE: &str = "-2004287478";
/// WASAPI's "device invalidated" (AUDCLNT_E_DEVICE_INVALIDATED).
const DEVICE_INVALIDATED: &str = "-2004287484";

/// The cause of a failed connection in words a user can act on.
fn explain(error: &str) -> String {
    if error.contains(DEVICE_IN_USE) {
        tr(
            "Das Gerät ist belegt. Bei Bluetooth-Kopfhörern nutzt meist ein Programm (z. B. Discord) ihr Headset-Mikro und schaltet sie in den Freisprechmodus: dort ein anderes Mikro wählen. Sonst hält ein Programm das Gerät exklusiv fest, etwa Voicemeeter, eine DAW oder ein ASIO-Treiber. Faderust verbindet von selbst, sobald es frei ist.",
            "The device is in use. With Bluetooth headphones, a program (e.g. Discord) usually uses their headset microphone and switches them to hands-free mode: pick another microphone there. Otherwise a program holds the device exclusively, such as Voicemeeter, a DAW or an ASIO driver. Faderust reconnects by itself as soon as it is free.",
        )
    } else if error.contains(DEVICE_INVALIDATED) {
        tr(
            "Das Gerät wurde gerade umgestellt oder entfernt. Faderust versucht es weiter.",
            "The device was just changed or removed. Faderust keeps trying.",
        )
    } else {
        format!(
            "{error}. {}",
            tr(
                "Nutzt ein anderes Programm das Gerät im Exklusivmodus?",
                "Is another program using the device in exclusive mode?"
            )
        )
    }
}

/// Turns engine events into messages, without repeating the same failure
/// every second while a device stays unavailable.
#[derive(Default)]
struct Notices {
    /// Last failure text per (is_output, slot).
    failing: std::collections::HashMap<(bool, usize), String>,
    /// Outputs released after silence: coming back is routine, not news.
    released: std::collections::HashSet<usize>,
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
                let resumed = *dir == Direction::Output && self.released.remove(slot);
                ((!quiet && !resumed) || was_failing).then(|| Notice {
                    level: "info",
                    text: format!(
                        "{}: {}",
                        tr("„{}“ verbunden", "“{}” connected").replace("{}", &label(dir, *slot)),
                        device.name
                    ),
                })
            }
            EngineEvent::Lost { dir, slot, reason } => Some(Notice {
                level: "warn",
                text: match reason {
                    LostReason::Removed => tr(
                        "„{}“ getrennt. Faderust verbindet neu, sobald das Gerät wieder da ist.",
                        "“{}” disconnected. Faderust reconnects as soon as the device is back.",
                    )
                    .replace("{}", &label(dir, *slot)),
                    LostReason::StreamError => tr(
                        "„{}“ meldet einen Fehler und wird neu verbunden.",
                        "“{}” reports an error and is being reconnected.",
                    )
                    .replace("{}", &label(dir, *slot)),
                },
            }),
            EngineEvent::Released { slot } => {
                self.released.insert(*slot);
                None
            }
            EngineEvent::ConnectFailed { dir, slot, error } => {
                let previous = self.failing.insert(key(dir, *slot), error.clone());
                (previous.as_ref() != Some(error)).then(|| Notice {
                    level: "error",
                    text: format!(
                        "{} {}",
                        tr("„{}“ lässt sich nicht öffnen:", "“{}” cannot be opened:")
                            .replace("{}", &label(dir, *slot)),
                        explain(error)
                    ),
                })
            }
        }
    }
}

fn run(
    app: AppHandle,
    config: Arc<Mutex<Config>>,
    path: PathBuf,
    rx: Receiver<Command>,
    skip: Option<String>,
) {
    let mut engine = build(
        &config.lock().unwrap_or_else(|e| e.into_inner()),
        skip.as_deref(),
    );
    // Told once the window had time to listen.
    let mut skip_notice = skip
        .as_ref()
        .map(|exe| (Instant::now() + Duration::from_secs(3), exe.clone()));
    let mut last_maintain: Option<Instant> = None;
    let mut last_emit = Instant::now();
    let mut last_visible_check = Instant::now();
    let mut visible = true;
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
                            engine = build(
                                &config.lock().unwrap_or_else(|e| e.into_inner()),
                                skip.as_deref(),
                            );
                            last_maintain = None;
                            notices = Notices::default();
                            quiet = true;
                        }
                        Command::ApplyGains => {
                            if let Some(e) = &engine {
                                apply_gains(e, &config.lock().unwrap_or_else(|e| e.into_inner()));
                            }
                        }
                        Command::SetFx(i, fx) => {
                            let mut c = config.lock().unwrap_or_else(|e| e.into_inner());
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
                        Command::SetMasterGain(o, db) => {
                            let mut c = config.lock().unwrap_or_else(|e| e.into_inner());
                            if let Some(out) = c.outputs.get_mut(o) {
                                out.master.db = clamp_db(db);
                                if let Some(e) = &engine {
                                    e.matrix.set_master_db(o, out.master.db);
                                }
                                dirty_since.get_or_insert_with(Instant::now);
                            }
                        }
                        Command::SetMasterMuted(o, muted) => {
                            let mut c = config.lock().unwrap_or_else(|e| e.into_inner());
                            if let Some(out) = c.outputs.get_mut(o) {
                                out.master.muted = muted;
                                if let Some(e) = &engine {
                                    e.matrix.set_master_muted(o, muted);
                                }
                                dirty_since.get_or_insert_with(Instant::now);
                            }
                        }
                        Command::SetInputMuted(i, muted) => {
                            let mut c = config.lock().unwrap_or_else(|e| e.into_inner());
                            if let Some(input) = c.inputs.get_mut(i) {
                                input.muted = muted;
                                if let Some(e) = &engine {
                                    e.matrix.set_input_muted(i, muted);
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
            let mut events = e.wake();
            let maintained = last_maintain.is_none_or(|t| t.elapsed() >= MAINTAIN_EVERY);
            if maintained {
                events.extend(e.maintain());
                last_maintain = Some(Instant::now());
            }
            if !events.is_empty() {
                let c = config.lock().unwrap_or_else(|e| e.into_inner());
                for event in &events {
                    let notice = notices.notice(&c, event, quiet);
                    // A failure repeats every second; log it once.
                    let repeat =
                        matches!(event, EngineEvent::ConnectFailed { .. }) && notice.is_none();
                    if !repeat {
                        log::write(&event.to_string());
                    }
                    if let Some(n) = notice {
                        let _ = app.emit("notice", n);
                    }
                }
            }
            if maintained {
                quiet = false;
            }
        }

        if let Some((at, exe)) = &skip_notice {
            if Instant::now() >= *at {
                let _ = app.emit(
                    "notice",
                    Notice {
                        level: "error",
                        text: tr(
                            "Die Aufnahme von {exe} hat Faderust beim letzten Mal zum Absturz gebracht. Sie bleibt bis zum nächsten Start aus. Details stehen in faderust.log.",
                            "Recording {exe} crashed Faderust last time. It stays off until the next start. Details are in faderust.log.",
                        )
                        .replace("{exe}", exe),
                    },
                );
                skip_notice = None;
            }
        }

        if last_visible_check.elapsed() >= VISIBLE_EVERY {
            visible = window_visible(&app);
            last_visible_check = Instant::now();
        }
        // Meters for a hidden window are wasted work, in Faderust and in
        // the web view drawing them.
        if visible && last_emit.elapsed() >= EMIT_EVERY {
            let state = snapshot(
                &config.lock().unwrap_or_else(|e| e.into_inner()),
                engine.as_ref(),
            );
            if let Some(shared) = app.try_state::<crate::AppState>() {
                *shared.last_state.lock().unwrap_or_else(|e| e.into_inner()) = Some(state.clone());
            }
            let _ = app.emit("state", state);
            last_emit = Instant::now();
        }

        if dirty_since.is_some_and(|t| t.elapsed() >= SAVE_EVERY) {
            if let Err(e) = config.lock().unwrap_or_else(|e| e.into_inner()).save(&path) {
                log::write(&format!("saving {}: {e}", path.display()));
            }
            dirty_since = None;
        }
    }
}

/// Whether the main window is on screen (not in the tray, not minimised).
fn window_visible(app: &AppHandle) -> bool {
    app.get_webview_window("main")
        .is_none_or(|w| w.is_visible().unwrap_or(true) && !w.is_minimized().unwrap_or(false))
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
    let mut c = config.lock().unwrap_or_else(|e| e.into_inner());
    let g = c.gains.get_mut(i)?.get_mut(o)?;
    f(g);
    g.db = clamp_db(g.db);
    Some(*g)
}

fn clamp_db(db: f32) -> f32 {
    if db.is_finite() {
        db.clamp(-120.0, 12.0)
    } else {
        -120.0
    }
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
        released: false,
        fx: None,
        master: None,
        muted: None,
    }
}

fn snapshot(config: &Config, engine: Option<&Engine<CpalBackend>>) -> UiState {
    let inputs = config
        .inputs
        .iter()
        .enumerate()
        .map(|(i, c)| UiDevice {
            fx: Some(c.fx),
            muted: Some(c.muted),
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
        .map(|(o, c)| UiDevice {
            released: engine.is_some_and(|e| e.is_released(o)),
            master: Some(c.master),
            ..device(
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
                        dropped_frames: s.dropped_frames.load(Relaxed),
                        looped: e.is_loop(i, o),
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
    use super::{explain, skipped, step_db};

    #[test]
    fn explains_a_device_held_by_another_program() {
        let error = "Failed to initialize audio client: OS Error -2004287478 (FormatMessageW() returned error 317) (os error -2004287478)";
        assert!(explain(error).contains("Headset-Mikro"));
        assert!(explain("something else").starts_with("something else."));
    }

    #[test]
    fn skips_only_the_crashed_program() {
        assert!(skipped("app:spotify.exe", Some("Spotify.exe")));
        assert!(!skipped("app:discord.exe", Some("spotify.exe")));
        assert!(!skipped("spotify.exe", Some("spotify.exe")));
        assert!(!skipped("app:spotify.exe", None));
    }

    #[test]
    fn steps_stay_in_fader_range() {
        assert_eq!(step_db(0.0, 1.0), 1.0);
        assert_eq!(step_db(11.5, 1.0), 12.0);
        assert_eq!(step_db(-59.0, -2.0), -120.0);
        assert_eq!(step_db(-120.0, 1.0), -59.0);
        assert_eq!(step_db(-120.0, -1.0), -120.0);
    }
}
