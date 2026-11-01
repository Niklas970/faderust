//! A colour picker: a square for saturation and brightness and a strip for
//! the hue, drawn on one canvas.

use iced::mouse;
use iced::widget::canvas::{self, Fill, Frame, Geometry, Path, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

use crate::style::{hsv_to_rgb, mix};
use crate::{Edit, Message, Target};

pub const W: f32 = 264.0;
pub const H: f32 = 150.0;
const SV_W: f32 = 222.0;
const HUE_X: f32 = 236.0;
const HUE_W: f32 = 24.0;

pub struct Picker {
    pub hsv: (f32, f32, f32),
    pub target: Target,
}

#[derive(Clone, Copy, PartialEq)]
enum Zone {
    Sv,
    Hue,
}

#[derive(Default)]
pub struct Drag(Option<Zone>);

impl Picker {
    fn at(&self, zone: Zone, p: Point) -> Message {
        let (h, s, v) = self.hsv;
        let (h, s, v) = match zone {
            Zone::Sv => (
                h,
                (p.x / SV_W).clamp(0.0, 1.0),
                1.0 - (p.y / H).clamp(0.0, 1.0),
            ),
            Zone::Hue => ((p.y / H).clamp(0.0, 1.0) * 360.0, s, v),
        };
        Message::Edit(Edit::Hsv(self.target, h, s, v))
    }
}

impl canvas::Program<Message> for Picker {
    type State = Drag;

    fn update(
        &self,
        state: &mut Drag,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        use iced::Event::Mouse;
        let local = |cursor: mouse::Cursor| {
            cursor.position_in(bounds).or_else(|| {
                cursor
                    .position()
                    .map(|c| Point::new(c.x - bounds.x, c.y - bounds.y))
            })
        };
        match event {
            Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let p = cursor.position_in(bounds)?;
                let zone = if p.x < SV_W + 7.0 {
                    Zone::Sv
                } else if p.x >= HUE_X - 4.0 {
                    Zone::Hue
                } else {
                    return None;
                };
                state.0 = Some(zone);
                Some(canvas::Action::publish(self.at(zone, p)).and_capture())
            }
            Mouse(mouse::Event::CursorMoved { .. }) => {
                let zone = state.0?;
                let p = local(cursor)?;
                Some(canvas::Action::publish(self.at(zone, p)).and_capture())
            }
            Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.0.is_some() => {
                state.0 = None;
                Some(canvas::Action::publish(Message::Edit(Edit::HsvDone)).and_capture())
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
        let mut f = Frame::new(renderer, bounds.size());
        let (h, s, v) = self.hsv;
        let hue = hsv_to_rgb(h, 1.0, 1.0);

        // Saturation to the right, brightness upward.
        let sv = Path::rounded_rectangle(Point::ORIGIN, Size::new(SV_W, H), 6.0.into());
        let across = canvas::gradient::Linear::new(Point::ORIGIN, Point::new(SV_W, 0.0))
            .add_stop(0.0, Color::WHITE)
            .add_stop(1.0, hue);
        f.fill(&sv, Fill::from(canvas::Gradient::Linear(across)));
        let down = canvas::gradient::Linear::new(Point::ORIGIN, Point::new(0.0, H))
            .add_stop(0.0, Color::TRANSPARENT)
            .add_stop(1.0, Color::BLACK);
        f.fill(&sv, Fill::from(canvas::Gradient::Linear(down)));
        f.stroke(
            &sv,
            Stroke::default()
                .with_color(Color {
                    a: 0.35,
                    ..Color::BLACK
                })
                .with_width(1.0),
        );
        let (cx, cy) = (s * SV_W, (1.0 - v) * H);
        let ring = Path::circle(Point::new(cx, cy), 7.0);
        f.stroke(
            &ring,
            Stroke::default().with_color(Color::BLACK).with_width(3.0),
        );
        f.stroke(
            &ring,
            Stroke::default().with_color(Color::WHITE).with_width(1.5),
        );

        // Hue strip.
        let strip =
            Path::rounded_rectangle(Point::new(HUE_X, 0.0), Size::new(HUE_W, H), 6.0.into());
        let mut rainbow = canvas::gradient::Linear::new(Point::new(0.0, 0.0), Point::new(0.0, H));
        for i in 0..=6 {
            rainbow = rainbow.add_stop(i as f32 / 6.0, hsv_to_rgb(i as f32 * 60.0, 1.0, 1.0));
        }
        f.fill(&strip, Fill::from(canvas::Gradient::Linear(rainbow)));
        let y = (h / 360.0).clamp(0.0, 1.0) * H;
        let mark = Path::rounded_rectangle(
            Point::new(HUE_X - 3.0, y - 3.0),
            Size::new(HUE_W + 6.0, 6.0),
            3.0.into(),
        );
        f.stroke(
            &mark,
            Stroke::default().with_color(Color::BLACK).with_width(3.0),
        );
        f.stroke(
            &mark,
            Stroke::default()
                .with_color(mix(Color::WHITE, hue, 0.0))
                .with_width(1.5),
        );
        vec![f.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Drag,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}
