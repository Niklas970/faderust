// No console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Faderust window built with iced: title bar, desk
//! (strips for inputs and output masters with A1-An), matrix and the
//! dialogs, driven by `faderust-core`.
//!
//! Everything runs against the engine: faders, mute, routing, effects,
//! presets, programs, Windows sound, devices, tray, autostart, updates.
//!
//! `FADERUST_SHOT=out.png` saves one screenshot after a moment and exits;
//! `FADERUST_VIEW=matrix` and `FADERUST_DIALOG=<name>` pick what it shows.

mod dialogs;
mod fader;
mod files;
mod i18n;
mod icons;
mod picker;
mod scale;
mod shell;
mod style;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use faderust_core::audio::{Command, Notice, UiState};
use faderust_core::config::{Config as CoreConfig, InputEntry};
use faderust_core::config::{Fx as CoreFx, Gain};
use faderust_core::selftest::Check;
use faderust_core::update::Update;
use faderust_core::{cable, AppState, Apps, Devices, Host, StreamDeckStatus, WindowsAudio};

use i18n::text;
use iced::alignment::{Horizontal, Vertical};
use iced::gradient;
use iced::widget::canvas::{self, Canvas};
use iced::widget::{button, column, container, image, row, space, stack};
use iced::{
    window, Background, Border, Color, Element, Font, Length, Shadow, Size, Subscription, Task,
    Theme,
};

use fader::{Fader, Kind};
use style::*;
use style::{key, Look};

const LOGO: &[u8] = include_bytes!("../assets/logo.png");

pub const RELEASE: &[(u32, &str)] = &[
    (0, "Nie frei"),
    (5, "Frei nach 5 s"),
    (30, "Frei nach 30 s"),
    (120, "Frei nach 2 min"),
];
pub const HIDE_AFTER: &[&str] = &["Nie", "Nach 5 s", "Nach 2 min"];

/// Row before a strip move and the `(output, dB)` pairs the move applied.
type Anchor = (Vec<Gain>, Vec<(usize, f32)>);

pub struct Strip {
    pub cache: canvas::Cache,
    pub label: String,
    pub db: f32,
    pub muted: bool,
    /// Meter as drawn (eased towards `target`).
    pub shown: f32,
    /// Fader position as drawn (0..1, eased towards the real one).
    pub pos: f32,
    /// Speed of the fader glide (the spring's velocity).
    vel: f32,
    target: f32,
    pub connected: bool,
    /// Input device id (`app:name.exe` for programs).
    device: String,
    pub routes: Vec<bool>,
    /// Programs are recorded from one program, devices from a capture device.
    pub program: bool,
    /// A silent program the desk hides until it is shown.
    pub hidden: bool,
    /// Kept on the desk even when silent (the eye button).
    pub keep: bool,
    /// Shown on request ("N still, zeigen") until the window closes.
    revealed: bool,
    /// Hidden by hand (the × button); comes back when sound starts again.
    dismissed: bool,
    /// Last time sound was heard from this program.
    heard: Instant,
    /// `gains` row as the engine reports it.
    row: Vec<Gain>,
    /// Row before the strip fader started moving, see [`App::move_strip`].
    anchor: Option<Anchor>,
}

pub struct Out {
    pub cache: canvas::Cache,
    pub label: String,
    /// Device state under the name: where it plays, or "Freigegeben".
    pub note: String,
    pub note_on: bool,
    pub hidden: bool,
    pub db: f32,
    pub muted: bool,
    pub shown: f32,
    /// Fader position as drawn (0..1, eased towards the real one).
    pub pos: f32,
    /// Speed of the fader glide (the spring's velocity).
    vel: f32,
    target: f32,
    /// Was playing at the last snapshot (a hidden output returns with sound).
    playing: bool,
}

#[derive(Clone, PartialEq)]
pub struct Fx {
    pub denoise: bool,
    pub low_cut: u16,
    pub eq: [f32; 3],
    pub gate: bool,
    pub gate_db: f32,
}

impl From<CoreFx> for Fx {
    fn from(f: CoreFx) -> Self {
        Fx {
            denoise: f.denoise,
            low_cut: f.low_cut_hz.round() as u16,
            eq: [f.low_db, f.mid_db, f.high_db],
            gate: f.gate,
            gate_db: f.gate_db,
        }
    }
}

impl From<&Fx> for CoreFx {
    fn from(f: &Fx) -> Self {
        CoreFx {
            denoise: f.denoise,
            low_cut_hz: f.low_cut as f32,
            low_db: f.eq[0],
            mid_db: f.eq[1],
            high_db: f.eq[2],
            gate: f.gate,
            gate_db: f.gate_db,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Desk,
    Matrix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    Presets,
    Windows,
    Settings,
    Design,
    Apps,
    Log,
    Selftest,
    Fx(usize),
}

pub struct App {
    pub view: View,
    pub masters_left: bool,
    pub compact: bool,
    pub pinned: bool,
    pub hide_after: &'static str,
    pub dialog: Option<Dialog>,
    visible: shell::Visible,
    win_id: Option<window::Id>,
    dir: PathBuf,
    prefs: shell::Prefs,
    pub autostart: bool,
    /// Recording device that programs pick after "+ Programm-Mikro".
    pub mic_device: Option<String>,
    pub mic_default: bool,
    /// What is typed in the colour fields of the design dialog.
    pub accent_input: Option<String>,
    pub picker: Option<(Target, (f32, f32, f32))>,
    pub signal_input: Option<String>,
    logo: image::Handle,
    #[cfg(windows)]
    tray: Option<shell::tray::Tray>,
    update: Option<Update>,
    updating: bool,
    checking: bool,
    last_check: Option<Instant>,
    core: Arc<AppState>,
    notices: Arc<Mutex<Vec<Notice>>>,
    /// Message for the user: when it came, text, level.
    pub notice: Option<(Instant, String, &'static str)>,
    pub strips: Vec<Strip>,
    pub outs: Vec<Out>,
    pub fx: Vec<Fx>,
    pub presets: Vec<String>,
    pub new_preset: String,
    /// Windows sound dialog.
    pub win: Option<WindowsAudio>,
    /// Programs dialog.
    pub apps_data: Option<Apps>,
    /// Devices dialog: devices on offer, the copy being edited, USB cable.
    pub devs: Devices,
    pub draft: Option<CoreConfig>,
    edit_at: Option<Instant>,
    reshaped: bool,
    pub usb: Option<cable::Status>,
    pub usb_busy: bool,
    last_usb: Instant,
    /// Inputs of the configuration, for "is this program an input".
    pub input_devices: Vec<String>,
    /// Error shown inside the open dialog.
    pub dialog_error: String,
    pub log_text: Option<String>,
    pub checks: Option<Vec<Check>>,
    pub deck: Option<StreamDeckStatus>,
    pub deck_error: String,
    /// Background jobs started by the current message.
    jobs: Vec<Task<Message>>,
    last: Instant,
    started: Instant,
    ready: bool,
    shot: Option<String>,
}

/// Applies the saved colours and language.
fn apply_look(p: &shell::Prefs) {
    let accent = if p.accent.is_empty() {
        style::DEFAULT_ACCENT
    } else {
        &p.accent
    };
    let signal = if p.signal.is_empty() {
        style::DEFAULT_SIGNAL
    } else {
        &p.signal
    };
    style::set_theme(&p.mode, accent, signal);
    let lang = if p.lang.is_empty() {
        shell::system_lang()
    } else {
        &p.lang
    };
    faderust_core::lang::set(lang);
}

/// Which colour the picker edits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    Accent,
    Signal,
}

#[derive(Debug, Clone)]
pub enum Edit {
    /// Opens or closes the colour picker of a colour.
    Picker(Target),
    /// The picker moved: hue, saturation, brightness.
    Hsv(Target, f32, f32, f32),
    /// The picker was let go: save the colour.
    HsvDone,
    Denoise(usize, bool),
    LowCut(usize, u16),
    Eq(usize, usize, f32),
    Gate(usize, bool),
    GateDb(usize, f32),
    ResetFx(usize),
    Refresh,
    AppDevice(Vec<u32>, Option<String>),
    AppInput(String, String),
    WinDefault(String, bool),
    WinVolume(String, f32),
    WinMute(String, bool),
    InLabel(usize, String),
    InDevice(usize, String),
    InAdd,
    InRemove(usize),
    InMove(usize, isize),
    OutLabel(usize, String),
    OutDevice(usize, String),
    OutBuffer(usize, String),
    OutRelease(usize, u32),
    OutAdd,
    MicAdd,
    /// Park the program on a quiet cable, then record it: (program, label, pids, cable id).
    AppInputParked(String, String, Vec<u32>, String),
    MicDefault(bool),
    OutRemove(usize),
    OutMove(usize, isize),
    UsbSetup,
    UsbEnable(bool),
    UsbCount(u8),
    UsbRemove,
    Autostart(bool),
    OpenLogFolder,
    CopyLog,
    CopyChecks,
    InstallDeck,
    MastersLeft(bool),
    Mode(&'static str),
    AccentHex(String),
    SignalHex(String),
    Accent(String),
    Signal(String),
    Lang(&'static str),
    HideAfter(&'static str),
    NewPreset(String),
    SavePreset,
    LoadPreset(String),
    ExportPreset(String),
    ImportPreset,
    DeletePreset(String),
    Keep(usize),
    HideOut(usize, bool),
    ShowSilent,
    /// Hide a program by hand.
    Dismiss(usize),
    /// Show one hidden program.
    ShowOne(usize),
}

#[derive(Debug, Clone)]
pub enum Message {
    SetView(View),
    WinId(Option<window::Id>),
    ToggleCompact,
    DismissNotice,
    TogglePin,
    CloseRequested,
    Minimize,
    ToggleMax,
    Drag,
    /// Start resizing the window from an edge or corner.
    Resize(window::Direction),
    Quit,
    CheckUpdate,
    UpdateChecked(Result<Option<Update>, String>, bool),
    InstallUpdate,
    UpdateInstalled(Result<(), String>),
    Open(Option<Dialog>),
    Edit(Edit),
    Route(usize, usize),
    Level(usize, f32),
    /// One input on one output: (input, output, slider position 0..1).
    Cell(usize, usize, f32),
    Master(usize, f32),
    Mute(usize),
    MasterMute(usize),
    Frame(Instant),
    Shot(window::Screenshot),
    Id(Option<window::Id>),
    Loaded(Loaded),
    Failed(String),
}

/// What a background job brings back.
#[derive(Debug, Clone)]
pub enum Loaded {
    Windows(Result<WindowsAudio, String>),
    Apps(Result<Apps, String>),
    Devices(Result<Devices, String>),
    Usb(cable::Status),
    Log(String),
    Selftest(Result<Vec<Check>, String>),
    Deck(StreamDeckStatus),
    Done(Then, Result<(), String>),
}

/// What to do after a command finished.
#[derive(Debug, Clone, Copy)]
pub enum Then {
    Nothing,
    Windows,
    Apps,
    Usb,
    Presets,
}

/// The audio thread tells the window through the shared snapshot, so only
/// notices need a queue.
struct GuiHost {
    notices: Arc<Mutex<Vec<Notice>>>,
    visible: shell::Visible,
}

impl Host for GuiHost {
    fn state(&self, _: UiState) {}
    fn notice(&self, notice: Notice) {
        self.notices
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(notice);
    }
    fn visible(&self) -> bool {
        shell::is_visible(&self.visible)
    }
}

/// The folder the Tauri app uses too, so the settings carry over.
fn config_dir() -> PathBuf {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    };
    base.unwrap_or_else(std::env::temp_dir)
        .join("io.github.niklas970.faderust")
}

/// A file shipped next to the program (`resources/`), when there is one.
fn resource(name: &str) -> Option<PathBuf> {
    let dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    [dir.join("resources").join(name), dir.join(name)]
        .into_iter()
        .find(|p| p.is_file())
}

fn bundled_plugin_version() -> Option<String> {
    resource(faderust_core::PLUGIN_VERSION_FILE)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Seconds of silence after which a program leaves the desk (0 = never).
fn hide_secs(label: &str) -> u64 {
    match label {
        "Nach 5 s" => 5,
        "Nach 2 min" => 120,
        _ => 0,
    }
}

/// Meter smoothing: fast up, slow down.
fn ease(shown: f32, target: f32, dt: f32) -> f32 {
    if target > shown {
        shown + (target - shown) * (1.0 - (-dt / 0.06).exp())
    } else {
        target.max(shown - 12.0 * dt)
    }
}

/// Glides the drawn fader position towards the real one with a critically
/// damped spring (soft start, soft stop); true while it moves.
fn glide(pos: &mut f32, vel: &mut f32, db: f32, dt: f32) -> bool {
    let want = scale::to_pos(db);
    if *pos == want {
        return false;
    }
    let change = *pos - want;
    if change.abs() < 0.0003 && vel.abs() < 0.01 {
        *pos = want;
        *vel = 0.0;
        return true;
    }
    // Smooth damping: reaches the target in about `SMOOTH` seconds.
    const SMOOTH: f32 = 0.08;
    let omega = 2.0 / SMOOTH;
    let x = omega * dt;
    let k = 1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x);
    let temp = (*vel + omega * change) * dt;
    *vel = (*vel - omega * temp) * k;
    *pos = want + (change + temp) * k;
    true
}

/// True while a fader is still gliding.
fn gliding(pos: f32, db: f32) -> bool {
    pos != scale::to_pos(db)
}

fn level_of(db: f32) -> f32 {
    if db.is_finite() {
        db
    } else {
        -120.0
    }
}

/// Runs `f` off the UI thread (the device calls can take a while and want
/// their own COM apartment) and turns its result into a message.
/// Case-insensitive "name contains pattern" for a configured device pattern.
pub fn contains(name: &str, pattern: &str) -> bool {
    !pattern.is_empty() && name.to_lowercase().contains(&pattern.to_lowercase())
}

fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> T + Send + 'static,
    then: impl Fn(T) -> Message + Send + 'static,
) -> Task<Message> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(f)
                .await
                .map_err(|e| e.to_string())
        },
        move |r| match r {
            Ok(v) => then(v),
            Err(e) => Message::Failed(e),
        },
    )
}

