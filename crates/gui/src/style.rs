//! Colours and sizes, taken from the web UI's app.css. The colours follow the
//! user's theme (light or dark, accent, signal), so they are functions that
//! read the current palette.

use std::sync::RwLock;

use iced::{Color, Font};

pub const ACCENTS: [(&str, &str, &str); 8] = [
    ("Kupfer", "Copper", "#d9834f"),
    ("Bernstein", "Amber", "#e0a43a"),
    ("Rot", "Red", "#e0605a"),
    ("Magenta", "Magenta", "#d9609e"),
    ("Violett", "Violet", "#9b7ce8"),
    ("Blau", "Blue", "#4f9bd9"),
    ("Grün", "Green", "#5fb56a"),
    ("Silber", "Silver", "#a7b0ba"),
];

pub const SIGNALS: [(&str, &str, &str); 5] = [
    ("Grün", "Green", "#5ccf6a"),
    ("Türkis", "Teal", "#3fc8b4"),
    ("Blau", "Blue", "#4fb3e8"),
    ("Gelb", "Yellow", "#e6d04a"),
    ("Weiß", "White", "#d8dde3"),
];

pub const DEFAULT_ACCENT: &str = "#d9834f";
pub const DEFAULT_SIGNAL: &str = "#5ccf6a";

#[derive(Clone, Copy)]
struct Palette {
    accent: Color,
    accent_light: Color,
    signal: Color,
    amber: Color,
    danger: Color,
    bg: Color,
    panel: Color,
    panel_2: Color,
    well: Color,
    border: Color,
    text: Color,
    muted: Color,
}

static PALETTE: RwLock<Option<Palette>> = RwLock::new(None);

fn parse_hex(h: &str) -> Option<Color> {
    let h = h.strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    Some(Color::from_rgb8((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    Color::from_rgb(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
    )
}

/// Whether Windows prefers light apps (dark everywhere else).
fn system_light() -> bool {
    #[cfg(windows)]
    {
        let out = faderust_core::proc::hidden("reg")
            .args([
                "query",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
                "/v",
                "AppsUseLightTheme",
            ])
            .output();
        if let Ok(o) = out {
            return String::from_utf8_lossy(&o.stdout).contains("0x1");
        }
    }
    false
}

/// Sets the palette; `mode` is "dark", "light" or "system".
pub fn set_theme(mode: &str, accent: &str, signal: &str) {
    let light = match mode {
        "light" => true,
        "dark" => false,
        _ => system_light(),
    };
    let accent = parse_hex(accent).unwrap_or_else(|| parse_hex(DEFAULT_ACCENT).unwrap());
    let signal = parse_hex(signal).unwrap_or_else(|| parse_hex(DEFAULT_SIGNAL).unwrap());
    let black = Color::BLACK;
    let white = Color::WHITE;
    let pal = if light {
        Palette {
            accent: mix(accent, black, 0.2),
            accent_light: accent,
            signal: mix(signal, black, 0.3),
            amber: rgb(183, 134, 15),
            danger: rgb(197, 59, 59),
            bg: rgb(228, 230, 233),
            panel: rgb(243, 244, 246),
            panel_2: rgb(233, 235, 238),
            well: rgb(42, 45, 50),
            border: rgb(170, 176, 184),
            text: rgb(18, 20, 23),
            muted: rgb(68, 75, 83),
        }
    } else {
        Palette {
            accent,
            accent_light: mix(accent, white, 0.3),
            signal,
            amber: rgb(226, 177, 60),
            danger: rgb(224, 82, 75),
            bg: Color::BLACK,
            panel: rgb(12, 13, 15),
            panel_2: rgb(23, 25, 28),
            well: rgb(5, 5, 6),
            border: rgb(51, 55, 61),
            text: rgb(236, 238, 241),
            muted: rgb(169, 175, 183),
        }
    };
    if let Ok(mut p) = PALETTE.write() {
        *p = Some(pal);
    }
}

fn pal() -> Palette {
    if let Ok(p) = PALETTE.read() {
        if let Some(p) = *p {
            return p;
        }
    }
    set_theme("dark", DEFAULT_ACCENT, DEFAULT_SIGNAL);
    PALETTE.read().ok().and_then(|p| *p).unwrap()
}

pub fn hex(c: &str) -> Color {
    parse_hex(c).unwrap_or(Color::WHITE)
}

pub fn accent() -> Color {
    pal().accent
}
pub fn accent_light() -> Color {
    pal().accent_light
}
pub fn signal() -> Color {
    pal().signal
}
pub fn amber() -> Color {
    pal().amber
}
pub fn danger() -> Color {
    pal().danger
}
pub fn bg() -> Color {
    pal().bg
}
pub fn panel() -> Color {
    pal().panel
}
pub fn panel_2() -> Color {
    pal().panel_2
}
pub fn well() -> Color {
    pal().well
}
pub fn border() -> Color {
    pal().border
}
pub fn text_color() -> Color {
    pal().text
}
pub fn muted_color() -> Color {
    pal().muted
}

pub const STRIP_W: f32 = 124.0;
pub const PAD: f32 = 18.0;
pub const MONO: Font = Font::MONOSPACE;

pub fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

pub const BOLD: Font = Font {
    weight: iced::font::Weight::Bold,
    ..Font::DEFAULT
};
