//! Faderust window built with iced instead of Tauri/WebView2: title bar, desk
//! (strips for inputs and output masters with A1-An), matrix and the
//! dialogs, driven by `faderust-core`.
//!
//! Wired to the engine: faders, mute, routing, effects, presets, hiding of
//! silent programs. Still placeholders (next stages): the dialogs Programme,
//! Windows-Sound and Geräte.
//!
//! `FADERUST_SHOT=out.png` saves one screenshot after a moment and exits;
//! `FADERUST_VIEW=matrix` and `FADERUST_DIALOG=<name>` pick what it shows.

mod dialogs;
mod fader;
mod scale;
mod style;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use faderust_core::audio::{Command, Notice, UiState};
use faderust_core::config::{Fx as CoreFx, Gain};
use faderust_core::{AppState, Host};

use iced::alignment::{Horizontal, Vertical};
use iced::gradient;
use iced::widget::canvas::{self, Canvas};
use iced::widget::{button, column, container, image, row, space, stack, text};
use iced::{
    window, Background, Border, Color, Element, Font, Length, Shadow, Size, Subscription, Task,
    Theme,
};

use fader::{Fader, Kind};
use style::*;

const LOGO: &[u8] = include_bytes!("../assets/logo.png");

pub const DEV_OUT: &[&str] = &[
    "Lautsprecher (Realtek Audio)",
    "Kopfhörer (WH-1000XM5)",
    "CABLE Input (VB-Audio)",
    "Stream (Elgato Wave:3)",
];
pub const DEV_IN: &[&str] = &[
    "Mikrofon (Blue Yeti)",
    "CABLE Output (VB-Audio)",
    "Programm: Spiel",
    "Programm: Browser",
];
pub const APP_OUT: &[&str] = &[
    "Windows-Standard",
    "CABLE Input (VB-Audio)",
    "Kopfhörer (WH-1000XM5)",
];
pub const RELEASE: &[&str] = &["Nie", "Nach 5 s", "Nach 30 s", "Nach 2 min"];
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
    target: f32,
    pub connected: bool,
    pub routes: Vec<bool>,
    /// Programs are recorded from one program, devices from a capture device.
    pub program: bool,
    /// A silent program the desk hides until it is shown.
    pub hidden: bool,
    /// Kept on the desk even when silent (the eye button).
    pub keep: bool,
    /// Shown on request ("N still, zeigen") until the window closes.
    revealed: bool,
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

pub struct AppRow {
    pub name: &'static str,
    pub device: &'static str,
    pub input: bool,
}