impl App {
    fn check_update(&mut self, manual: bool) -> Task<Message> {
        if self.checking || self.updating {
            return Task::none();
        }
        self.checking = true;
        self.last_check = Some(Instant::now());
        blocking(
            || faderust_core::update::check(env!("CARGO_PKG_VERSION")),
            move |r| Message::UpdateChecked(r, manual),
        )
    }

    /// A cable's playback end that is no output yet and whose recording end
    /// is no input (that would loop).
    pub fn mic_cable(&self) -> Option<String> {
        use faderust_core::names::{cable_counterpart, is_cable, own_cable};
        let draft = self.draft.as_ref()?;
        let matches = |name: &str, pattern: &str| {
            !pattern.is_empty() && name.to_lowercase().contains(&pattern.to_lowercase())
        };
        let recordings: Vec<String> = self.devs.inputs.iter().map(|d| d.name.clone()).collect();
        self.devs
            .outputs
            .iter()
            .find(|d| {
                is_cable(&d.name)
                    && (d.name.to_lowercase().contains("input") || own_cable(&d.name).is_some())
                    && !draft.outputs.iter().any(|o| matches(&d.name, &o.device))
                    && !draft
                        .inputs
                        .iter()
                        .any(|i| matches(&cable_counterpart(&d.name, &recordings), &i.device))
            })
            .map(|d| d.name.clone())
    }

    fn cable_inputs(&self) -> Vec<String> {
        self.devs
            .inputs
            .iter()
            .filter(|d| faderust_core::names::is_cable(&d.name))
            .map(|d| d.name.clone())
            .collect()
    }

    /// A cable nobody hears: Faderust neither records its other end nor
    /// plays on it. A program input parked there is heard only through its fader.
    pub fn quiet_cable(&self) -> Option<faderust_core::RenderDevice> {
        let data = self.apps_data.as_ref()?;
        let cfg = self.config();
        let cables = self.cable_inputs();
        data.devices
            .iter()
            .find(|d| self.is_quiet(d, &cfg, &cables))
            .cloned()
    }

    fn is_quiet(
        &self,
        d: &faderust_core::RenderDevice,
        cfg: &CoreConfig,
        cables: &[String],
    ) -> bool {
        let counterpart = faderust_core::names::cable_counterpart(&d.name, cables);
        d.cable
            && !cfg.inputs.iter().any(|i| contains(&counterpart, &i.device))
            && !cfg.outputs.iter().any(|o| contains(&d.name, &o.device))
    }

    /// Cable capture devices no input uses yet. Not offered when Faderust
    /// plays on the cable's other end (that would loop) or a program input
    /// is parked on it (heard twice otherwise).
    pub fn unused_cables(&self) -> Vec<String> {
        use faderust_core::names::own_cable;
        let Some(data) = &self.apps_data else {
            return vec![];
        };
        let cfg = self.config();
        let cables = self.cable_inputs();
        cables
            .iter()
            .filter(|c| {
                let playback = match own_cable(c) {
                    None => c.to_lowercase().replace("output", "input"),
                    Some(own) => data
                        .devices
                        .iter()
                        .find(|d| own_cable(&d.name).as_deref() == Some(own.as_str()))
                        .map(|d| d.name.to_lowercase())
                        .unwrap_or_default(),
                };
                !cfg.inputs.iter().any(|i| contains(c, &i.device))
                    && !cfg.outputs.iter().any(|o| contains(&playback, &o.device))
                    && !data.apps.iter().any(|a| {
                        let program = format!("app:{}", a.exe.to_lowercase());
                        cfg.inputs.iter().any(|i| i.device == program)
                            && a.device.as_ref().is_some_and(|id| {
                                data.devices.iter().find(|d| &d.id == id).is_some_and(|d| {
                                    &faderust_core::names::cable_counterpart(&d.name, &cables) == *c
                                })
                            })
                    })
            })
            .cloned()
            .collect()
    }

    /// The program is recorded as an input, reaches an output and still plays
    /// on a device you hear: a cable somebody listens to, a device Faderust
    /// plays on too, or the Windows default.
    pub fn doubled(&self, a: &faderust_core::App) -> bool {
        let Some(data) = &self.apps_data else {
            return false;
        };
        let cfg = self.config();
        let program = format!("app:{}", a.exe.to_lowercase());
        let Some(row) = cfg.inputs.iter().position(|i| i.device == program) else {
            return false;
        };
        let routed = !cfg.inputs[row].muted
            && cfg
                .gains
                .get(row)
                .is_some_and(|g| g.iter().any(|g| !g.muted && g.db > -59.0));
        if !routed {
            return false;
        }
        let default_id = self
            .win
            .as_ref()
            .and_then(|w| w.playback.iter().find(|e| e.is_default))
            .map(|e| e.id.clone());
        let wanted = a.device.clone().or(default_id);
        let Some(d) = data.devices.iter().find(|x| Some(&x.id) == wanted.as_ref()) else {
            return false;
        };
        if d.cable {
            !self.is_quiet(d, &cfg, &self.cable_inputs())
        } else {
            a.device.is_none() || cfg.outputs.iter().any(|o| contains(&d.name, &o.device))
        }
    }

    pub fn has_cable(&self) -> bool {
        self.devs
            .outputs
            .iter()
            .any(|d| faderust_core::names::is_cable(&d.name))
    }

    pub fn accent(&self) -> String {
        if self.prefs.accent.is_empty() {
            style::DEFAULT_ACCENT.into()
        } else {
            self.prefs.accent.clone()
        }
    }

    pub fn signal(&self) -> String {
        if self.prefs.signal.is_empty() {
            style::DEFAULT_SIGNAL.into()
        } else {
            self.prefs.signal.clone()
        }
    }

    pub fn mode(&self) -> &str {
        match self.prefs.mode.as_str() {
            "light" => "light",
            "system" => "system",
            _ => "dark",
        }
    }

    pub fn lang(&self) -> &str {
        match self.prefs.lang.as_str() {
            "en" => "en",
            "de" => "de",
            _ if faderust_core::lang::is_english() => "en",
            _ => "de",
        }
    }

    /// Changes colours or language: saves, applies and redraws everything.
    fn set_look(&mut self, change: impl FnOnce(&mut shell::Prefs)) {
        let mut p = self.prefs.clone();
        change(&mut p);
        apply_look(&p);
        shell::save_prefs(&self.dir, &p);
        self.prefs = p;
        self.clear_caches();
    }

    fn clear_caches(&self) {
        for s in &self.strips {
            s.cache.clear();
        }
        for o in &self.outs {
            o.cache.clear();
        }
    }

    fn save_prefs(&mut self) {
        let prefs = shell::Prefs {
            view: if self.view == View::Matrix {
                "matrix"
            } else {
                "desk"
            }
            .into(),
            masters_right: !self.masters_left,
            compact: self.compact,
            pinned: self.pinned,
            hide_after: Some(hide_secs(self.hide_after)),
            keep: self
                .strips
                .iter()
                .filter(|s| s.keep)
                .map(|s| s.device.clone())
                .collect(),
            hidden_outputs: self
                .outs
                .iter()
                .filter(|o| o.hidden)
                .map(|o| o.label.clone())
                .collect(),
            ..self.prefs.clone()
        };
        if prefs != self.prefs {
            shell::save_prefs(&self.dir, &prefs);
            self.prefs = prefs;
        }
    }

    /// Closing the window keeps the mixer running in the tray.
    fn hide(&mut self) -> Task<Message> {
        self.visible.store(false, Ordering::Relaxed);
        match self.win_id {
            Some(id) => window::set_mode(id, window::Mode::Hidden),
            None => Task::none(),
        }
    }

    fn show(&mut self) -> Task<Message> {
        self.visible.store(true, Ordering::Relaxed);
        match self.win_id {
            Some(id) => Task::batch([
                window::set_mode(id, window::Mode::Windowed),
                window::minimize(id, false),
                window::gain_focus(id),
            ]),
            None => Task::none(),
        }
    }

    fn load_windows(&self) -> Task<Message> {
        blocking(faderust_core::get_windows_audio, |r| {
            Message::Loaded(Loaded::Windows(r))
        })
    }

    fn load_apps(&self) -> Task<Message> {
        blocking(faderust_core::get_apps, |r| {
            Message::Loaded(Loaded::Apps(r))
        })
    }

    fn load_devices(&self) -> Task<Message> {
        blocking(faderust_core::get_devices, |r| {
            Message::Loaded(Loaded::Devices(r))
        })
    }

    fn load_usb(&self) -> Task<Message> {
        let core = Arc::clone(&self.core);
        blocking(
            move || faderust_core::get_usb_cable(&core),
            |s| Message::Loaded(Loaded::Usb(s)),
        )
    }

    fn load_log(&self) -> Task<Message> {
        blocking(
            || faderust_core::log::tail(400),
            |t| Message::Loaded(Loaded::Log(t)),
        )
    }

