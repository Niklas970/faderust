//! iced prototype of the desk strips: fader with scale, LED meter, readout and
//! mute. Meant to judge look and CPU, not to be wired to the engine.
//!
//! `FADERUST_SHOT=out.png` saves one screenshot after a moment and exits.

mod scale;

use std::time::{Duration, Instant};

use iced::alignment::{Horizontal, Vertical};
use iced::gradient;
use iced::mouse;
use iced::widget::canvas::{self, Canvas, Fill, Frame, Geometry, Path, Stroke, Text};
use iced::widget::{button, column, container, row, text};
use iced::{
    window, Background, Border, Color, Element, Font, Length, Pixels, Point, Rectangle, Renderer,
    Shadow, Size, Subscription, Task, Theme,
};

const ACCENT: Color = Color::from_rgb(0.851, 0.514, 0.310);
const ACCENT_LIGHT: Color = Color::from_rgb(0.918, 0.708, 0.576);
const SIGNAL: Color = Color::from_rgb(0.361, 0.812, 0.416);
const AMBER: Color = Color::from_rgb(0.886, 0.694, 0.235);
const DANGER: Color = Color::from_rgb(0.878, 0.322, 0.294);
const BG: Color = Color::BLACK;
const PANEL_2: Color = Color::from_rgb(0.090, 0.098, 0.110);
const WELL: Color = Color::from_rgb(0.020, 0.020, 0.024);
const BORDER: Color = Color::from_rgb(0.200, 0.216, 0.239);
const TEXT: Color = Color::from_rgb(0.925, 0.933, 0.945);
const MUTED: Color = Color::from_rgb(0.663, 0.686, 0.718);

const STRIP_W: f32 = 124.0;
const STRIP_H: f32 = 330.0;
const PAD: f32 = 18.0;
const MONO: Font = Font::MONOSPACE;

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

struct Strip {
    /// Everything but the meter; redrawn only when the fader or mute changes.
    cache: canvas::Cache,
    label: &'static str,
    db: f32,
    muted: bool,
    /// Simulated source level in dBFS.
    shown: f32,
    phase: f32,
}

struct App {
    strips: Vec<Strip>,
    last: Instant,
    started: Instant,
    shot: Option<String>,
}

#[derive(Debug, Clone)]
enum Message {
    Level(usize, f32),
    Mute(usize),
    Frame(Instant),
    Shot(window::Screenshot),
    Id(Option<window::Id>),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let strip = |label, db, phase| Strip {
            cache: canvas::Cache::new(),
            label,
            db,
            muted: false,
            shown: -120.0,
            phase,
        };
        let mut strips = vec![
            strip("Spiel", -6.0, 0.0),
            strip("Voice", 0.0, 1.7),
            strip("Musik", -14.0, 3.1),
            strip("Browser", -24.0, 4.4),
        ];
        strips[3].muted = true;
        let app = App {
            strips,
            last: Instant::now(),
            started: Instant::now(),
            shot: std::env::var("FADERUST_SHOT").ok(),
        };
        (app, Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Level(i, db) => {
                self.strips[i].db = db;
                self.strips[i].cache.clear();
            }
            Message::Mute(i) => {
                self.strips[i].muted = !self.strips[i].muted;
                self.strips[i].cache.clear();
            }
            Message::Frame(now) => {
                let dt = now.duration_since(self.last).as_secs_f32().min(0.1);
                self.last = now;
                let t = now.duration_since(self.started).as_secs_f32();
                for s in &mut self.strips {
                    // A made-up program level that breathes between -40 and -4 dB.
                    let target =
                        -22.0 + 18.0 * (t * 1.3 + s.phase).sin() * (t * 0.37 + s.phase).cos();
                    s.shown = if target > s.shown {
                        s.shown + (target - s.shown) * (1.0 - (-dt / 0.06).exp())
                    } else {
                        target.max(s.shown - 12.0 * dt)
                    };
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

    fn view(&self) -> Element<'_, Message> {
        let strips =
            self.strips.iter().enumerate().map(|(i, s)| {
                let fader = Canvas::new(Fader {
                    cache: &s.cache,
                    db: s.db,
                    muted: s.muted,
                    level: s.shown,
                    index: i,
                })
                .width(STRIP_W)
                .height(STRIP_H);
                let readout = text(if s.db <= scale::OFF {
                    "−∞ dB".to_string()
                } else {
                    format!("{} dB", scale::format(s.db))
                })
                .font(MONO)
                .size(13)
                .color(if s.muted { MUTED } else { ACCENT_LIGHT });
                let label = container(text(s.label.to_uppercase()).size(12).color(TEXT).font(
                    Font {
                        weight: iced::font::Weight::Bold,
                        ..Font::DEFAULT
                    },
                ))
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
                });
                let mute = button(
                    text("MUTE")
                        .size(12)
                        .color(if s.muted {
                            Color::WHITE
                        } else {
                            rgb(174, 180, 187)
                        })
                        .center(),
                )
                .width(72)
                .height(32)
                .on_press(Message::Mute(i))
                .style(mute_style(s.muted));
                column![label, fader, readout, mute]
                    .spacing(8)
                    .width(STRIP_W)
                    .align_x(Horizontal::Center)
                    .into()
            });
        container(row(strips).spacing(18))
            .padding(24)
            .style(|_| container::Style {
                background: Some(Background::Color(BG)),
                ..container::Style::default()
            })
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(33)).map(Message::Frame)
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

/// Fader track, scale, cap and LED meter of one strip in a single canvas.
struct Fader<'a> {
    cache: &'a canvas::Cache,
    db: f32,
    muted: bool,
    level: f32,
    index: usize,
}

