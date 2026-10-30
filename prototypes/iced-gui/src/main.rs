//! iced prototype of the Faderust window: title bar, desk (strips for inputs
//! and output masters with A1-A3), matrix and the dialogs. Not connected to
//! the engine; the meters show made-up levels. Meant to judge look and CPU.
//!
//! `FADERUST_SHOT=out.png` saves one screenshot after a moment and exits;
//! `FADERUST_VIEW=matrix` and `FADERUST_DIALOG=<name>` pick what it shows.

mod dialogs;
mod fader;
mod scale;
mod style;

use std::time::{Duration, Instant};

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

pub const OUT_NAMES: [&str; 3] = ["PS5", "Kopfhörer", "Box"];
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

pub struct Strip {
    pub cache: canvas::Cache,
    pub label: &'static str,
    pub db: f32,
    pub muted: bool,
    pub shown: f32,
    phase: f32,
    pub routes: [bool; 3],
    /// Programs are recorded from one program, devices from a capture device.
    pub program: bool,
    /// A silent program the desk hides until it is shown.
    pub hidden: bool,
}

pub struct Out {
    pub cache: canvas::Cache,
    pub label: &'static str,
    /// Device state under the name: where it plays, or "Freigegeben".
    pub note: &'static str,
    pub note_on: bool,
    pub hidden: bool,
    pub db: f32,
    pub muted: bool,
    pub shown: f32,
    phase: f32,
}

