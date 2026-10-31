//! Fader with scale, cap and LED meter, drawn in one canvas.

use std::time::{Duration, Instant};

use iced::alignment::{Horizontal, Vertical};
use iced::mouse;
use iced::widget::canvas::{self, Fill, Frame, Geometry, Path, Stroke, Text};
use iced::{Color, Pixels, Point, Rectangle, Renderer, Size, Theme};

use crate::scale;
use crate::style::*;
use crate::Message;

/// Which gain a fader sets.
#[derive(Debug, Clone, Copy)]
pub enum Kind {
    Input(usize),
    Master(usize),
}

impl Kind {
    fn message(self, db: f32) -> Message {
        match self {
            Kind::Input(i) => Message::Level(i, db),
            Kind::Master(o) => Message::Master(o, db),
        }
    }
}

/// Fader track, scale, cap and LED meter of one strip in a single canvas.
pub struct Fader<'a> {
    pub cache: &'a canvas::Cache,
    pub db: f32,
    pub muted: bool,
    pub level: f32,
    /// The level already includes this fader (an output's master).
    pub post_fader: bool,
    /// Without the scale, for the narrower program strips.
    pub narrow: bool,
    pub kind: Kind,
}

#[derive(Default)]
pub struct Drag {
    active: bool,
    grab: f32,
    last_click: Option<Instant>,
}

impl Fader<'_> {
    /// Width of the track; the meter lies to the right of it.
    fn hit_width(&self) -> f32 {
        if self.narrow {
            62.0
        } else {
            94.0
        }
    }
    fn top() -> f32 {
        PAD
    }
    /// Travel of the fader inside a canvas `total` high.
    fn height(total: f32) -> f32 {
        total - 2.0 * PAD
    }
    fn pos_at(y: f32, total: f32) -> f32 {
        (1.0 - (y - Self::top()) / Self::height(total)).clamp(0.0, 1.0)
    }
    fn y_of(pos: f32, total: f32) -> f32 {
        Self::top() + (1.0 - pos) * Self::height(total)
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
        let set = |db: f32| canvas::Action::publish(self.kind.message(scale::clamp(db)));
        match event {
            Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let p = cursor.position_in(bounds)?;
                if p.x > self.hit_width() {
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
                let at = Self::pos_at(p.y, bounds.height);
                let cap_half = 20.0 / Self::height(bounds.height);
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
                        (Self::pos_at(p.y, bounds.height) + state.grab).clamp(0.0, 1.0),
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
        if state.active
            || cursor
                .position_in(bounds)
                .is_some_and(|p| p.x <= self.hit_width())
        {
            mouse::Interaction::ResizingVertically
        } else {
            mouse::Interaction::default()
        }
    }
}

impl Fader<'_> {
    fn draw_body(&self, f: &mut Frame) {
        let total = f.height();
        let pos = scale::to_pos(self.db);
        let cap_y = Self::y_of(pos, total);
        let off = self.db <= scale::OFF;
        let (top, h) = (Self::top(), Self::height(total));

        // Narrow: no scale, the cap and slot move left (58 px track).
        let (sx, cx) = if self.narrow {
            (24.0, 0.0)
        } else {
            (53.0, 29.0)
        };
        // Scale: numbers on the left, small marks beside the slot.
        for (d, label) in scale::TICKS.iter().copied().filter(|_| !self.narrow) {
            let y = Self::y_of(scale::to_pos(d), total);
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
        let slot = Path::rounded_rectangle(Point::new(sx, top), Size::new(10.0, h), 5.0.into());
        f.fill(&slot, Fill::from(Color::from_rgb8(5, 5, 5)));
        f.stroke(
            &slot,
            Stroke::default()
                .with_color(rgb(44, 47, 51))
                .with_width(1.0),
        );
        let fill_h = (total - PAD - 3.0 - cap_y).max(0.0);
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
                            Point::new(sx + 3.0 - grow, cap_y - grow),
                            Size::new(4.0 + 2.0 * grow, fill_h + 2.0 * grow),
                            (2.0 + grow).into(),
                        ),
                        Fill::from(Color { a: alpha, ..g }),
                    );
                }
            }
            f.fill(
                &Path::rounded_rectangle(
                    Point::new(sx + 3.0, cap_y),
                    Size::new(4.0, fill_h),
                    2.0.into(),
                ),
                Fill::from(core),
            );
        }

        // Cap: graphite gradient, accent edges, index line and a value window.
        let (cw, ch) = (58.0, 40.0);
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
        let total = f.height();
        let (top, h) = (Self::top(), Self::height(total));
        let (mx, mw) = (if self.narrow { 64.0 } else { 98.0 }, 14.0);
        let frame = Path::rounded_rectangle(Point::new(mx, top), Size::new(mw, h), 2.0.into());
        f.fill(&frame, Fill::from(WELL));
        f.stroke(&frame, Stroke::default().with_color(BORDER).with_width(1.0));
        let shown = if self.muted {
            scale::OFF
        } else if self.post_fader {
            self.level
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