    fn load_selftest(&self) -> Task<Message> {
        let core = Arc::clone(&self.core);
        blocking(
            move || faderust_core::run_selftest(&core, bundled_plugin_version()),
            |r| Message::Loaded(Loaded::Selftest(r)),
        )
    }

    fn load_deck(&self) -> Task<Message> {
        let core = Arc::clone(&self.core);
        blocking(
            move || {
                faderust_core::streamdeck_status(
                    &core,
                    resource(faderust_core::PLUGIN_VERSION_FILE),
                )
            },
            |s| Message::Loaded(Loaded::Deck(s)),
        )
    }

    fn refresh_dialog(&mut self) -> Task<Message> {
        self.dialog_error.clear();
        match self.dialog {
            Some(Dialog::Windows) => self.load_windows(),
            Some(Dialog::Apps) => {
                self.input_devices = self.config().inputs.into_iter().map(|i| i.device).collect();
                Task::batch([self.load_apps(), self.load_windows(), self.load_devices()])
            }
            Some(Dialog::Settings) => {
                Task::batch([self.load_devices(), self.load_usb(), self.load_deck()])
            }
            Some(Dialog::Log) => self.load_log(),
            Some(Dialog::Selftest) => {
                self.checks = None;
                self.load_selftest()
            }
            _ => Task::none(),
        }
    }

    fn config(&self) -> CoreConfig {
        self.core
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn endpoint_mut(&mut self, id: &str) -> Option<&mut faderust_core::Endpoint> {
        let w = self.win.as_mut()?;
        w.playback
            .iter_mut()
            .chain(w.recording.iter_mut())
            .find(|e| e.id == id)
    }

    /// A virtual cable's capture side or a program as a new input, at 0 dB
    /// on every output.
    fn add_input(&mut self, device: String, label: String) -> Result<(), String> {
        let mut next = self.config();
        next.inputs.push(InputEntry {
            label,
            device,
            fx: Default::default(),
            muted: false,
        });
        next.gains
            .push(next.outputs.iter().map(|_| Gain::default()).collect());
        faderust_core::save_config(next, &self.core)?;
        self.input_devices = self.config().inputs.into_iter().map(|i| i.device).collect();
        Ok(())
    }

    /// Edits the copy of the configuration the devices dialog works on; it
    /// is saved a moment after the last change.
    fn draft_edit(&mut self, f: impl FnOnce(&mut CoreConfig)) {
        if let Some(d) = &mut self.draft {
            f(d);
            self.edit_at = Some(Instant::now());
        }
    }

    /// Saves the draft. Unless the lists were reshaped, the faders and
    /// presets of the running configuration are newer than the copy.
    fn flush_draft(&mut self) {
        let Some(mut next) = self.draft.clone() else {
            return;
        };
        self.edit_at = None;
        if !self.reshaped {
            let fresh = self.config();
            if fresh.inputs.len() == next.inputs.len() && fresh.outputs.len() == next.outputs.len()
            {
                next.gains = fresh.gains;
                next.presets = fresh.presets;
            }
        }
        match faderust_core::save_config(next, &self.core) {
            Ok(()) => self.reshaped = false,
            Err(e) => self.dialog_error = e,
        }
    }

    fn open(&mut self, d: Option<Dialog>) -> Task<Message> {
        if self.dialog == Some(Dialog::Settings) && d != Some(Dialog::Settings) {
            self.flush_draft();
            self.draft = None;
        }
        self.dialog = d;
        self.dialog_error.clear();
        if d == Some(Dialog::Settings) {
            self.draft = Some(self.config());
            self.reshaped = false;
        }
        self.refresh_dialog()
    }

    fn loaded(&mut self, l: Loaded) {
        match l {
            Loaded::Windows(Ok(w)) => self.win = Some(w),
            Loaded::Apps(Ok(a)) => self.apps_data = Some(a),
            Loaded::Devices(Ok(d)) => self.devs = d,
            Loaded::Windows(Err(e)) | Loaded::Apps(Err(e)) | Loaded::Devices(Err(e)) => {
                self.dialog_error = e
            }
            Loaded::Usb(s) => self.usb = Some(s),
            Loaded::Log(t) => self.log_text = Some(t),
            Loaded::Selftest(Ok(c)) => self.checks = Some(c),
            Loaded::Selftest(Err(e)) => self.dialog_error = e,
            Loaded::Deck(d) => self.deck = Some(d),
            Loaded::Done(then, r) => {
                self.usb_busy = false;
                match r {
                    Ok(()) => self.dialog_error.clear(),
                    Err(e) => self.dialog_error = e,
                }
                let t = match then {
                    Then::Nothing => Task::none(),
                    Then::Windows => self.load_windows(),
                    Then::Apps => self.load_apps(),
                    Then::Usb => self.load_usb(),
                    Then::Presets => {
                        self.refresh_presets();
                        Task::none()
                    }
                };
                self.jobs.push(t);
            }
        }
    }
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let notices = Arc::new(Mutex::new(Vec::new()));
        let minimized = std::env::args().any(|a| a == shell::MINIMIZED_ARG);
        let visible: shell::Visible = Arc::new(AtomicBool::new(!minimized));
        let dir = config_dir();
        let core = faderust_core::start(
            Arc::new(GuiHost {
                notices: Arc::clone(&notices),
                visible: Arc::clone(&visible),
            }),
            &dir,
        );
        let prefs = shell::load_prefs(&dir);
        apply_look(&prefs);
        let dialog = std::env::var("FADERUST_DIALOG")
            .ok()
            .and_then(|d| match d.as_str() {
                "presets" => Some(Dialog::Presets),
                "windows" => Some(Dialog::Windows),
                "settings" => Some(Dialog::Settings),
                "design" => Some(Dialog::Design),
                "apps" => Some(Dialog::Apps),
                "fx" => Some(Dialog::Fx(0)),
                "log" => Some(Dialog::Log),
                "selftest" => Some(Dialog::Selftest),
                _ => None,
            });
        let mut app = App {
            view: if std::env::var("FADERUST_VIEW").as_deref() == Ok("matrix")
                || prefs.view == "matrix"
            {
                View::Matrix
            } else {
                View::Desk
            },
            masters_left: !prefs.masters_right,
            compact: prefs.compact,
            pinned: prefs.pinned,
            hide_after: match prefs.hide_after {
                Some(0) => HIDE_AFTER[0],
                Some(5) => HIDE_AFTER[1],
                _ => HIDE_AFTER[2],
            },
            visible,
            win_id: None,
            dir,
            autostart: shell::autostart_enabled(),
            mic_device: None,
            mic_default: true,
            accent_input: None,
            picker: None,
            signal_input: None,
            logo: image::Handle::from_bytes(LOGO),
            #[cfg(windows)]
            tray: tray_icon(),
            update: None,
            updating: false,
            checking: false,
            last_check: None,
            prefs,
            dialog: None,
            core,
            notices,
            notice: None,
            strips: vec![],
            outs: vec![],
            fx: vec![],
            presets: vec![],
            new_preset: String::new(),
            win: None,
            apps_data: None,
            devs: Devices {
                inputs: vec![],
                outputs: vec![],
            },
            draft: None,
            edit_at: None,
            reshaped: false,
            usb: None,
            usb_busy: false,
            last_usb: Instant::now(),
            input_devices: vec![],
            dialog_error: String::new(),
            log_text: None,
            checks: None,
            deck: None,
            deck_error: String::new(),
            jobs: vec![],
            last: Instant::now(),
            started: Instant::now(),
            ready: false,
            shot: std::env::var("FADERUST_SHOT").ok(),
        };
        app.refresh_presets();
        let task = Task::batch([app.open(dialog), window::latest().map(Message::WinId)]);
        (app, task)
    }

    fn send(&mut self, cmd: Command) {
        if let Err(e) = faderust_core::send(&self.core, cmd) {
            self.say("error", e);
        }
    }

    fn say(&mut self, level: &'static str, text: String) {
        self.notice = Some((Instant::now(), text, level));
    }

    fn refresh_presets(&mut self) {
        self.presets = self
            .core
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .presets
            .iter()
            .map(|p| p.name.clone())
            .collect();
    }

    /// Takes the engine's latest snapshot into the strips and outputs.
    fn apply(&mut self, snap: &UiState, dt: f32) {
        let now = Instant::now();
        let secs = hide_secs(self.hide_after);
        let startup = now.duration_since(self.started) < Duration::from_secs(4);
        while self.strips.len() < snap.inputs.len() {
            self.strips.push(Strip {
                cache: canvas::Cache::new(),
                label: String::new(),
                db: 0.0,
                muted: false,
                shown: -120.0,
                pos: 0.75,
                vel: 0.0,
                target: -120.0,
                connected: false,
                device: String::new(),
                routes: vec![],
                program: false,
                hidden: false,
                keep: false,
                revealed: false,
                dismissed: false,
                // Not heard yet counts as heard a moment ago only, so silent
                // programs are gone a few seconds after the start.
                heard: now - Duration::from_secs(secs.saturating_sub(if startup { 0 } else { 5 })),
                row: vec![],
                anchor: None,
            });
            self.fx.push(CoreFx::default().into());
        }
        self.strips.truncate(snap.inputs.len());
        self.fx.truncate(snap.inputs.len());
        for (i, (s, d)) in self.strips.iter_mut().zip(&snap.inputs).enumerate() {
            let row: Vec<Gain> = (0..snap.outputs.len())
                .map(|o| {
                    snap.gains
                        .get(i)
                        .and_then(|r| r.get(o))
                        .copied()
                        .unwrap_or(Gain {
                            db: 0.0,
                            muted: true,
                        })
                })
                .collect();
            let db = scale::strip_value(&row);
            let muted = d.muted.unwrap_or(false);
            if s.label != d.label || s.db != db || s.muted != muted || s.connected != d.connected {
                s.cache.clear();
            }
            s.label.clone_from(&d.label);
            s.db = db;
            s.muted = muted;
            s.connected = d.connected;
            s.routes = row.iter().map(|g| !g.muted).collect();
            s.row = row;
            s.program = d.device.starts_with("app:");
            if s.device != d.device {
                s.device.clone_from(&d.device);
                s.keep = self.prefs.keep.contains(&d.device);
            }
            s.target = level_of(d.level_db);
            if s.connected && s.target > -60.0 {
                s.heard = now;
            }
            if s.connected && s.target > -60.0 {
                s.dismissed = false;
            }
            s.hidden = s.program
                && (s.dismissed
                    || (secs > 0
                        && !s.keep
                        && !s.revealed
                        && now.duration_since(s.heard) >= Duration::from_secs(secs)));
            if self.dialog != Some(Dialog::Fx(i)) {
                if let Some(fx) = d.fx {
                    self.fx[i] = fx.into();
                }
            }
            s.shown = ease(s.shown, s.target, dt);
            if glide(&mut s.pos, &mut s.vel, s.db, dt) {
                s.cache.clear();
            }
        }
        while self.outs.len() < snap.outputs.len() {
            self.outs.push(Out {
                cache: canvas::Cache::new(),
                label: String::new(),
                note: String::new(),
                note_on: false,
                hidden: false,
                db: 0.0,
                muted: false,
                shown: -120.0,
                pos: 0.75,
                vel: 0.0,
                target: -120.0,
                playing: false,
            });
        }
        self.outs.truncate(snap.outputs.len());
        for (m, d) in self.outs.iter_mut().zip(&snap.outputs) {
            if m.label.is_empty() {
                m.hidden = self.prefs.hidden_outputs.contains(&d.label);
            }
            let master = d.master.unwrap_or_default();
            let note = if d.released {
                "Freigegeben".to_string()
            } else {
                d.device_name.clone().unwrap_or_else(|| d.device.clone())
            };
            if m.label != d.label || m.db != master.db || m.muted != master.muted || m.note != note
            {
                m.cache.clear();
            }
            m.label.clone_from(&d.label);
            m.db = master.db;
            m.muted = master.muted;
            m.note = note;
            m.note_on = d.connected && !d.released;
            m.target = level_of(d.level_db);
            // A hidden output comes back when sound starts on it again.
            let playing = m.target > -60.0;
            if m.hidden && playing && !m.playing {
                m.hidden = false;
            }
            m.playing = playing;
            m.shown = ease(m.shown, m.target, dt);
            if glide(&mut m.pos, &mut m.vel, m.db, dt) {
                m.cache.clear();
            }
        }
    }