#[derive(Clone, PartialEq)]
pub struct Fx {
    pub denoise: bool,
    pub low_cut: u16,
    pub eq: [f32; 3],
    pub gate: bool,
    pub gate_db: f32,
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

impl App {
    fn new() -> (Self, Task<Message>) {
        let strip = |label, db, phase, routes, program| Strip {
            cache: canvas::Cache::new(),
            label,
            db,
            muted: false,
            shown: -120.0,
            phase,
            routes,
            program,
            hidden: false,
        };
        let out = |label, note, note_on, db, phase| Out {
            cache: canvas::Cache::new(),
            label,
            note,
            note_on,
            hidden: false,
            db,
            muted: false,
            shown: -120.0,
            phase,
        };
        let mut strips = vec![
            strip("Mikro", 0.0, 1.7, [true, true, true], false),
            strip("PC", 0.0, 0.0, [true, true, true], false),
            strip("PS5", 0.0, 3.1, [true, true, true], false),
            strip("Discord", 0.0, 4.4, [true, true, true], true),
            strip("Spotify", 0.0, 5.3, [true, true, true], true),
            strip("Chrome", 0.0, 6.1, [true, true, true], true),
        ];
        strips[1].muted = true;
        strips[5].hidden = true;
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
        let fx = Fx {
            denoise: true,
            low_cut: 80,
            eq: [0.0, 1.5, 0.0],
            gate: true,
            gate_db: -48.0,
        };
        let app = App {
            view: if std::env::var("FADERUST_VIEW").as_deref() == Ok("matrix") {
                View::Matrix
            } else {
                View::Desk
            },
            masters_left: true,
            hide_after: HIDE_AFTER[2],
            dialog,
            strips,
            outs: vec![
                out("PS5", "Lautsprecher (USB Sou…", true, 0.0, 0.4),
                out("Kopfhörer", "Kopfhörer (Galaxy Buds…", true, 0.0, 2.2),
                out("Box", "Freigegeben", false, 0.0, 3.6),
            ],
            fx: vec![fx.clone(); 6],
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
                AppRow {
                    name: "Spotify.exe",
                    device: APP_OUT[0],
                    input: false,
                },
                AppRow {
                    name: "Discord.exe",
                    device: APP_OUT[2],
                    input: false,
                },
            ],
            settings: Settings {
                autostart: true,
                inputs: vec![("Mikrofon".into(), DEV_IN[0]), ("Spiel".into(), DEV_IN[2])],
                outputs: vec![
                    ("PS5".into(), DEV_OUT[0], "60".into(), RELEASE[0]),
                    ("Kopfhörer".into(), DEV_OUT[1], "60".into(), RELEASE[2]),
                ],
            },
            presets: vec!["Spielen".into(), "Streamen".into(), "Nur Kopfhörer".into()],
            new_preset: String::new(),
            win_default: 0,
            win_volume: [80.0, 55.0, 100.0],
            usb_running: true,
            usb_count: 2,
            last: Instant::now(),
            started: Instant::now(),
            shot: std::env::var("FADERUST_SHOT").ok(),
        };
        (app, Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SetView(v) => self.view = v,
            Message::Open(d) => self.dialog = d,
            Message::Edit(e) => self.edit(e),
            Message::Route(i, o) => {
                self.strips[i].routes[o] = !self.strips[i].routes[o];
            }
            Message::Level(i, db) => {
                self.strips[i].db = db;
                self.strips[i].cache.clear();
            }
            Message::Master(o, db) => {
                self.outs[o].db = db;
                self.outs[o].cache.clear();
            }
            Message::Mute(i) => {
                self.strips[i].muted = !self.strips[i].muted;
                self.strips[i].cache.clear();
            }
            Message::MasterMute(o) => {
                self.outs[o].muted = !self.outs[o].muted;
                self.outs[o].cache.clear();
            }
            Message::Frame(now) => {
                let dt = now.duration_since(self.last).as_secs_f32().min(0.1);
                self.last = now;
                let t = now.duration_since(self.started).as_secs_f32();
                let ease = |shown: f32, phase: f32| {
                    // A made-up program level that breathes between -40 and -4 dB.
                    let target = -22.0 + 18.0 * (t * 1.3 + phase).sin() * (t * 0.37 + phase).cos();
                    if target > shown {
                        shown + (target - shown) * (1.0 - (-dt / 0.06).exp())
                    } else {
                        target.max(shown - 12.0 * dt)
                    }
                };
                for s in &mut self.strips {
                    s.shown = ease(s.shown, s.phase);
                }
                for o in &mut self.outs {
                    o.shown = ease(o.shown, o.phase);
                }
                if self.shot.is_some() && t > 1.5 {
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

    fn edit(&mut self, e: Edit) {
        match e {
            Edit::Denoise(i, on) => self.fx[i].denoise = on,
            Edit::LowCut(i, hz) => self.fx[i].low_cut = hz,
            Edit::Eq(i, b, v) => self.fx[i].eq[b] = v,
            Edit::Gate(i, on) => self.fx[i].gate = on,
            Edit::GateDb(i, v) => self.fx[i].gate_db = v,
            Edit::ResetFx(i) => {
                self.fx[i] = Fx {
                    denoise: false,
                    low_cut: 0,
                    eq: [0.0; 3],
                    gate: false,
                    gate_db: -50.0,
                }
            }
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
                if !self.new_preset.trim().is_empty() {
                    self.presets.push(std::mem::take(&mut self.new_preset));
                }
            }
            Edit::WinDefault(i) => self.win_default = i,
            Edit::WinVolume(i, v) => self.win_volume[i] = v.round(),
            Edit::HideOut(o, h) => {
                self.outs[o].hidden = h;
                if !h {
                    // "N ausgeblendet" brings them all back.
                    for o in &mut self.outs {
                        o.hidden = false;
                    }
                }
            }
            Edit::ShowSilent => self.strips.iter_mut().for_each(|s| s.hidden = false),
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
            button(text("v0.6.3").size(12).font(MONO).color(MUTED))
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

    fn note(text_: &'static str, on: bool) -> Element<'static, Message> {
        container(
            text(text_)
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

    fn chips(routes: [bool; 3], strip: Option<usize>, narrow: bool) -> Element<'static, Message> {
        row((0..3).map(|o| {
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
                small("◉", Message::Open(None), 34.0),
            ]
            .spacing(4)
            .into()
        } else {
            small("Effekte", Message::Open(Some(Dialog::Fx(i))), 72.0).into()
        };
        Self::card(
            column![
                Self::chips(s.routes, Some(i), narrow),
                Self::name_label(s.label),
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
                Self::name_label(m.label),
                Self::note(m.note, m.note_on),
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
        let head = row![space().width(110)].extend(OUT_NAMES.iter().map(|o| {
            container(text(o.to_uppercase()).size(12).color(TEXT).font(BOLD))
                .width(110)
                .align_x(Horizontal::Center)
                .into()
        }));
        let mut rows = column![head].spacing(10);
        for (i, s) in self.strips.iter().enumerate() {
            let cells = (0..OUT_NAMES.len()).map(|o| {
                let on = s.routes[o];
                button(
                    text(if on {
                        scale::format(s.db)
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
        .title("Faderust – iced Prototyp")
        .run()
}
