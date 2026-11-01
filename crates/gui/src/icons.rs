//! Small vector icons for the title bar, drawn on a canvas so they look the
//! same on every Windows font.

use iced::mouse;
use iced::widget::canvas::{self, Fill, Frame, Geometry, Path, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

#[derive(Clone, Copy)]
pub enum Glyph {
    /// A slim window with narrow strips.
    Compact,
    /// Two stacked windows: the window stays on top.
    Pin,
    /// A loudspeaker with sound waves (Windows sound).
    Speaker,
}

pub struct Icon {
    pub glyph: Glyph,
    pub color: Color,
    /// Drawn filled (the option is on).
    pub on: bool,
}

impl<M> canvas::Program<M> for Icon {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        // Designed on 16 x 16, centred in whatever the canvas offers.
        let k = (bounds.width.min(bounds.height) / 16.0).max(0.1);
        let ox = (bounds.width - 16.0 * k) / 2.0;
        let oy = (bounds.height - 16.0 * k) / 2.0;
        let p = |x: f32, y: f32| Point::new(ox + x * k, oy + y * k);
        let line = Stroke::default()
            .with_color(self.color)
            .with_width(1.5 * k)
            .with_line_cap(canvas::LineCap::Round)
            .with_line_join(canvas::LineJoin::Round);
        match self.glyph {
            Glyph::Compact => {
                let frame = Path::rounded_rectangle(
                    p(1.5, 2.5),
                    Size::new(13.0 * k, 11.0 * k),
                    (2.0 * k).into(),
                );
                f.stroke(&frame, line);
                for x in [5.0, 9.0] {
                    let bar = Path::rounded_rectangle(
                        p(x, 5.5),
                        Size::new(2.0 * k, 5.0 * k),
                        (1.0 * k).into(),
                    );
                    f.fill(&bar, Fill::from(self.color));
                }
            }
            Glyph::Pin => {
                // Two windows: the front one stays on top of the one behind.
                let mut dim = self.color;
                dim.a *= 0.6;
                let back = Path::new(|b| {
                    b.move_to(p(2.2, 5.0));
                    b.line_to(p(2.2, 12.3));
                    b.line_to(p(3.2, 13.3));
                    b.line_to(p(10.0, 13.3));
                });
                f.stroke(&back, line.with_color(dim));
                let front = Path::rounded_rectangle(
                    p(5.5, 2.0),
                    Size::new(9.0 * k, 7.5 * k),
                    (1.8 * k).into(),
                );
                if self.on {
                    f.fill(&front, Fill::from(self.color));
                }
                f.stroke(&front, line);
            }
            Glyph::Speaker => {
                // Cabinet with a cone, and two waves.
                let body = Path::new(|b| {
                    b.move_to(p(1.5, 6.0));
                    b.line_to(p(4.5, 6.0));
                    b.line_to(p(8.5, 2.5));
                    b.line_to(p(8.5, 13.5));
                    b.line_to(p(4.5, 10.0));
                    b.line_to(p(1.5, 10.0));
                    b.close();
                });
                if self.on {
                    f.fill(&body, Fill::from(self.color));
                }
                f.stroke(&body, line);
                for (r, a) in [(2.6_f32, 1.0_f32), (5.2, 0.7)] {
                    let wave = Path::new(|b| {
                        b.arc(canvas::path::Arc {
                            center: p(8.5, 8.0),
                            radius: r * k,
                            start_angle: iced::Radians(-0.9),
                            end_angle: iced::Radians(0.9),
                        });
                    });
                    let mut c = self.color;
                    c.a *= a;
                    f.stroke(&wave, line.with_color(c));
                }
            }
        }
        vec![f.into_geometry()]
    }
}