    /// Moving a strip shifts all its outputs by the same amount. The levels
    /// from before the move are kept while only the strip fader moves them,
    /// so pulling it down and up again restores the differences between
    /// outputs.
    fn move_strip(&mut self, i: usize, db: f32) {
        let Some(s) = self.strips.get_mut(i) else {
            return;
        };
        let now = s.row.clone();
        let untouched = s.anchor.as_ref().is_some_and(|(base, applied)| {
            applied.len() == now.len()
                && base.iter().zip(&now).all(|(b, n)| b.muted == n.muted)
                && applied.iter().all(|&(o, v)| now[o].db == v)
        });
        let base = match (&s.anchor, untouched) {
            (Some((base, _)), true) => base.clone(),
            _ => now,
        };
        let changes = scale::strip_changes(&base, db);
        for &(o, v) in &changes {
            s.row[o].db = v;
        }
        s.db = db;
        s.cache.clear();
        s.anchor = Some((base, changes.clone()));
        for (o, v) in changes {
            self.send(Command::SetGain(i, o, v));
        }
    }

    /// One cell of the matrix: an input's level on a single output.
    fn set_cell(&mut self, i: usize, o: usize, db: f32) {
        let Some(s) = self.strips.get_mut(i) else {
            return;
        };
        let Some(g) = s.row.get_mut(o) else {
            return;
        };
        if g.db == db {
            return;
        }
        g.db = db;
        s.db = scale::strip_value(&s.row);
        s.anchor = None;
        s.cache.clear();
        self.send(Command::SetGain(i, o, db));
    }

    /// Switching an output on that sits at -inf brings it to the strip's level.
    fn toggle_route(&mut self, i: usize, o: usize) {
        let Some(g) = self.strips.get(i).and_then(|s| s.row.get(o)).copied() else {
            return;
        };
        if !g.muted {
            self.strips[i].row[o].muted = true;
            return self.send(Command::SetMuted(i, o, true));
        }
        if g.db <= scale::OFF {
            let level = scale::strip_value(&self.strips[i].row);
            let db = if level > scale::OFF { level } else { 0.0 };
            self.strips[i].row[o].db = db;
            self.send(Command::SetGain(i, o, db));
        }
        self.strips[i].row[o].muted = false;
        self.send(Command::SetMuted(i, o, false));
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        let direct = self.handle(message);
        let mut jobs = std::mem::take(&mut self.jobs);
        jobs.push(direct);
        Task::batch(jobs)
    }

