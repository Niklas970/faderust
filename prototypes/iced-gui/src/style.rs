//! Colours and sizes, taken from the web UI's app.css (dark mode).

use iced::{Color, Font};

pub const ACCENT: Color = Color::from_rgb(0.851, 0.514, 0.310);
pub const ACCENT_LIGHT: Color = Color::from_rgb(0.918, 0.708, 0.576);
pub const SIGNAL: Color = Color::from_rgb(0.361, 0.812, 0.416);
pub const AMBER: Color = Color::from_rgb(0.886, 0.694, 0.235);
pub const DANGER: Color = Color::from_rgb(0.878, 0.322, 0.294);
pub const BG: Color = Color::BLACK;
pub const PANEL: Color = Color::from_rgb(0.047, 0.051, 0.059);
pub const PANEL_2: Color = Color::from_rgb(0.090, 0.098, 0.110);
pub const WELL: Color = Color::from_rgb(0.020, 0.020, 0.024);
pub const BORDER: Color = Color::from_rgb(0.200, 0.216, 0.239);
pub const TEXT: Color = Color::from_rgb(0.925, 0.933, 0.945);
pub const MUTED: Color = Color::from_rgb(0.663, 0.686, 0.718);

pub const STRIP_W: f32 = 124.0;
pub const STRIP_H: f32 = 300.0;
pub const PAD: f32 = 18.0;
pub const MONO: Font = Font::MONOSPACE;

pub fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

pub const BOLD: Font = Font {
    weight: iced::font::Weight::Bold,
    ..Font::DEFAULT
};