pub struct Settings {
    pub autostart: bool,
    pub inputs: Vec<(String, &'static str)>,
    pub outputs: Vec<(String, &'static str, String, &'static str)>,
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
    Fx(usize),
}

pub struct App {
    pub view: View,
    pub masters_left: bool,
    pub hide_after: &'static str,
    pub dialog: Option<Dialog>,
    core: Arc<AppState>,
    notices: Arc<Mutex<Vec<Notice>>>,
    /// Message for the user: when it came, text, level.
    pub notice: Option<(Instant, String, &'static str)>,
    pub strips: Vec<Strip>,
    pub outs: Vec<Out>,
    pub fx: Vec<Fx>,
    pub apps: Vec<AppRow>,
    pub settings: Settings,
    pub presets: Vec<String>,
    pub new_preset: String,
    pub win_default: usize,
    pub win_volume: [f32; 3],
    pub usb_running: bool,
    pub usb_count: u8,
    last: Instant,
    started: Instant,
    ready: bool,
    shot: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Edit {
    Denoise(usize, bool),
    LowCut(usize, u16),
    Eq(usize, usize, f32),
    Gate(usize, bool),
    GateDb(usize, f32),
    ResetFx(usize),
    AppDevice(usize, &'static str),
    AppInput(usize),
    Autostart(bool),
    InLabel(usize, String),
    InDevice(usize, &'static str),
    OutLabel(usize, String),
    OutDevice(usize, &'static str),
    OutBuffer(usize, String),
    OutRelease(usize, &'static str),
    MastersLeft(bool),
    HideAfter(&'static str),
    NewPreset(String),
    SavePreset,
    LoadPreset(String),
    DeletePreset(String),
    Keep(usize),
    WinDefault(usize),
    WinVolume(usize, f32),
    HideOut(usize, bool),
    ShowSilent,
    UsbRunning(bool),
    UsbCount(u8),
}

#[derive(Debug, Clone)]
pub enum Message {
    SetView(View),
    Open(Option<Dialog>),
    Edit(Edit),
    Route(usize, usize),
    Level(usize, f32),
    Master(usize, f32),
    Mute(usize),
    MasterMute(usize),
    Frame(Instant),
    Shot(window::Screenshot),
    Id(Option<window::Id>),
}

/// The audio thread tells the window through the shared snapshot, so only
/// notices need a queue.
struct GuiHost {
    notices: Arc<Mutex<Vec<Notice>>>,
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
        true
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

fn level_of(db: f32) -> f32 {
    if db.is_finite() {
        db
    } else {
        -120.0
    }
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let notices = Arc::new(Mutex::new(Vec::new()));
        let core = faderust_core::start(
            Arc::new(GuiHost {
                notices: Arc::clone(&notices),
            }),
            &config_dir(),
        );
        let dialog = std::env::var("FADERUST_DIALOG")
            .ok()
            .and_then(|d| match d.as_str() {
                "presets" => Some(Dialog::Presets),
                "windows" => Some(Dialog::Windows),
                "settings" => Some(Dialog::Settings),
                "design" => Some(Dialog::Design),
                "apps" => Some(Dialog::Apps),
                "fx" => Some(Dialog::Fx(0)),
                _ => None,
            });
        let mut app = App {
            view: if std::env::var("FADERUST_VIEW").as_deref() == Ok("matrix") {
                View::Matrix
            } else {
                View::Desk
            },
            masters_left: true,
            hide_after: HIDE_AFTER[2],
            dialog,
            core,
            notices,
            notice: None,
            strips: vec![],
            outs: vec![],
            fx: vec![],
            apps: vec![
                AppRow {
                    name: "Spiel (forza.exe)",
                    device: APP_OUT[1],
                    input: true,
                },
                AppRow {
                    name: "Browser (brave.exe)",
                    device: APP_OUT[0],
                    input: true,
                },
            ],
            settings: Settings {
                autostart: true,
                inputs: vec![("Mikrofon".into(), DEV_IN[0])],
                outputs: vec![("PS5".into(), DEV_OUT[0], "60".into(), RELEASE[0])],
            },
            presets: vec![],
            new_preset: String::new(),
            win_default: 0,
            win_volume: [80.0, 55.0, 100.0],
            usb_running: false,
            usb_count: 1,
            last: Instant::now(),
            started: Instant::now(),
            ready: false,
            shot: std::env::var("FADERUST_SHOT").ok(),
        };
        app.refresh_presets();
        (app, Task::none())
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
                target: -120.0,
                connected: false,
                routes: vec![],
                program: false,
                hidden: false,
                keep: false,
                revealed: false,
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
            s.target = level_of(d.level_db);
            if s.connected && s.target > -60.0 {
                s.heard = now;
            }
            s.hidden = s.program
                && secs > 0
                && !s.keep
                && !s.revealed
                && now.duration_since(s.heard) >= Duration::from_secs(secs);
            if self.dialog != Some(Dialog::Fx(i)) {
                if let Some(fx) = d.fx {
                    self.fx[i] = fx.into();
                }
            }
            s.shown = ease(s.shown, s.target, dt);
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
                target: -120.0,
                playing: false,
            });
        }
        self.outs.truncate(snap.outputs.len());
        for (m, d) in self.outs.iter_mut().zip(&snap.outputs) {
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
        match message {
            Message::SetView(v) => self.view = v,
            Message::Open(d) => self.dialog = d,
            Message::Edit(e) => self.edit(e),
            Message::Route(i, o) => self.toggle_route(i, o),
            Message::Level(i, db) => self.move_strip(i, db),
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
            Edit::AppDevice(i, d) => self.apps[i].device = d,
            Edit::AppInput(i) => self.apps[i].input = true,
            Edit::Autostart(on) => self.settings.autostart = on,
            Edit::InLabel(i, t) => self.settings.inputs[i].0 = t,
            Edit::InDevice(i, d) => self.settings.inputs[i].1 = d,
            Edit::OutLabel(i, t) => self.settings.outputs[i].0 = t,
            Edit::OutDevice(i, d) => self.settings.outputs[i].1 = d,
            Edit::OutBuffer(i, t) => {
                if t.len() <= 3 && t.chars().all(|c| c.is_ascii_digit()) {
                    self.settings.outputs[i].2 = t;
                }
            }
            Edit::OutRelease(i, r) => self.settings.outputs[i].3 = r,
            Edit::MastersLeft(l) => self.masters_left = l,
            Edit::HideAfter(h) => self.hide_after = h,
            Edit::NewPreset(t) => self.new_preset = t,
            Edit::SavePreset => {
                let name = std::mem::take(&mut self.new_preset);
                match faderust_core::save_preset(name, &self.core) {
                    Ok(()) => self.refresh_presets(),
                    Err(e) => self.say("error", e),
                }
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
            }
            Edit::WinDefault(i) => self.win_default = i,
            Edit::WinVolume(i, v) => self.win_volume[i] = v.round(),
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
            }
            Edit::ShowSilent => self
                .strips
                .iter_mut()
                .filter(|s| s.hidden)
                .for_each(|s| s.revealed = true),
            Edit::UsbRunning(on) => self.usb_running = on,
            Edit::UsbCount(n) => self.usb_count = n,
        }
    }

    // --- title bar -------------------------------------------------------

    fn titlebar(&self) -> Element<'_, Message> {
        let nav = |icon: &'static str, label: &'static str, d: Dialog| {
            let on = self.dialog == Some(d);
            button(
                container(
                    row![
                        text(icon)
                            .size(14)
                            .color(if on { Color::WHITE } else { ACCENT }),
                        text(label).size(13).font(BOLD).color(Color::WHITE)
                    ]
                    .spacing(7)
                    .align_y(Vertical::Center),
                )
                .center_y(Length::Fill),
            )
            .height(32)
            .padding([0, 14])
            .on_press(Message::Open(Some(d)))
            .style(move |_, status| nav_style(on, matches!(status, button::Status::Hovered)))
        };
        let win = |label: &'static str, close: bool| {
            button(container(text(label).size(12).color(MUTED)).center(Length::Fill))
                .width(46)
                .height(Length::Fill)
                .padding(0)
                .style(move |_, status| {
                    let hot = matches!(status, button::Status::Hovered);
                    button::Style {
                        background: hot.then(|| {
                            Background::Color(if close { rgb(196, 43, 28) } else { PANEL_2 })
                        }),
                        text_color: if hot { Color::WHITE } else { MUTED },
                        border: Border::default(),
                        ..button::Style::default()
                    }
                })
        };
        let brand = row![
            image(image::Handle::from_bytes(LOGO))
                .width(22)
                .height(22)
                .border_radius(3),
            text("FADERUST").size(13).font(BOLD).color(TEXT),
            button(
                text(concat!("v", env!("CARGO_PKG_VERSION")))
                    .size(12)
                    .font(MONO)
                    .color(MUTED)
            )
            .padding([2, 6])
            .style(|_, status| button::Style {
                background: None,
                text_color: MUTED,
                border: Border {
                    color: if matches!(status, button::Status::Hovered) {
                        BORDER
                    } else {
                        Color::TRANSPARENT
                    },
                    width: 1.0,
                    radius: 3.0.into(),
                },
                ..button::Style::default()
            }),
        ]
        .spacing(8)
        .align_y(Vertical::Center);
        container(
            row![
                brand,
                space().width(Length::Fill),
                row![
                    nav("⚐", "Presets", Dialog::Presets),
                    nav("♪", "Windows", Dialog::Windows),
                    nav("≡", "Geräte", Dialog::Settings),
                    nav("◐", "Design", Dialog::Design),
                ]
                .spacing(6),
                space().width(24),
                win("–", false),
                win("▢", false),
                win("✕", true),
            ]
            .spacing(12)
            .align_y(Vertical::Center),
        )
        .padding(iced::Padding::default().left(12))
        .height(48)
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(PANEL)),
            border: Border {
                color: BORDER,
                width: 1.0,
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
    }

    // --- desk ------------------------------------------------------------

    fn name_label(text_: &str) -> Element<'static, Message> {
        container(text(text_.to_uppercase()).size(12).color(TEXT).font(BOLD))
            .width(Length::Fill)
            .padding([3, 4])
            .align_x(Horizontal::Center)
            .style(|_| container::Style {
                background: Some(Background::Color(PANEL_2)),
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
                .color(if on { SIGNAL } else { MUTED }),
        )
        .width(Length::Fill)
        .height(14)
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
        .color(if muted { MUTED } else { ACCENT_LIGHT })
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
                background: Some(Background::Color(PANEL)),
                border: Border {
                    color: BORDER,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..container::Style::default()
            })
            .into()
    }

    fn chips(routes: &[bool], strip: Option<usize>, narrow: bool) -> Element<'static, Message> {
        row((0..routes.len()).map(|o| {
            let on = routes[o];
            let b = button(
                text(format!("A{}", o + 1))
                    .size(11)
                    .font(BOLD)
                    .color(if on { Color::WHITE } else { MUTED })
                    .center(),
            )
            .width(if narrow { 26.0 } else { 34.0 })
            .height(22)
            .padding(0)
            .style(move |_, _| small_toggle(on));
            match strip {
                Some(i) => b.on_press(Message::Route(i, o)),
                None => b,
            }
            .into()
        }))
        .spacing(4)
        .into()
    }