    fn handle(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SetView(v) => {
                self.view = v;
                self.save_prefs();
            }
            Message::WinId(id) => {
                self.win_id = id;
                if let (true, Some(id)) = (self.pinned, id) {
                    return window::set_level(id, window::Level::AlwaysOnTop);
                }
            }
            Message::DismissNotice => self.notice = None,
            Message::ToggleCompact => {
                self.compact = !self.compact;
                self.save_prefs();
                for s in &self.strips {
                    s.cache.clear();
                }
                for o in &self.outs {
                    o.cache.clear();
                }
            }
            Message::TogglePin => {
                self.pinned = !self.pinned;
                self.save_prefs();
                if let Some(id) = self.win_id {
                    return window::set_level(
                        id,
                        if self.pinned {
                            window::Level::AlwaysOnTop
                        } else {
                            window::Level::Normal
                        },
                    );
                }
            }
            Message::CloseRequested => return self.hide(),
            Message::Minimize => {
                if let Some(id) = self.win_id {
                    return window::minimize(id, true);
                }
            }
            Message::ToggleMax => {
                if let Some(id) = self.win_id {
                    return window::toggle_maximize(id);
                }
            }
            Message::Resize(direction) => {
                if let Some(id) = self.win_id {
                    return window::drag_resize(id, direction);
                }
            }
            Message::Drag => {
                if let Some(id) = self.win_id {
                    return window::drag(id);
                }
            }
            Message::CheckUpdate => return self.check_update(true),
            Message::UpdateChecked(r, manual) => {
                self.checking = false;
                match r {
                    Ok(u) => {
                        if manual && u.is_none() {
                            self.say("info", "Faderust ist aktuell".into());
                        }
                        self.update = u.or(self.update.take());
                    }
                    Err(e) if manual => {
                        self.say("warn", format!("Update-Suche fehlgeschlagen: {e}"))
                    }
                    Err(_) => {}
                }
            }
            Message::InstallUpdate => {
                if let (Some(u), false) = (self.update.clone(), self.updating) {
                    self.updating = true;
                    return blocking(
                        move || faderust_core::update::install(&u),
                        Message::UpdateInstalled,
                    );
                }
            }
            Message::UpdateInstalled(r) => {
                self.updating = false;
                match r {
                    Ok(()) => return self.handle(Message::Quit),
                    Err(e) => self.say("error", format!("Update fehlgeschlagen: {e}")),
                }
            }
            Message::Quit => {
                faderust_core::log::write("Faderust beendet");
                // A stream that never closes must not keep the process, and
                // with it the audio devices, alive after Quit.
                let _ = std::thread::Builder::new()
                    .name("faderust-exit-guard".into())
                    .spawn(|| {
                        std::thread::sleep(Duration::from_secs(3));
                        std::process::exit(0);
                    });
                return iced::exit();
            }
            Message::Open(d) => return self.open(d),
            Message::Loaded(l) => self.loaded(l),
            Message::Failed(e) => self.dialog_error = e,
            Message::Edit(e) => self.edit(e),
            Message::Route(i, o) => self.toggle_route(i, o),
            Message::Level(i, db) => self.move_strip(i, db),
            Message::Cell(i, o, pos) => self.set_cell(i, o, scale::clamp(scale::from_pos(pos))),
            Message::Master(o, db) => {
                if let Some(m) = self.outs.get_mut(o) {
                    m.db = db;
                    m.cache.clear();
                }
                self.send(Command::SetMasterGain(o, db));
            }
            Message::Mute(i) => {
                if let Some(s) = self.strips.get_mut(i) {
                    s.muted = !s.muted;
                    s.cache.clear();
                    let muted = s.muted;
                    self.send(Command::SetInputMuted(i, muted));
                }
            }
            Message::MasterMute(o) => {
                if let Some(m) = self.outs.get_mut(o) {
                    m.muted = !m.muted;
                    m.cache.clear();
                    let muted = m.muted;
                    self.send(Command::SetMasterMuted(o, muted));
                }
            }
            Message::Frame(now) => {
                #[cfg_attr(not(windows), allow(unused_mut))]
                let mut show = INSTANCE
                    .lock()
                    .ok()
                    .and_then(|g| g.as_ref().map(|rx| rx.try_iter().count() > 0))
                    .unwrap_or(false);
                #[cfg(windows)]
                if let Some(tray) = &self.tray {
                    for a in tray.poll() {
                        match a {
                            shell::tray::Action::Show => show = true,
                            shell::tray::Action::Quit => return self.handle(Message::Quit),
                        }
                    }
                }
                let due = match self.last_check {
                    None => now.duration_since(self.started) > Duration::from_secs(3),
                    Some(t) => t.elapsed() > Duration::from_secs(6 * 3600),
                };
                if due && !self.checking && self.shot.is_none() {
                    let t = self.check_update(false);
                    self.jobs.push(t);
                }
                if show {
                    let t = self.show();
                    self.jobs.push(t);
                }
                let dt = now.duration_since(self.last).as_secs_f32().min(0.1);
                self.last = now;
                let snap = self
                    .core
                    .last_state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                if let Some(snap) = snap {
                    self.apply(&snap, dt);
                    self.ready = true;
                }
                let notices: Vec<Notice> =
                    std::mem::take(&mut *self.notices.lock().unwrap_or_else(|e| e.into_inner()));
                if let Some(n) = notices.into_iter().last() {
                    self.say(n.level, n.text);
                }
                if self
                    .notice
                    .as_ref()
                    .is_some_and(|(at, ..)| at.elapsed() > Duration::from_secs(12))
                {
                    self.notice = None;
                }
                if self
                    .edit_at
                    .is_some_and(|t| t.elapsed() >= Duration::from_millis(800))
                {
                    self.flush_draft();
                }
                if self.dialog == Some(Dialog::Settings)
                    && !self.usb_busy
                    && self.last_usb.elapsed() >= Duration::from_secs(2)
                {
                    self.last_usb = Instant::now();
                    let t = Task::batch([self.load_usb(), self.load_deck()]);
                    self.jobs.push(t);
                }
                let t = now.duration_since(self.started).as_secs_f32();
                if self.shot.is_some() && t > 2.5 && self.ready {
                    return window::latest().map(Message::Id);
                }
            }
            Message::Id(Some(id)) => return window::screenshot(id).map(Message::Shot),
            Message::Id(None) => {}
            Message::Shot(shot) => {
                if let Some(path) = self.shot.take() {
                    save_png(&path, &shot);
                }
                return iced::exit();
            }
        }
        Task::none()
    }

    fn set_fx(&mut self, i: usize, f: impl FnOnce(&mut Fx)) {
        if let Some(fx) = self.fx.get_mut(i) {
            f(fx);
            let core: CoreFx = (&*fx).into();
            self.send(Command::SetFx(i, core));
        }
    }

    fn edit(&mut self, e: Edit) {
        match e {
            Edit::Denoise(i, on) => self.set_fx(i, |f| f.denoise = on),
            Edit::LowCut(i, hz) => self.set_fx(i, |f| f.low_cut = hz),
            Edit::Eq(i, b, v) => self.set_fx(i, |f| f.eq[b] = v),
            Edit::Gate(i, on) => self.set_fx(i, |f| f.gate = on),
            Edit::GateDb(i, v) => self.set_fx(i, |f| f.gate_db = v),
            Edit::ResetFx(i) => self.set_fx(i, |f| *f = CoreFx::default().into()),
            Edit::Refresh => {
                let t = self.refresh_dialog();
                self.jobs.push(t);
            }
            Edit::AppDevice(pids, device) => {
                self.jobs.push(blocking(
                    move || faderust_core::set_app_output(pids, device),
                    |r| Message::Loaded(Loaded::Done(Then::Apps, r)),
                ));
            }
            Edit::AppInputParked(device, label, pids, cable) => {
                self.jobs.push(blocking(
                    move || {
                        faderust_core::set_app_output(pids, Some(cable))?;
                        // Recording it mid-move can leave it without sound.
                        std::thread::sleep(Duration::from_millis(800));
                        Ok(())
                    },
                    move |r: Result<(), String>| match r {
                        Ok(()) => Message::Edit(Edit::AppInput(device.clone(), label.clone())),
                        Err(e) => Message::Failed(e),
                    },
                ));
            }
            Edit::AppInput(device, label) => {
                let r = self.add_input(device, label);
                if let Err(e) = r {
                    self.dialog_error = e;
                }
                self.jobs.push(self.load_apps());
            }
            Edit::WinDefault(id, comm) => {
                self.jobs.push(blocking(
                    move || faderust_core::set_default_device(id, comm),
                    |r| Message::Loaded(Loaded::Done(Then::Windows, r)),
                ));
            }
            Edit::WinVolume(id, v) => {
                let vol = (v / 100.0).clamp(0.0, 1.0);
                if let Some(e) = self.endpoint_mut(&id) {
                    e.volume = vol;
                }
                self.jobs.push(blocking(
                    move || faderust_core::set_device_volume(id, Some(vol), None),
                    |r| Message::Loaded(Loaded::Done(Then::Nothing, r)),
                ));
            }
            Edit::WinMute(id, muted) => {
                if let Some(e) = self.endpoint_mut(&id) {
                    e.muted = muted;
                }
                self.jobs.push(blocking(
                    move || faderust_core::set_device_volume(id, None, Some(muted)),
                    |r| Message::Loaded(Loaded::Done(Then::Nothing, r)),
                ));
            }
            Edit::InLabel(i, t) => self.draft_edit(|c| {
                if let Some(x) = c.inputs.get_mut(i) {
                    x.label = t;
                }
            }),
            Edit::InDevice(i, d) => self.draft_edit(|c| {
                if let Some(x) = c.inputs.get_mut(i) {
                    x.device = d;
                }
            }),
            Edit::InAdd => {
                let device = self.devs.inputs.first().map(|d| d.name.clone());
                if let Some(device) = device {
                    self.reshaped = true;
                    self.draft_edit(|c| {
                        c.inputs.push(InputEntry {
                            label: format!("Eingang {}", c.inputs.len() + 1),
                            device,
                            fx: Default::default(),
                            muted: false,
                        });
                        c.gains
                            .push(c.outputs.iter().map(|_| Gain::default()).collect());
                        for p in &mut c.presets {
                            p.gains
                                .push(c.outputs.iter().map(|_| Gain::default()).collect());
                        }
                    });
                }
            }
            Edit::InRemove(i) => {
                self.reshaped = true;
                self.draft_edit(|c| {
                    if i < c.inputs.len() {
                        c.inputs.remove(i);
                        if i < c.gains.len() {
                            c.gains.remove(i);
                        }
                        for p in &mut c.presets {
                            if i < p.gains.len() {
                                p.gains.remove(i);
                            }
                        }
                    }
                });
            }
            Edit::InMove(i, by) => {
                self.reshaped = true;
                self.draft_edit(|c| {
                    let j = i as isize + by;
                    if j >= 0 && (j as usize) < c.inputs.len() {
                        let j = j as usize;
                        c.inputs.swap(i, j);
                        if i < c.gains.len() && j < c.gains.len() {
                            c.gains.swap(i, j);
                        }
                        for p in &mut c.presets {
                            if i < p.gains.len() && j < p.gains.len() {
                                p.gains.swap(i, j);
                            }
                        }
                    }
                });
            }
            Edit::OutLabel(o, t) => self.draft_edit(|c| {
                if let Some(x) = c.outputs.get_mut(o) {
                    x.label = t;
                }
            }),
            Edit::OutDevice(o, d) => self.draft_edit(|c| {
                if let Some(x) = c.outputs.get_mut(o) {
                    x.device = d;
                }
            }),
            Edit::OutBuffer(o, t) => {
                if t.len() <= 3 && t.chars().all(|c| c.is_ascii_digit()) {
                    self.draft_edit(|c| {
                        if let Some(x) = c.outputs.get_mut(o) {
                            x.buffer_ms = t.parse().unwrap_or(0);
                        }
                    });
                }
            }
            Edit::OutRelease(o, secs) => self.draft_edit(|c| {
                if let Some(x) = c.outputs.get_mut(o) {
                    x.release_secs = secs;
                }
            }),
            Edit::OutAdd => {
                let device = self.devs.outputs.first().map(|d| d.name.clone());
                if let Some(device) = device {
                    self.reshaped = true;
                    self.draft_edit(|c| {
                        c.outputs.push(faderust_core::config::OutputEntry {
                            label: format!("Ausgang {}", c.outputs.len() + 1),
                            device,
                            buffer_ms: 60,
                            master: Default::default(),
                            release_secs: 30,
                        });
                        for row in c
                            .gains
                            .iter_mut()
                            .chain(c.presets.iter_mut().flat_map(|p| p.gains.iter_mut()))
                        {
                            row.push(Gain::default());
                        }
                    });
                }
            }
            Edit::MicDefault(on) => self.mic_default = on,
            Edit::MicAdd => {
                if let Some(cable) = self.mic_cable() {
                    self.reshaped = true;
                    let recordings: Vec<String> =
                        self.devs.inputs.iter().map(|d| d.name.clone()).collect();
                    let counterpart = faderust_core::names::cable_counterpart(&cable, &recordings);
                    self.draft_edit(|c| {
                        c.outputs.push(faderust_core::config::OutputEntry {
                            label: i18n::tr("Programm-Mikro"),
                            device: cable,
                            buffer_ms: 30,
                            master: Default::default(),
                            release_secs: 30,
                        });
                        // The first input (usually the microphone) at 0 dB, the rest muted.
                        for row in c
                            .gains
                            .iter_mut()
                            .chain(c.presets.iter_mut().flat_map(|p| p.gains.iter_mut()))
                        {
                            row.push(Gain::default());
                        }
                        for (i, row) in c.gains.iter_mut().enumerate() {
                            if let Some(g) = row.last_mut() {
                                g.muted = i != 0;
                            }
                        }
                    });
                    if self.mic_default {
                        let name = counterpart.clone();
                        self.jobs.push(blocking(
                            move || {
                                let w = faderust_core::get_windows_audio()?;
                                match w.recording.iter().find(|d| d.name == name) {
                                    Some(d) => {
                                        faderust_core::set_default_device(d.id.clone(), true)
                                    }
                                    None => Ok(()),
                                }
                            },
                            |r| Message::Loaded(Loaded::Done(Then::Nothing, r)),
                        ));
                    }
                    self.mic_device = Some(counterpart);
                }
            }
            Edit::OutRemove(o) => {
                self.reshaped = true;
                self.draft_edit(|c| {
                    if o < c.outputs.len() {
                        c.outputs.remove(o);
                        for row in c
                            .gains
                            .iter_mut()
                            .chain(c.presets.iter_mut().flat_map(|p| p.gains.iter_mut()))
                        {
                            if o < row.len() {
                                row.remove(o);
                            }
                        }
                    }
                });
            }
            Edit::OutMove(o, by) => {
                self.reshaped = true;
                self.draft_edit(|c| {
                    let j = o as isize + by;
                    if j >= 0 && (j as usize) < c.outputs.len() {
                        let j = j as usize;
                        c.outputs.swap(o, j);
                        for row in c
                            .gains
                            .iter_mut()
                            .chain(c.presets.iter_mut().flat_map(|p| p.gains.iter_mut()))
                        {
                            if o < row.len() && j < row.len() {
                                row.swap(o, j);
                            }
                        }
                    }
                });
            }
            Edit::UsbSetup => {
                self.usb_busy = true;
                self.dialog_error.clear();
                let core = Arc::clone(&self.core);
                self.jobs.push(blocking(
                    move || {
                        faderust_core::setup_usb_cable()?;
                        faderust_core::set_usb_cable(true, &core)
                    },
                    |r| Message::Loaded(Loaded::Done(Then::Usb, r)),
                ));
            }
            Edit::UsbEnable(on) => {
                let core = Arc::clone(&self.core);
                self.jobs.push(blocking(
                    move || faderust_core::set_usb_cable(on, &core),
                    |r| Message::Loaded(Loaded::Done(Then::Usb, r)),
                ));
            }
            Edit::UsbCount(n) => {
                let core = Arc::clone(&self.core);
                self.jobs.push(blocking(
                    move || faderust_core::set_usb_cable_count(n, &core),
                    |r| Message::Loaded(Loaded::Done(Then::Usb, r)),
                ));
            }
            Edit::UsbRemove => {
                self.usb_busy = true;
                let core = Arc::clone(&self.core);
                self.jobs.push(blocking(
                    move || faderust_core::remove_usb_cable(&core),
                    |r| Message::Loaded(Loaded::Done(Then::Usb, r)),
                ));
            }
            Edit::OpenLogFolder => {
                if let Err(e) = faderust_core::open_log_folder() {
                    self.say("info", e);
                }
            }
            Edit::CopyLog => {
                if let Some(t) = &self.log_text {
                    self.jobs.push(iced::clipboard::write(t.clone()));
                    self.say("info", "Protokoll kopiert".into());
                }
            }
            Edit::CopyChecks => {
                if let Some(c) = &self.checks {
                    let text = c
                        .iter()
                        .map(|c| format!("[{:?}] {}: {}", c.status, c.title, c.detail))
                        .collect::<Vec<_>>()
                        .join("\n");
                    self.jobs.push(iced::clipboard::write(text));
                    self.say("info", "Bericht kopiert".into());
                }
            }
            Edit::InstallDeck => {
                let r = faderust_core::install_streamdeck(
                    resource(faderust_core::PLUGIN_FILE),
                    &std::env::temp_dir().join("faderust"),
                );
                self.deck_error = r.err().unwrap_or_default();
            }
            Edit::Mode(m) => self.set_look(|p| p.mode = m.into()),
            Edit::Picker(t) => {
                self.picker = if self.picker.is_some_and(|(x, _)| x == t) {
                    None
                } else {
                    let hex = match t {
                        Target::Accent => self.accent(),
                        Target::Signal => self.signal(),
                    };
                    Some((t, style::rgb_to_hsv(style::hex(&hex))))
                };
            }
            Edit::Hsv(t, h, s, v) => {
                self.picker = Some((t, (h, s, v)));
                let hex = style::to_hex(style::hsv_to_rgb(h, s, v));
                let mut p = self.prefs.clone();
                match t {
                    Target::Accent => {
                        self.accent_input = Some(hex.clone());
                        p.accent = hex;
                    }
                    Target::Signal => {
                        self.signal_input = Some(hex.clone());
                        p.signal = hex;
                    }
                }
                // Live, without writing the file on every move.
                apply_look(&p);
                self.prefs = p;
                self.clear_caches();
            }
            Edit::HsvDone => shell::save_prefs(&self.dir, &self.prefs),
            Edit::AccentHex(t) => {
                let hex = style::parse_hex_input(&t);
                self.accent_input = Some(t);
                if let Some(c) = hex {
                    self.set_look(|p| p.accent = c);
                }
            }
            Edit::SignalHex(t) => {
                let hex = style::parse_hex_input(&t);
                self.signal_input = Some(t);
                if let Some(c) = hex {
                    self.set_look(|p| p.signal = c);
                }
            }
            Edit::Accent(c) => {
                self.accent_input = None;
                self.set_look(|p| p.accent = c);
            }
            Edit::Signal(c) => {
                self.signal_input = None;
                self.set_look(|p| p.signal = c);
            }
            Edit::Lang(l) => self.set_look(|p| p.lang = l.into()),
            Edit::MastersLeft(l) => {
                self.masters_left = l;
                self.save_prefs();
            }
            Edit::Autostart(on) => match shell::set_autostart(on) {
                Ok(()) => self.autostart = on,
                Err(e) => self.dialog_error = e,
            },
            Edit::HideAfter(h) => {
                self.hide_after = h;
                self.save_prefs();
            }
            Edit::NewPreset(t) => self.new_preset = t,
            Edit::SavePreset => {
                let name = std::mem::take(&mut self.new_preset);
                match faderust_core::save_preset(name, &self.core) {
                    Ok(()) => self.refresh_presets(),
                    Err(e) => self.say("error", e),
                }
            }
            Edit::ExportPreset(name) => {
                let core = self.core.clone();
                self.jobs.push(blocking(
                    move || -> Result<(), String> {
                        let text = faderust_core::export_preset(&name, &core)?;
                        let Some(path) = files::save_path(&name)? else {
                            return Ok(());
                        };
                        std::fs::write(&path, text).map_err(|e| e.to_string())
                    },
                    |r| Message::Loaded(Loaded::Done(Then::Nothing, r)),
                ));
            }
            Edit::ImportPreset => {
                let core = self.core.clone();
                self.jobs.push(blocking(
                    move || -> Result<(), String> {
                        let Some(path) = files::open_path()? else {
                            return Ok(());
                        };
                        let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                        faderust_core::import_preset(&text, &core).map(drop)
                    },
                    |r| Message::Loaded(Loaded::Done(Then::Presets, r)),
                ));
            }
            Edit::LoadPreset(name) => {
                if let Err(e) = faderust_core::load_preset(name, &self.core) {
                    self.say("error", e);
                }
                self.dialog = None;
            }
            Edit::DeletePreset(name) => {
                if let Err(e) = faderust_core::delete_preset(name, &self.core) {
                    self.say("error", e);
                }
                self.refresh_presets();
            }
            Edit::Keep(i) => {
                if let Some(s) = self.strips.get_mut(i) {
                    s.keep = !s.keep;
                }
                self.save_prefs();
            }
            Edit::HideOut(o, h) => {
                if let Some(m) = self.outs.get_mut(o) {
                    m.hidden = h;
                }
                if !h {
                    // "N ausgeblendet" brings them all back.
                    for o in &mut self.outs {
                        o.hidden = false;
                    }
                }
                self.save_prefs();
            }
            Edit::ShowSilent => self.strips.iter_mut().filter(|s| s.hidden).for_each(|s| {
                s.revealed = true;
                s.dismissed = false;
            }),
            Edit::Dismiss(i) => {
                if let Some(s) = self.strips.get_mut(i) {
                    s.dismissed = true;
                    s.keep = false;
                    s.revealed = false;
                    s.hidden = true;
                }
                self.save_prefs();
            }
            Edit::ShowOne(i) => {
                if let Some(s) = self.strips.get_mut(i) {
                    s.revealed = true;
                    s.dismissed = false;
                    s.hidden = false;
                }
            }
        }
    }

    // --- title bar -------------------------------------------------------

    fn titlebar(&self) -> Element<'_, Message> {
        let compact = self.compact;
        let nav = |glyph: icons::Glyph, label: &'static str, d: Dialog| {
            let on = self.dialog == Some(d);
            button(
                container(
                    {
                        let color = if on { on_accent() } else { accent_light() };
                        let icon: Element<'_, Message> = Canvas::new(icons::Icon {
                            glyph,
                            color,
                            on: false,
                        })
                        .width(24)
                        .height(24)
                        .into();
                        if compact {
                            row![icon]
                        } else {
                            row![
                                icon,
                                text(label).size(14).font(BOLD).color(if on {
                                    on_accent()
                                } else {
                                    text_color()
                                })
                            ]
                        }
                    }
                    .spacing(7)
                    .align_y(Vertical::Center),
                )
                .center_y(Length::Fill),
            )
            .height(if compact { 28 } else { 36 })
            .padding([0, if compact { 12 } else { 22 }])
            .on_press(Message::Open(Some(d)))
            .style(move |t, st| {
                let mut v = key(if on { Look::Primary } else { Look::Secondary })(t, st);
                if !on {
                    v.border.color = mix(border(), accent(), 0.3);
                    let hot = matches!(st, button::Status::Hovered | button::Status::Pressed);
                    // Plain black on dark, plain white on light.
                    let (base, lift) = if text_color().r > 0.5 {
                        (Color::BLACK, Color::WHITE)
                    } else {
                        (Color::WHITE, Color::BLACK)
                    };
                    v.background = Some(Background::Color(if hot {
                        mix(base, lift, 0.12)
                    } else {
                        base
                    }));
                }
                v.border.width = 1.5;
                v.border.radius = 6.0.into();
                v
            })
        };
        let win = |label: &'static str, close: bool, msg: Message| {
            button(container(text(label).size(12).color(muted_color())).center(Length::Fill))
                .width(46)
                .height(Length::Fill)
                .padding(0)
                .on_press(msg)
                .style(move |_, status| {
                    let hot = matches!(status, button::Status::Hovered);
                    button::Style {
                        background: hot.then(|| {
                            Background::Color(if close { rgb(196, 43, 28) } else { panel_2() })
                        }),
                        text_color: if hot { Color::WHITE } else { muted_color() },
                        border: Border::default(),
                        ..button::Style::default()
                    }
                })
        };
        let toggle = |glyph: icons::Glyph, on: bool, tip: &'static str, msg: Message| {
            dialogs::tip(
                button(
                    container(
                        Canvas::new(icons::Icon {
                            glyph,
                            color: if on { accent_light() } else { muted_color() },
                            on,
                        })
                        .width(18)
                        .height(18),
                    )
                    .center(Length::Fill),
                )
                .width(40)
                .height(Length::Fill)
                .padding(0)
                .on_press(msg)
                .style(move |_, status| {
                    let hot = matches!(status, button::Status::Hovered);
                    button::Style {
                        background: (hot || on).then(|| {
                            Background::Color(if hot {
                                panel_2()
                            } else {
                                mix(panel(), accent(), 0.12)
                            })
                        }),
                        text_color: muted_color(),
                        border: Border::default(),
                        ..button::Style::default()
                    }
                }),
                tip,
            )
        };
        let logo = image(self.logo.clone())
            .width(22)
            .height(22)
            .border_radius(3);
        let brand = if compact {
            row![logo]
        } else {
            row![
                logo,
                text("FADERUST").size(13).font(BOLD).color(text_color()),
                button(
                    text(concat!("v", env!("CARGO_PKG_VERSION")))
                        .size(12)
                        .font(MONO)
                        .color(muted_color())
                )
                .padding([2, 6])
                .on_press(Message::CheckUpdate)
                .style(|_, status| button::Style {
                    background: None,
                    text_color: muted_color(),
                    border: Border {
                        color: if matches!(status, button::Status::Hovered) {
                            border()
                        } else {
                            Color::TRANSPARENT
                        },
                        width: 1.0,
                        radius: 3.0.into(),
                    },
                    ..button::Style::default()
                }),
            ]
        }
        .spacing(8)
        .align_y(Vertical::Center);
        let brand: Element<'_, Message> = match &self.update {
            Some(u) => row![
                brand,
                button(
                    text(if self.updating {
                        "Lädt …".to_string()
                    } else {
                        format!("{} {}", i18n::tr("Update auf"), u.version)
                    })
                    .size(12)
                    .font(BOLD)
                    .color(on_accent())
                )
                .padding([4, 12])
                .on_press_maybe((!self.updating).then_some(Message::InstallUpdate))
                .style(key(Look::Primary)),
            ]
            .spacing(10)
            .align_y(Vertical::Center)
            .into(),
            None => brand.into(),
        };
        let bar = container(
            row![
                brand,
                space().width(Length::Fill),
                row![
                    nav(icons::Glyph::Bookmark, "Presets", Dialog::Presets),
                    nav(icons::Glyph::Speaker, "Windows", Dialog::Windows),
                    nav(icons::Glyph::Sliders, "Geräte", Dialog::Settings),
                    nav(icons::Glyph::Disc, "Design", Dialog::Design),
                ]
                .spacing(6),
                space().width(16),
                toggle(
                    icons::Glyph::Compact,
                    self.compact,
                    if self.compact {
                        "Normale Ansicht"
                    } else {
                        "Kompakte Ansicht"
                    },
                    Message::ToggleCompact
                ),
                toggle(
                    icons::Glyph::Pin,
                    self.pinned,
                    if self.pinned {
                        "Nicht mehr im Vordergrund halten"
                    } else {
                        "Immer im Vordergrund"
                    },
                    Message::TogglePin
                ),
                win("–", false, Message::Minimize),
                win("▢", false, Message::ToggleMax),
                win("✕", true, Message::CloseRequested),
            ]
            .spacing(12)
            .align_y(Vertical::Center),
        )
        .padding(iced::Padding::default().left(12))
        .height(if compact { 36 } else { 48 })
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(panel())),
            border: Border {
                color: border(),
                width: 1.0,
                ..Border::default()
            },
            ..container::Style::default()
        });
        iced::widget::mouse_area(bar)
            .on_press(Message::Drag)
            .on_double_click(Message::ToggleMax)
            .into()
    }

    // --- desk ------------------------------------------------------------

    fn name_label(text_: &str) -> Element<'static, Message> {
        container(
            text(text_.to_uppercase())
                .size(12)
                .color(text_color())
                .font(BOLD),
        )
        .width(Length::Fill)
        .padding([3, 4])
        .align_x(Horizontal::Center)
        .style(|_| container::Style {
            background: Some(Background::Color(panel_2())),
            border: Border {
                radius: 3.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
    }

    fn note(text_: &str, on: bool) -> Element<'static, Message> {
        container(
            text(text_.to_string())
                .size(11)
                .wrapping(iced::widget::text::Wrapping::None)
                .color(if on { signal() } else { muted_color() }),
        )
        .width(Length::Fill)
        .height(14)
        .align_x(Horizontal::Center)
        .clip(true)
        .into()
    }

    fn readout(db: f32, muted: bool) -> Element<'static, Message> {
        text(if db <= scale::OFF {
            "−∞ dB".to_string()
        } else {
            format!("{} dB", scale::format(db))
        })
        .font(Font {
            weight: iced::font::Weight::Bold,
            ..MONO
        })
        .size(13)
        .color(if muted { muted_color() } else { accent_light() })
        .into()
    }

    /// Lit keycap: an LED strip on top glows red when muted.
    fn mute_button(on: bool, msg: Message, label: &'static str) -> Element<'static, Message> {
        let key = button(
            text(label)
                .size(12)
                .font(BOLD)
                .color(if on { Color::WHITE } else { rgb(174, 180, 187) })
                .center(),
        )
        .width(72)
        .height(32)
        .padding(iced::Padding::default().top(6))
        .on_press(msg)
        .style(mute_style(on));
        let led =
            container(
                container(space())
                    .width(16)
                    .height(3)
                    .style(move |_| container::Style {
                        background: Some(Background::Color(if on {
                            rgb(255, 210, 207)
                        } else {
                            rgb(21, 23, 26)
                        })),
                        border: Border {
                            radius: 2.0.into(),
                            ..Border::default()
                        },
                        ..container::Style::default()
                    }),
            )
            .center_x(72)
            .padding([4, 0]);
        stack![key, led].into()
    }

    /// One strip in its card: A1-A3, name, note, fader, readout, mute, effects.
    fn card<'a>(content: Element<'a, Message>, width: f32) -> Element<'a, Message> {
        container(content)
            .padding([10, 6])
            .width(width)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .style(|_| container::Style {
                background: Some(Background::Color(panel())),
                border: Border {
                    color: hairline(),
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..container::Style::default()
            })
            .into()
    }

    fn chips(routes: &[bool], strip: Option<usize>, narrow: bool) -> Element<'static, Message> {
        let chips = row((0..routes.len()).map(|o| {
            let on = routes[o];
            let b = button(
                text(format!("A{}", o + 1))
                    .size(11)
                    .font(BOLD)
                    .color(if on { on_accent() } else { muted_color() })
                    .center(),
            )
            .width(if narrow { 22.0 } else { 34.0 })
            .height(22)
            .padding(0)
            .style(key(if on { Look::Primary } else { Look::Secondary }));
            match strip {
                Some(i) => b.on_press(Message::Route(i, o)),
                None => b,
            }
            .into()
        }))
        .spacing(if narrow { 3 } else { 4 });
        container(chips).center_x(Length::Fill).into()
    }

    fn strip(&self, i: usize) -> Element<'_, Message> {
        let s = &self.strips[i];
        let narrow = s.program || self.compact;
        let fader = Canvas::new(Fader {
            cache: &s.cache,
            db: s.db,
            pos: s.pos,
            muted: s.muted,
            level: s.shown,
            kind: Kind::Input(i),
            post_fader: false,
            narrow,
        })
        .width(if narrow { 104.0 } else { STRIP_W })
        .height(Length::Fill);
        let small = |label: &'static str, msg: Message, w: f32| {
            button(text(label).size(12).color(text_color()).center())
                .width(w)
                .height(24)
                .padding(0)
                .on_press(msg)
                .style(key(Look::Secondary))
        };
        let foot: Element<'_, Message> = if narrow && s.program {
            let keep = s.keep;
            let look = if keep { Look::Primary } else { Look::Secondary };
            row![
                small("FX", Message::Open(Some(Dialog::Fx(i))), 34.0),
                dialogs::tip(
                    button(
                        text(if keep { "◉" } else { "○" })
                            .size(13)
                            .color(look_text(look))
                            .center()
                    )
                    .width(34)
                    .height(24)
                    .padding(0)
                    .on_press(Message::Edit(Edit::Keep(i)))
                    .style(key(look)),
                    if keep {
                        "Bleibt auf dem Pult, auch wenn still"
                    } else {
                        "Immer anzeigen, auch wenn still"
                    },
                ),
            ]
            .spacing(4)
            .into()
        } else {
            small("Effekte", Message::Open(Some(Dialog::Fx(i))), 72.0).into()
        };
        let top: Element<'_, Message> = if s.program {
            row![
                Self::chips(&s.routes, Some(i), narrow),
                dialogs::tip(
                    button(text("✕").size(10).color(text_color()).center())
                        .width(20)
                        .height(22)
                        .padding(0)
                        .on_press(Message::Edit(Edit::Dismiss(i)))
                        .style(key(Look::Secondary)),
                    "Programm ausblenden",
                ),
            ]
            .spacing(3)
            .align_y(Vertical::Center)
            .into()
        } else {
            Self::chips(&s.routes, Some(i), narrow)
        };
        Self::card(
            column![
                top,
                Self::name_label(&s.label),
                space().height(14),
                fader,
                Self::readout(s.db, s.muted),
                Self::mute_button(s.muted, Message::Mute(i), "MUTE"),
                foot,
            ]
            .spacing(8)
            .align_x(Horizontal::Center)
            .into(),
            if narrow { 126.0 } else { 164.0 },
        )
    }

    fn master(&self, o: usize) -> Element<'_, Message> {
        let m = &self.outs[o];
        let fader = Canvas::new(Fader {
            cache: &m.cache,
            db: m.db,
            pos: m.pos,
            muted: m.muted,
            level: m.shown,
            kind: Kind::Master(o),
            post_fader: true,
            narrow: self.compact,
        })
        .width(if self.compact { 104.0 } else { STRIP_W })
        .height(Length::Fill);
        let tag = row![
            space().width(22),
            space().width(Length::Fill),
            container(
                text(format!("A{}", o + 1))
                    .size(11)
                    .font(BOLD)
                    .color(on_accent())
                    .center()
            )
            .width(34)
            .height(22)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .style(|_| container::Style {
                background: Some(Background::Color(accent())),
                border: Border {
                    radius: 3.0.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            }),
            space().width(Length::Fill),
            button(text("✕").size(10).color(text_color()).center())
                .width(22)
                .height(22)
                .padding(0)
                .on_press(Message::Edit(Edit::HideOut(o, true)))
                .style(key(Look::Secondary)),
        ]
        .align_y(Vertical::Center)
        .width(Length::Fill);
        Self::card(
            column![
                tag,
                Self::name_label(&m.label),
                if self.compact {
                    Element::from(space().height(0))
                } else {
                    Self::note(&m.note, m.note_on)
                },
                fader,
                Self::readout(m.db, m.muted),
                Self::mute_button(m.muted, Message::MasterMute(o), "MUTE"),
                space().height(24),
            ]
            .spacing(8)
            .align_x(Horizontal::Center)
            .into(),
            if self.compact { 126.0 } else { 164.0 },
        )
    }

    fn heading(
        label: &'static str,
        extra: Option<Element<'static, Message>>,
        compact: bool,
    ) -> Element<'static, Message> {
        let slim = compact && extra.is_none();
        let mut r = row![].spacing(12).align_y(Vertical::Center);
        if !compact {
            r = r.push(text(label).size(12).font(BOLD).color(muted_color()));
        }
        if let Some(e) = extra {
            r = r.push(e);
        }
        container(r)
            .height(if slim { 4 } else { 26 })
            .align_y(Vertical::Center)
            .into()
    }

    fn section<'a>(
        head: Element<'a, Message>,
        cards: Vec<Element<'a, Message>>,
        fill: bool,
    ) -> Element<'a, Message> {
        let strips = row(cards).spacing(10).height(Length::Fill);
        let strips: Element<'a, Message> = if fill {
            // The last group takes the rest of the window and scrolls sideways,
            // so its frame always ends at the window edge.
            iced::widget::scrollable(container(strips).padding(iced::Padding::default().bottom(10)))
                .direction(iced::widget::scrollable::Direction::Horizontal(
                    iced::widget::scrollable::Scrollbar::new()
                        .width(8)
                        .scroller_width(8)
                        .margin(2),
                ))
                .style(style::scroll_style)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            strips.into()
        };
        container(column![head, strips].spacing(6).height(Length::Fill))
            .padding([8, 10])
            .width(if fill { Length::Fill } else { Length::Shrink })
            .height(Length::Fill)
            .into()
    }

    /// Thin vertical rule between the groups.
    fn divider<'a>() -> Element<'a, Message> {
        container(space())
            .width(2)
            .height(Length::Fill)
            .padding([6, 0])
            .style(|_| container::Style {
                // In the design colour.
                background: Some(Background::Color(mix(panel(), accent(), 0.6))),
                ..container::Style::default()
            })
            .into()
    }

    fn desk(&self) -> Element<'_, Message> {
        let link = |label: String, msg: Message| -> Element<'static, Message> {
            button(text(label).size(12).color(accent_light()))
                .padding([2, 8])
                .on_press(msg)
                .style(|_, status| button::Style {
                    background: None,
                    text_color: accent_light(),
                    border: Border {
                        color: if matches!(status, button::Status::Hovered) {
                            accent()
                        } else {
                            Color::TRANSPARENT
                        },
                        width: 1.0,
                        radius: 3.0.into(),
                    },
                    ..button::Style::default()
                })
                .into()
        };
        let hidden_outs = self.outs.iter().filter(|o| o.hidden).count();
        let outputs = Self::section(
            Self::heading(
                "AUSGÄNGE",
                (hidden_outs > 0).then(|| {
                    link(
                        format!("{hidden_outs} {}", i18n::tr("ausgeblendet")),
                        Message::Edit(Edit::HideOut(0, false)),
                    )
                }),
                self.compact,
            ),
            (0..self.outs.len())
                .filter(|&o| !self.outs[o].hidden)
                .map(|o| self.master(o))
                .collect(),
            false,
        );
        let devices = Self::section(
            Self::heading("GERÄTE", None, self.compact),
            (0..self.strips.len())
                .filter(|&i| !self.strips[i].program)
                .map(|i| self.strip(i))
                .collect(),
            false,
        );
        let add: Element<'_, Message> =
            button(text("+ Programm").size(12).font(BOLD).color(accent_light()))
                .padding([4, 12])
                .on_press(Message::Open(Some(Dialog::Apps)))
                .style(key(Look::Secondary))
                .into();
        let silent = self.strips.iter().filter(|s| s.program && s.hidden).count();
        let extra: Element<'_, Message> = if silent > 0 {
            let mut choices = vec![dialogs::Choice {
                id: String::new(),
                label: i18n::tr("Alle zeigen"),
            }];
            choices.extend(
                self.strips
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.program && s.hidden)
                    .map(|(i, s)| dialogs::Choice {
                        id: i.to_string(),
                        label: s.label.clone(),
                    }),
            );
            let menu = style::pick(
                choices,
                None::<dialogs::Choice>,
                |c: dialogs::Choice| match c.id.parse() {
                    Ok(i) => Message::Edit(Edit::ShowOne(i)),
                    Err(_) => Message::Edit(Edit::ShowSilent),
                },
            )
            .placeholder(format!("{silent} {}", i18n::tr("still, zeigen")))
            .text_size(12)
            .padding([3, 8]);
            row![add, menu].spacing(8).align_y(Vertical::Center).into()
        } else {
            add
        };
        let programs = Self::section(
            Self::heading("PROGRAMME", Some(extra), self.compact),
            (0..self.strips.len())
                .filter(|&i| self.strips[i].program && !self.strips[i].hidden)
                .map(|i| self.strip(i))
                .collect(),
            true,
        );
        if self.masters_left {
            row![outputs, Self::divider(), devices, Self::divider(), programs].spacing(2)
        } else {
            row![devices, Self::divider(), programs, Self::divider(), outputs].spacing(2)
        }
        .height(Length::Fill)
        .into()
    }

    fn matrix(&self) -> Element<'_, Message> {
        const CELL: f32 = 250.0;
        const LABEL: f32 = 150.0;
        let head = row![space().width(LABEL)]
            .extend(self.outs.iter().enumerate().map(|(o, m)| {
                container(
                    row![
                        container(
                            text(format!("A{}", o + 1))
                                .size(11)
                                .font(BOLD)
                                .color(on_accent())
                                .center()
                        )
                        .width(30)
                        .height(20)
                        .center_x(30)
                        .center_y(20)
                        .style(|_| container::Style {
                            background: Some(Background::Color(accent())),
                            border: Border {
                                radius: 3.0.into(),
                                ..Border::default()
                            },
                            ..container::Style::default()
                        }),
                        column![
                            text(m.label.to_uppercase())
                                .size(12)
                                .font(BOLD)
                                .color(text_color()),
                            text(m.note.clone())
                                .size(11)
                                .wrapping(iced::widget::text::Wrapping::None)
                                .color(if m.note_on { signal() } else { muted_color() }),
                        ]
                        .spacing(1),
                    ]
                    .spacing(8)
                    .align_y(Vertical::Center),
                )
                .width(CELL)
                .clip(true)
                .into()
            }))
            .spacing(12);
        let mut rows = column![head].spacing(10);
        for (i, s) in self.strips.iter().enumerate() {
            let cells = (0..s.row.len()).map(|o| {
                let g = s.row[o];
                let dim = g.muted;
                let slider = iced::widget::slider(0.0..=1.0, scale::to_pos(g.db), move |p| {
                    Message::Cell(i, o, p)
                })
                .step(0.002_f32)
                .width(Length::Fill)
                .style(move |_, status| {
                    let hot = matches!(
                        status,
                        iced::widget::slider::Status::Hovered
                            | iced::widget::slider::Status::Dragged
                    );
                    let tone = if dim { muted_color() } else { accent() };
                    iced::widget::slider::Style {
                        rail: iced::widget::slider::Rail {
                            backgrounds: (Background::Color(tone), Background::Color(well())),
                            width: 6.0,
                            border: Border {
                                color: border(),
                                width: 1.0,
                                radius: 3.0.into(),
                            },
                        },
                        handle: iced::widget::slider::Handle {
                            shape: iced::widget::slider::HandleShape::Rectangle {
                                width: 12,
                                border_radius: 3.0.into(),
                            },
                            background: Background::Color(if hot {
                                accent_light()
                            } else {
                                mix(panel_2(), text_color(), 0.55)
                            }),
                            border_width: 1.0,
                            border_color: border(),
                        },
                    }
                });
                let look = if dim { Look::Danger } else { Look::Secondary };
                container(
                    row![
                        slider,
                        container(text(scale::format(g.db)).font(MONO).size(12).color(if dim {
                            muted_color()
                        } else {
                            text_color()
                        }))
                        .width(44)
                        .align_x(Horizontal::Right),
                        button(
                            text("M")
                                .size(11)
                                .font(BOLD)
                                .color(look_text(look))
                                .center()
                        )
                        .width(26)
                        .height(24)
                        .padding(0)
                        .on_press(Message::Route(i, o))
                        .style(key(look)),
                    ]
                    .spacing(8)
                    .align_y(Vertical::Center),
                )
                .width(CELL)
                .padding([6, 10])
                .style(|_| container::Style {
                    background: Some(Background::Color(panel())),
                    border: Border {
                        color: hairline(),
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..container::Style::default()
                })
                .into()
            });
            let name = button(
                column![
                    text(s.label.to_uppercase())
                        .size(12)
                        .font(BOLD)
                        .color(text_color()),
                    text(if s.program { "Programm" } else { "Gerät" })
                        .size(11)
                        .color(muted_color()),
                ]
                .spacing(1),
            )
            .width(LABEL)
            .padding([4, 8])
            .on_press(Message::Open(Some(Dialog::Fx(i))))
            .style(|_, status| button::Style {
                background: None,
                text_color: text_color(),
                border: Border {
                    color: if matches!(status, button::Status::Hovered) {
                        accent()
                    } else {
                        Color::TRANSPARENT
                    },
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..button::Style::default()
            });
            rows = rows.push(
                row![name]
                    .extend(cells)
                    .spacing(12)
                    .align_y(Vertical::Center),
            );
        }
        let add = button(text("+ Programm").size(12).font(BOLD).color(accent_light()))
            .padding([4, 12])
            .on_press(Message::Open(Some(Dialog::Apps)))
            .style(key(Look::Secondary));
        rows = rows.push(add);
        container(
            iced::widget::scrollable(container(rows).padding(4))
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .padding(12)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(mix(bg(), panel(), 0.45))),
            border: Border {
                color: border(),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..container::Style::default()
        })
        .into()
    }

    fn view(&self) -> Element<'_, Message> {
        let body = match self.view {
            View::Desk => self.desk(),
            View::Matrix => self.matrix(),
        };
        let toast: Element<'_, Message> = match &self.notice {
            Some((_, text_, level)) => {
                let (tone, glyph) = match *level {
                    "error" => (danger(), "!"),
                    "warn" => (amber(), "!"),
                    _ => (accent(), "i"),
                };
                let badge = container(text(glyph).size(13).font(BOLD).color(Color::WHITE).center())
                    .width(22)
                    .height(22)
                    .center_x(22)
                    .center_y(22)
                    .style(move |_| container::Style {
                        background: Some(Background::Color(tone)),
                        border: Border {
                            radius: 11.0.into(),
                            ..Border::default()
                        },
                        ..container::Style::default()
                    });
                let close = button(text("×").size(16).color(muted_color()).center())
                    .width(24)
                    .height(24)
                    .padding(0)
                    .on_press(Message::DismissNotice)
                    .style(|_, status| button::Style {
                        background: matches!(status, button::Status::Hovered)
                            .then(|| Background::Color(panel_2())),
                        text_color: muted_color(),
                        border: Border {
                            radius: 12.0.into(),
                            ..Border::default()
                        },
                        ..button::Style::default()
                    });
                container(
                    container(
                        row![
                            badge,
                            text(text_.clone()).size(13).color(text_color()),
                            close
                        ]
                        .spacing(12)
                        .align_y(Vertical::Center),
                    )
                    .padding([10, 14])
                    .max_width(620)
                    .style(move |_| container::Style {
                        background: Some(Background::Color(panel())),
                        border: Border {
                            color: mix(border(), tone, 0.6),
                            width: 1.0,
                            radius: 10.0.into(),
                        },
                        shadow: Shadow {
                            color: Color {
                                a: 0.5,
                                ..Color::BLACK
                            },
                            offset: iced::Vector::new(0.0, 6.0),
                            blur_radius: 18.0,
                        },
                        ..container::Style::default()
                    }),
                )
                .padding(24)
                .center_x(Length::Fill)
                .align_bottom(Length::Fill)
                .into()
            }
            None => space().height(0).into(),
        };
        let page = column![
            self.titlebar(),
            container(body)
                .padding(10)
                .width(Length::Fill)
                .height(Length::Fill),
        ];
        let page = container(page)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(bg())),
                ..container::Style::default()
            });
        let content: Element<'_, Message> = match self.dialog {
            Some(d) => stack![page, dialogs::view(self, d), toast].into(),
            None => stack![page, toast].into(),
        };
        // A visible edge around the borderless window, with grips to resize it.
        let framed = container(content)
            .padding(1)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(frame_color())),
                ..container::Style::default()
            });
        stack![framed, Self::grips()].into()
    }

    /// Invisible strips along the edges and corners that start a resize.
    fn grips() -> Element<'static, Message> {
        use iced::window::Direction as D;
        let grip = |w: Length,
                    h: Length,
                    d: D,
                    cursor: iced::mouse::Interaction|
         -> Element<'static, Message> {
            iced::widget::mouse_area(space().width(w).height(h))
                .on_press(Message::Resize(d))
                .interaction(cursor)
                .into()
        };
        let (edge, corner) = (5.0, 12.0);
        let fill = Length::Fill;
        let top = container(grip(
            fill,
            edge.into(),
            D::North,
            iced::mouse::Interaction::ResizingVertically,
        ))
        .align_top(fill);
        let bottom = container(grip(
            fill,
            edge.into(),
            D::South,
            iced::mouse::Interaction::ResizingVertically,
        ))
        .align_bottom(fill);
        let left = container(grip(
            edge.into(),
            fill,
            D::West,
            iced::mouse::Interaction::ResizingHorizontally,
        ))
        .align_left(fill);
        let right = container(grip(
            edge.into(),
            fill,
            D::East,
            iced::mouse::Interaction::ResizingHorizontally,
        ))
        .align_right(fill);
        let corner_grip = |d: D, cursor| grip(corner.into(), corner.into(), d, cursor);
        let diag_down = iced::mouse::Interaction::ResizingDiagonallyDown;
        let diag_up = iced::mouse::Interaction::ResizingDiagonallyUp;
        stack![
            top,
            bottom,
            left,
            right,
            container(corner_grip(D::NorthWest, diag_down))
                .align_top(fill)
                .align_left(fill),
            container(corner_grip(D::NorthEast, diag_up))
                .align_top(fill)
                .align_right(fill),
            container(corner_grip(D::SouthWest, diag_up))
                .align_bottom(fill)
                .align_left(fill),
            container(corner_grip(D::SouthEast, diag_down))
                .align_bottom(fill)
                .align_right(fill),
        ]
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let moving = self.strips.iter().any(|s| gliding(s.pos, s.db))
            || self.outs.iter().any(|m| gliding(m.pos, m.db));
        let every = if shell::is_visible(&self.visible) {
            // Faster while a fader glides, so the movement is fluid.
            if moving {
                11
            } else {
                33
            }
        } else {
            250
        };
        Subscription::batch([
            iced::time::every(Duration::from_millis(every)).map(Message::Frame),
            window::close_requests().map(|_| Message::CloseRequested),
        ])
    }
}