#[derive(Default)]
struct Drag {
    active: bool,
    grab: f32,
    last_click: Option<Instant>,
}

impl Fader<'_> {
    fn top() -> f32 {
        PAD
    }
    fn height() -> f32 {
        STRIP_H - 2.0 * PAD
    }
    fn pos_at(y: f32) -> f32 {
        (1.0 - (y - Self::top()) / Self::height()).clamp(0.0, 1.0)
    }
    fn y_of(pos: f32) -> f32 {
        Self::top() + (1.0 - pos) * Self::height()
    }
}

impl canvas::Program<Message> for Fader<'_> {
    type State = Drag;

    fn update(
        &self,
        state: &mut Drag,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        use iced::Event::Mouse;
        let pos = scale::to_pos(self.db);
        let set = |db: f32| canvas::Action::publish(Message::Level(self.index, scale::clamp(db)));
        match event {
            Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let p = cursor.position_in(bounds)?;
                if p.x > 94.0 {
                    return None;
                }
                let now = Instant::now();
                if state
                    .last_click
                    .is_some_and(|t| now.duration_since(t) < Duration::from_millis(350))
                {
                    state.last_click = None;
                    return Some(set(0.0).and_capture());
                }
                state.last_click = Some(now);
                state.active = true;
                let at = Self::pos_at(p.y);
                let cap_half = 20.0 / Self::height();
                state.grab = if (at - pos).abs() <= cap_half {
                    pos - at
                } else {
                    0.0
                };
                if state.grab == 0.0 {
                    return Some(set(scale::from_pos(at)).and_capture());
                }
                Some(canvas::Action::capture())
            }
            Mouse(mouse::Event::CursorMoved { .. }) if state.active => {
                let p = cursor.position_in(bounds).or_else(|| {
                    cursor
                        .position()
                        .map(|c| Point::new(c.x - bounds.x, c.y - bounds.y))
                })?;
                Some(
                    set(scale::from_pos(
                        (Self::pos_at(p.y) + state.grab).clamp(0.0, 1.0),
                    ))
                    .and_capture(),
                )
            }
            Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.active => {
                state.active = false;
                Some(canvas::Action::capture())
            }
            Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(bounds) => {
                let y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => {
                        *y
                    }
                };
                let step = if y > 0.0 { 1.0 } else { -1.0 };
                let next = if self.db <= scale::OFF {
                    if step > 0.0 {
                        -59.0
                    } else {
                        scale::OFF
                    }
                } else {
                    self.db + step
                };
                Some(set(next).and_capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &Drag,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let body = self
            .cache
            .draw(renderer, bounds.size(), |f| self.draw_body(f));
        let mut f = Frame::new(renderer, bounds.size());
        self.draw_meter(&mut f);
        vec![body, f.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Drag,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.active || cursor.position_in(bounds).is_some_and(|p| p.x <= 94.0) {
            mouse::Interaction::ResizingVertically
        } else {
            mouse::Interaction::default()
        }
    }
}

impl Fader<'_> {
    fn draw_body(&self, f: &mut Frame) {
        let pos = scale::to_pos(self.db);
        let cap_y = Fader::y_of(pos);
        let off = self.db <= scale::OFF;
        let (top, h) = (Fader::top(), Fader::height());

        // Scale: numbers on the left, small marks beside the slot.
        for (d, label) in scale::TICKS {
            let y = Fader::y_of(scale::to_pos(d));
            let zero = d == 0.0;
            f.fill_text(Text {
                content: label.to_string(),
                position: Point::new(26.0, y),
                color: if zero { TEXT } else { MUTED },
                size: Pixels(11.0),
                font: MONO,
                align_x: Horizontal::Right.into(),
                align_y: Vertical::Center,
                ..Text::default()
            });
            let mark = if zero { MUTED } else { BORDER };
            for x in [29.0, 47.0] {
                f.fill_rectangle(Point::new(x, y - 0.5), Size::new(7.0, 1.0), mark);
            }
        }

        // Slot with a recessed look and the accent bar up to the cap.
        let slot = Path::rounded_rectangle(Point::new(53.0, top), Size::new(10.0, h), 5.0.into());
        f.fill(&slot, Fill::from(Color::from_rgb8(5, 5, 5)));
        f.stroke(
            &slot,
            Stroke::default()
                .with_color(rgb(44, 47, 51))
                .with_width(1.0),
        );
        let fill_h = (STRIP_H - PAD - 3.0 - cap_y).max(0.0);
        if fill_h > 0.0 {
            let (core, glow) = if self.muted {
                (rgb(85, 90, 97), None)
            } else {
                (ACCENT, Some(ACCENT))
            };
            if let Some(g) = glow {
                for (grow, alpha) in [(4.0, 0.06), (2.5, 0.10), (1.0, 0.16)] {
                    f.fill(
                        &Path::rounded_rectangle(
                            Point::new(56.0 - grow, cap_y - grow),
                            Size::new(4.0 + 2.0 * grow, fill_h + 2.0 * grow),
                            (2.0 + grow).into(),
                        ),
                        Fill::from(Color { a: alpha, ..g }),
                    );
                }
            }
            f.fill(
                &Path::rounded_rectangle(
                    Point::new(56.0, cap_y),
                    Size::new(4.0, fill_h),
                    2.0.into(),
                ),
                Fill::from(core),
            );
        }

        // Cap: graphite gradient, accent edges, index line and a value window.
        let (cx, cw, ch) = (29.0, 58.0, 40.0);
        let cy = cap_y - ch / 2.0;
        let cap = Path::rounded_rectangle(Point::new(cx, cy), Size::new(cw, ch), 4.0.into());
        for (grow, alpha) in [(6.0, 0.10), (3.0, 0.18), (1.0, 0.30)] {
            f.fill(
                &Path::rounded_rectangle(
                    Point::new(cx - grow, cy - grow + 3.0),
                    Size::new(cw + 2.0 * grow, ch + 2.0 * grow),
                    (4.0 + grow).into(),
                ),
                Fill::from(Color {
                    a: alpha * 0.5,
                    ..Color::BLACK
                }),
            );
        }
        let body = canvas::gradient::Linear::new(Point::new(0.0, cy), Point::new(0.0, cy + ch))
            .add_stop(0.0, rgb(85, 89, 95))
            .add_stop(0.45, rgb(47, 51, 56))
            .add_stop(0.55, rgb(35, 38, 42))
            .add_stop(1.0, rgb(51, 55, 60));
        f.fill(
            &cap,
            Fill {
                style: canvas::Style::Gradient(canvas::Gradient::Linear(body)),
                ..Fill::default()
            },
        );
        f.stroke(
            &cap,
            Stroke::default()
                .with_color(rgb(12, 13, 15))
                .with_width(1.0),
        );
        let edge = if off { rgb(85, 90, 97) } else { ACCENT };
        f.fill_rectangle(Point::new(cx, cy + 1.0), Size::new(3.0, ch - 2.0), edge);
        f.fill_rectangle(
            Point::new(cx + cw - 3.0, cy + 1.0),
            Size::new(3.0, ch - 2.0),
            edge,
        );
        f.fill_rectangle(
            Point::new(cx, cap_y - 1.0),
            Size::new(cw, 2.0),
            if off {
                rgb(133, 139, 147)
            } else {
                ACCENT_LIGHT
            },
        );
        let win = Path::rounded_rectangle(
            Point::new(cx + cw / 2.0 - 23.0, cap_y - 9.0),
            Size::new(46.0, 18.0),
            2.0.into(),
        );
        f.fill(&win, Fill::from(rgb(11, 12, 14)));
        f.fill_text(Text {
            content: scale::format(self.db),
            position: Point::new(cx + cw / 2.0, cap_y),
            color: if off { MUTED } else { rgb(241, 243, 245) },
            size: Pixels(12.0),
            font: MONO,
            align_x: Horizontal::Center.into(),
            align_y: Vertical::Center,
            ..Text::default()
        });
    }

    /// LED meter: 4 px on, 2 px off, signal / amber / clip like the web one.
    fn draw_meter(&self, f: &mut Frame) {
        let (top, h) = (Self::top(), Self::height());
        let (mx, mw) = (98.0, 14.0);
        let frame = Path::rounded_rectangle(Point::new(mx, top), Size::new(mw, h), 2.0.into());
        f.fill(&frame, Fill::from(WELL));
        f.stroke(&frame, Stroke::default().with_color(BORDER).with_width(1.0));
        let shown = if self.muted {
            scale::OFF
        } else {
            self.level + self.db.max(scale::OFF)
        };
        let lit = ((shown + 60.0) / 60.0).clamp(0.0, 1.0);
        let inner = h - 2.0;
        let mut y = top + h - 1.0;
        while y - 4.0 >= top + 1.0 {
            let frac = (top + h - 1.0 - y + 4.0) / inner;
            if frac - 4.0 / inner < lit {
                let color = if frac > 0.95 {
                    DANGER
                } else if frac > 0.80 {
                    AMBER
                } else {
                    SIGNAL
                };
                f.fill_rectangle(
                    Point::new(mx + 1.0, y - 4.0),
                    Size::new(mw - 2.0, 4.0),
                    color,
                );
            }
            y -= 6.0;
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
    Theme::Dark
}

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .theme(theme)
        .window_size(Size::new(24.0 * 2.0 + 4.0 * STRIP_W + 3.0 * 18.0, 500.0))
        .title("Faderust – iced Prototyp")
        .run()
}