    fn strip(&self, i: usize) -> Element<'_, Message> {
        let s = &self.strips[i];
        let narrow = s.program;
        let fader = Canvas::new(Fader {
            cache: &s.cache,
            db: s.db,
            muted: s.muted,
            level: s.shown,
            kind: Kind::Input(i),
            post_fader: false,
            narrow,
        })
        .width(if narrow { 84.0 } else { STRIP_W })
        .height(Length::Fill);
        let small = |label: &'static str, msg: Message, w: f32| {
            button(text(label).size(12).color(TEXT).center())
                .width(w)
                .height(24)
                .padding(0)
                .on_press(msg)
                .style(|_, _| flat(PANEL_2))
        };
        let foot: Element<'_, Message> = if narrow {
            row![
                small("FX", Message::Open(Some(Dialog::Fx(i))), 34.0),
                small("◉", Message::Edit(Edit::Keep(i)), 34.0),
            ]
            .spacing(4)
            .into()
        } else {
            small("Effekte", Message::Open(Some(Dialog::Fx(i))), 72.0).into()
        };
        Self::card(
            column![
                Self::chips(&s.routes, Some(i), narrow),
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
            if narrow { 106.0 } else { 150.0 },
        )
    }

    fn master(&self, o: usize) -> Element<'_, Message> {
        let m = &self.outs[o];
        let fader = Canvas::new(Fader {
            cache: &m.cache,
            db: m.db,
            muted: m.muted,
            level: m.shown,
            kind: Kind::Master(o),
            post_fader: true,
            narrow: false,
        })
        .width(STRIP_W)
        .height(Length::Fill);
        let tag = row![
            container(
                text(format!("A{}", o + 1))
                    .size(11)
                    .font(BOLD)
                    .color(Color::WHITE)
                    .center()
            )
            .width(34)
            .height(22)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .style(|_| container::Style {
                background: Some(Background::Color(ACCENT)),
                border: Border {
                    radius: 3.0.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            }),
            space().width(Length::Fill),
            button(text("×").size(14).color(MUTED).center())
                .width(22)
                .height(22)
                .padding(0)
                .on_press(Message::Edit(Edit::HideOut(o, true)))
                .style(|_, status| button::Style {
                    background: None,
                    text_color: if matches!(status, button::Status::Hovered) {
                        TEXT
                    } else {
                        MUTED
                    },
                    ..button::Style::default()
                }),
        ]
        .align_y(Vertical::Center)
        .width(Length::Fill);
        Self::card(
            column![
                tag,
                Self::name_label(&m.label),
                Self::note(&m.note, m.note_on),
                fader,
                Self::readout(m.db, m.muted),
                Self::mute_button(m.muted, Message::MasterMute(o), "MUTE"),
                space().height(24),
            ]
            .spacing(8)
            .align_x(Horizontal::Center)
            .into(),
            150.0,
        )
    }

    fn heading(
        label: &'static str,
        extra: Option<Element<'static, Message>>,
    ) -> Element<'static, Message> {
        let mut r = row![text(label).size(12).font(BOLD).color(MUTED)]
            .spacing(12)
            .align_y(Vertical::Center);
        if let Some(e) = extra {
            r = r.push(e);
        }
        container(r).height(26).into()
    }

    fn section<'a>(
        head: Element<'a, Message>,
        cards: Vec<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        column![head, row(cards).spacing(10).height(Length::Fill)]
            .spacing(6)
            .height(Length::Fill)
            .into()
    }

    fn divider() -> Element<'static, Message> {
        container(space())
            .width(2)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(BORDER)),
                ..container::Style::default()
            })
            .into()
    }

    fn desk(&self) -> Element<'_, Message> {
        let link = |label: String, msg: Message| -> Element<'static, Message> {
            button(text(label).size(12).color(ACCENT_LIGHT))
                .padding([2, 8])
                .on_press(msg)
                .style(|_, status| button::Style {
                    background: None,
                    text_color: ACCENT_LIGHT,
                    border: Border {
                        color: if matches!(status, button::Status::Hovered) {
                            ACCENT
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
                        format!("{hidden_outs} ausgeblendet"),
                        Message::Edit(Edit::HideOut(0, false)),
                    )
                }),
            ),
            (0..self.outs.len())
                .filter(|&o| !self.outs[o].hidden)
                .map(|o| self.master(o))
                .collect(),
        );
        let devices = Self::section(
            Self::heading("GERÄTE", None),
            (0..self.strips.len())
                .filter(|&i| !self.strips[i].program)
                .map(|i| self.strip(i))
                .collect(),
        );
        let add: Element<'_, Message> =
            button(text("+ Programm").size(12).font(BOLD).color(ACCENT_LIGHT))
                .height(24)
                .padding([0, 12])
                .on_press(Message::Open(Some(Dialog::Apps)))
                .style(|_, _| button::Style {
                    background: None,
                    text_color: ACCENT_LIGHT,
                    border: Border {
                        color: ACCENT,
                        width: 1.0,
                        radius: 3.0.into(),
                    },
                    ..button::Style::default()
                })
                .into();
        let silent = self.strips.iter().filter(|s| s.program && s.hidden).count();
        let extra: Element<'_, Message> = if silent > 0 {
            row![
                add,
                link(
                    format!("{silent} still, zeigen"),
                    Message::Edit(Edit::ShowSilent)
                )
            ]
            .spacing(8)
            .align_y(Vertical::Center)
            .into()
        } else {
            add
        };
        let programs = Self::section(
            Self::heading("PROGRAMME", Some(extra)),
            (0..self.strips.len())
                .filter(|&i| self.strips[i].program && !self.strips[i].hidden)
                .map(|i| self.strip(i))
                .collect(),
        );
        if self.masters_left {
            row![outputs, Self::divider(), devices, Self::divider(), programs].spacing(10)
        } else {
            row![devices, Self::divider(), programs, Self::divider(), outputs].spacing(10)
        }
        .height(Length::Fill)
        .into()
    }

    fn matrix(&self) -> Element<'_, Message> {
        let head = row![space().width(110)].extend(self.outs.iter().map(|o| {
            container(text(o.label.to_uppercase()).size(12).color(TEXT).font(BOLD))
                .width(110)
                .align_x(Horizontal::Center)
                .into()
        }));
        let mut rows = column![head].spacing(10);
        for (i, s) in self.strips.iter().enumerate() {
            let cells = (0..s.routes.len()).map(|o| {
                let on = s.routes[o];
                button(
                    text(if on {
                        scale::format(s.row[o].db)
                    } else {
                        "AUS".into()
                    })
                    .font(MONO)
                    .size(13)
                    .color(if on { Color::BLACK } else { MUTED })
                    .center(),
                )
                .width(110)
                .height(40)
                .on_press(Message::Route(i, o))
                .style(move |_, _| button::Style {
                    background: Some(Background::Color(if on { SIGNAL } else { PANEL_2 })),
                    text_color: Color::WHITE,
                    border: Border {
                        color: BORDER,
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    ..button::Style::default()
                })
                .into()
            });
            rows = rows.push(
                row![
                    container(text(s.label.to_uppercase()).size(12).color(TEXT).font(BOLD))
                        .width(110)
                ]
                .extend(cells)
                .align_y(Vertical::Center),
            );
        }
        rows.into()
    }

    fn view(&self) -> Element<'_, Message> {
        let body = match self.view {
            View::Desk => self.desk(),
            View::Matrix => self.matrix(),
        };
        let banner: Element<'_, Message> = match &self.notice {
            Some((_, text_, level)) => container(text(text_.clone()).size(13).color(Color::WHITE))
                .padding([6, 12])
                .width(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(Background::Color(match *level {
                        "error" => DANGER,
                        "warn" => AMBER,
                        _ => ACCENT,
                    })),
                    ..container::Style::default()
                })
                .into(),
            None => space().height(0).into(),
        };
        let page = column![
            self.titlebar(),
            banner,
            container(body)
                .padding(10)
                .width(Length::Fill)
                .height(Length::Fill),
        ];
        let page = container(page)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(BG)),
                ..container::Style::default()
            });
        match self.dialog {
            Some(d) => stack![page, dialogs::view(self, d)].into(),
            None => page.into(),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(33)).map(Message::Frame)
    }
}