fn mute_style(on: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let (top, bottom, border) = if on {
            (rgb(232, 102, 95), rgb(176, 54, 48), rgb(90, 26, 23))
        } else if matches!(status, button::Status::Hovered) {
            (rgb(66, 70, 76), rgb(40, 43, 47), rgb(16, 17, 19))
        } else {
            (rgb(59, 63, 69), rgb(36, 39, 43), rgb(16, 17, 19))
        };
        button::Style {
            background: Some(Background::Gradient(
                gradient::Linear::new(iced::Radians(std::f32::consts::PI))
                    .add_stop(0.0, top)
                    .add_stop(1.0, bottom)
                    .into(),
            )),
            text_color: Color::WHITE,
            border: Border {
                color: border,
                width: 1.0,
                radius: 4.0.into(),
            },
            shadow: Shadow {
                color: Color {
                    a: 0.5,
                    ..Color::BLACK
                },
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 3.0,
            },
            snap: true,
        }
    }
}

fn save_png(path: &str, shot: &window::Screenshot) {
    let file = std::fs::File::create(path).expect("create png");
    let mut enc = png::Encoder::new(
        std::io::BufWriter::new(file),
        shot.size.width,
        shot.size.height,
    );
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .expect("png header")
        .write_image_data(&shot.rgba)
        .expect("png data");
}

