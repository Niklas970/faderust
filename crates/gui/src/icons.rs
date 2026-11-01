//! Small vector icons for the title bar, drawn on a canvas so they look the
//! same on every Windows font.

use iced::mouse;
use iced::widget::canvas::{self, Fill, Frame, Geometry, Path, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

#[derive(Clone, Copy)]
pub enum Glyph {
    /// A slim window with narrow strips.
    Compact,
    /// A push pin: the window stays on top.
    Pin,
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
                // A push pin, tilted: head, collar, tapered body, needle.
                f.with_save(|f| {
                    f.translate(iced::Vector::new(ox + 8.0 * k, oy + 8.0 * k));
                    f.rotate(0.65);
                    let q = |x: f32, y: f32| Point::new((x - 8.0) * k, (y - 8.0) * k);
                    let head = Path::rounded_rectangle(
                        q(6.0, 1.5),
                        Size::new(4.0 * k, 2.5 * k),
                        (1.0 * k).into(),
                    );
                    let collar = Path::rounded_rectangle(
                        q(4.8, 4.0),
                        Size::new(6.4 * k, 1.6 * k),
                        (0.8 * k).into(),
                    );
                    let body = Path::new(|b| {
                        b.move_to(q(6.0, 5.6));
                        b.line_to(q(10.0, 5.6));
                        b.line_to(q(9.0, 10.0));
                        b.line_to(q(7.0, 10.0));
                        b.close();
                    });
                    for shape in [&head, &collar, &body] {
                        if self.on {
                            f.fill(shape, Fill::from(self.color));
                        }
                        f.stroke(shape, line);
                    }
                    f.stroke(&Path::line(q(8.0, 10.0), q(8.0, 14.8)), line);
                });
            }
        }
        vec![f.into_geometry()]
    }
}