fn nav_style(active: bool, hot: bool) -> button::Style {
    button::Style {
        background: Some(Background::Color(if active { ACCENT } else { PANEL_2 })),
        text_color: Color::WHITE,
        border: Border {
            color: if active || hot { ACCENT } else { BORDER },
            width: 1.0,
            radius: 3.0.into(),
        },
        ..button::Style::default()
    }
}

pub fn flat(color: Color) -> button::Style {
    button::Style {
        background: Some(Background::Color(color)),
        text_color: Color::WHITE,
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..button::Style::default()
    }
}

fn small_toggle(on: bool) -> button::Style {
    button::Style {
        background: Some(Background::Color(if on { ACCENT } else { PANEL_2 })),
        text_color: Color::WHITE,
        border: Border {
            color: if on { ACCENT } else { BORDER },
            width: 1.0,
            radius: 3.0.into(),
        },
        ..button::Style::default()
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
            background: BG,
            text: TEXT,
            primary: ACCENT,
            success: SIGNAL,
            warning: AMBER,
            danger: DANGER,
        },
    )
}

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .theme(theme)
        .window(window::Settings {
            size: Size::new(1400.0, 860.0),
            icon: window::icon::from_file_data(LOGO, None).ok(),
            ..window::Settings::default()
        })
        .title("Faderust")
        .run()
}