fn theme(_: &App) -> Theme {
    Theme::custom(
        "Faderust".to_string(),
        iced::theme::Palette {
            background: bg(),
            text: text_color(),
            primary: accent(),
            success: signal(),
            warning: amber(),
            danger: danger(),
        },
    )
}

/// Set by `main` when this is the running instance.
static INSTANCE: Mutex<Option<Receiver<()>>> = Mutex::new(None);

#[cfg(windows)]
fn tray_icon() -> Option<shell::tray::Tray> {
    let decoder = png::Decoder::new(std::io::Cursor::new(LOGO));
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    (info.width == info.height && info.color_type == png::ColorType::Rgba)
        .then(|| shell::tray::Tray::new(buf, info.width))
        .flatten()
}

fn main() -> iced::Result {
    // A second start only brings the running window to the front.
    match shell::single_instance() {
        Ok(rx) => *INSTANCE.lock().unwrap_or_else(|e| e.into_inner()) = Some(rx),
        Err(()) => return Ok(()),
    }
    let minimized = std::env::args().any(|a| a == shell::MINIMIZED_ARG);
    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .theme(theme)
        .window(window::Settings {
            size: Size::new(1400.0, 860.0),
            min_size: Some(Size::new(480.0, 420.0)),
            icon: window::icon::from_file_data(LOGO, None).ok(),
            // The title bar is drawn by the app; closing goes to the tray.
            decorations: false,
            visible: !minimized,
            exit_on_close_request: false,
            ..window::Settings::default()
        })
        .title("Faderust")
        .run()
}

#[cfg(test)]
mod glide_tests {
    use super::*;

    #[test]
    fn fader_glides_without_overshoot_and_settles() {
        let (mut pos, mut vel) = (0.2_f32, 0.0_f32);
        let want = scale::to_pos(0.0);
        let mut last = pos;
        for _ in 0..300 {
            glide(&mut pos, &mut vel, 0.0, 1.0 / 90.0);
            assert!(pos >= last - 1e-6 && pos <= want + 1e-4, "overshoot");
            last = pos;
        }
        assert_eq!(pos, want);
        assert!(!glide(&mut pos, &mut vel, 0.0, 1.0 / 90.0));
    }
}
