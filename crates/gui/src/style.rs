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

pub fn mix(a: Color, b: Color, t: f32) -> Color {
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

/// Colour of the one-pixel edge around the window: clearly visible on both
/// the dark and the light design.
pub fn frame_color() -> Color {
    mix(border(), accent(), 0.35)
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
            border: rgb(140, 147, 156),
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
            panel: Color::BLACK,
            panel_2: rgb(22, 23, 26),
            well: Color::BLACK,
            border: rgb(72, 77, 86),
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

/// "#rrggbb" or "rrggbb" typed by the user, as "#rrggbb" in lower case.
pub fn parse_hex_input(t: &str) -> Option<String> {
    let t = t.trim().trim_start_matches('#');
    (t.len() == 6 && t.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| format!("#{}", t.to_lowercase()))
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
/// A quiet outline for groups and cards.
pub fn hairline() -> Color {
    mix(panel(), border(), 0.5)
}
pub fn muted_color() -> Color {
    pal().muted
}

pub const STRIP_W: f32 = 136.0;
pub const PAD: f32 = 18.0;
pub const MONO: Font = Font::MONOSPACE;

pub fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

pub const BOLD: Font = Font {
    weight: iced::font::Weight::Bold,
    ..Font::DEFAULT
};

// --- widget looks -----------------------------------------------------------

use iced::widget::{button, checkbox, pick_list, text_input};
use iced::{gradient, Background, Border, Shadow, Theme};

/// Readable text on the accent colour (black on a pale accent).
pub fn on_accent() -> Color {
    let c = accent();
    let l = 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
    if l > 0.6 {
        Color::from_rgb(0.07, 0.08, 0.09)
    } else {
        Color::WHITE
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Look {
    /// The accent colour: the main action or an option that is on.
    Primary,
    /// A plain raised key.
    Secondary,
    /// Red: an option that is on and means "off" (mute).
    Danger,
}

pub fn vertical(top: Color, bottom: Color) -> Background {
    Background::Gradient(
        gradient::Linear::new(iced::Radians(std::f32::consts::PI))
            .add_stop(0.0, top)
            .add_stop(1.0, bottom)
            .into(),
    )
}

/// Text colour that goes with a `Look`.
pub fn look_text(look: Look) -> Color {
    match look {
        Look::Primary => on_accent(),
        Look::Secondary => text_color(),
        Look::Danger => Color::WHITE,
    }
}

/// Top colour, bottom colour and edge of a key in a given state.
fn key_colors(look: Look, status: button::Status) -> (Color, Color, Color) {
    let base = match look {
        Look::Primary => accent(),
        Look::Secondary => panel_2(),
        Look::Danger => danger(),
    };
    let lift = if text_color().r > 0.5 {
        Color::WHITE
    } else {
        Color::BLACK
    };
    match status {
        button::Status::Pressed => (
            mix(base, Color::BLACK, 0.18),
            mix(base, Color::BLACK, 0.08),
            accent(),
        ),
        button::Status::Hovered => (mix(base, lift, 0.16), mix(base, lift, 0.06), accent_light()),
        button::Status::Disabled => (mix(base, bg(), 0.5), mix(base, bg(), 0.5), border()),
        button::Status::Active => (
            mix(base, lift, 0.09),
            base,
            if look == Look::Secondary {
                border()
            } else {
                mix(base, Color::BLACK, 0.3)
            },
        ),
    }
}

fn key_style(look: Look, status: button::Status, flat: bool) -> button::Style {
    let (top, bottom, edge) = key_colors(look, status);
    button::Style {
        // Dialog rows of many keys render wrongly with gradients on some
        // graphics drivers, so those use one solid colour.
        background: Some(if flat {
            Background::Color(
                if look == Look::Secondary && status == button::Status::Active {
                    // Plain black on dark, plain white on light.
                    if text_color().r > 0.5 {
                        Color::BLACK
                    } else {
                        Color::WHITE
                    }
                } else {
                    mix(top, bottom, 0.5)
                },
            )
        } else {
            vertical(top, bottom)
        }),
        text_color: look_text(look),
        border: Border {
            color: edge,
            width: 1.0,
            radius: 5.0.into(),
        },
        shadow: Shadow {
            color: Color {
                a: if flat { 0.0 } else { 0.25 },
                ..Color::BLACK
            },
            offset: iced::Vector::new(0.0, 1.0),
            blur_radius: 2.0,
        },
        snap: true,
    }
}

/// A raised key with a soft gradient, a clear edge and hover/press states.
pub fn key(look: Look) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| key_style(look, status, false)
}

/// Like `key`, but with a solid fill and no shadow (dialogs).
pub fn flat(look: Look) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| key_style(look, status, true)
}

pub fn pick<'a, T, L, V, M>(
    options: L,
    selected: Option<V>,
    on_select: impl Fn(T) -> M + 'a,
) -> pick_list::PickList<'a, T, L, V, M>
where
    T: ToString + PartialEq + Clone + 'a,
    L: std::borrow::Borrow<[T]> + 'a,
    V: std::borrow::Borrow<T> + 'a,
    M: Clone,
{
    pick_list(options, selected, on_select)
        .style(|_, status| {
            let hot = matches!(
                status,
                pick_list::Status::Hovered | pick_list::Status::Opened { .. }
            );
            pick_list::Style {
                text_color: text_color(),
                placeholder_color: muted_color(),
                handle_color: muted_color(),
                background: Background::Color(well_light()),
                border: Border {
                    color: if hot { accent() } else { border() },
                    width: 1.0,
                    radius: 4.0.into(),
                },
            }
        })
        .menu_style(|_| iced::overlay::menu::Style {
            background: Background::Color(panel()),
            border: Border {
                color: accent(),
                width: 1.0,
                radius: 4.0.into(),
            },
            text_color: text_color(),
            selected_text_color: on_accent(),
            selected_background: Background::Color(accent()),
            shadow: Shadow {
                color: Color {
                    a: 0.45,
                    ..Color::BLACK
                },
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 10.0,
            },
        })
}

/// Fields: a light recess so they read against the dialog.
fn well_light() -> Color {
    mix(
        panel(),
        if text_color().r > 0.5 {
            Color::WHITE
        } else {
            Color::BLACK
        },
        0.06,
    )
}

pub fn input<'a, M: Clone + 'a>(placeholder: &str, value: &str) -> text_input::TextInput<'a, M> {
    text_input(placeholder, value).style(|_, status| {
        let focused = matches!(status, text_input::Status::Focused { .. });
        let hot = focused || matches!(status, text_input::Status::Hovered);
        text_input::Style {
            background: Background::Color(well_light()),
            border: Border {
                color: if focused {
                    accent()
                } else if hot {
                    accent_light()
                } else {
                    border()
                },
                width: 1.0,
                radius: 4.0.into(),
            },
            icon: muted_color(),
            placeholder: muted_color(),
            value: text_color(),
            selection: Color {
                a: 0.35,
                ..accent()
            },
        }
    })
}

pub fn check<'a, M: 'a>(on: bool) -> checkbox::Checkbox<'a, M> {
    checkbox(on).style(|_, status| {
        let hot = matches!(status, checkbox::Status::Hovered { .. });
        let checked = matches!(
            status,
            checkbox::Status::Active { is_checked: true }
                | checkbox::Status::Hovered { is_checked: true }
        );
        checkbox::Style {
            background: Background::Color(if checked { accent() } else { well_light() }),
            icon_color: on_accent(),
            border: Border {
                color: if checked || hot { accent() } else { border() },
                width: 1.0,
                radius: 3.0.into(),
            },
            text_color: Some(text_color()),
        }
    })
}

/// A vertical scroll area whose bar is visible and keeps its distance.
pub fn scroll<'a, M: 'a>(
    content: impl Into<iced::Element<'a, M>>,
) -> iced::widget::Scrollable<'a, M> {
    use iced::widget::scrollable;
    scrollable(content)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(8)
                .scroller_width(8)
                .margin(2),
        ))
        .style(scroll_style)
}

pub fn scroll_style(
    _: &Theme,
    status: iced::widget::scrollable::Status,
) -> iced::widget::scrollable::Style {
    use iced::widget::scrollable;
    let hot = matches!(
        status,
        scrollable::Status::Hovered {
            is_vertical_scrollbar_hovered: true,
            ..
        } | scrollable::Status::Dragged { .. }
    );
    let rail = scrollable::Rail {
        background: Some(Background::Color(mix(panel(), border(), 0.25))),
        border: Border {
            radius: 4.0.into(),
            ..Border::default()
        },
        scroller: scrollable::Scroller {
            background: Background::Color(if hot {
                accent()
            } else {
                mix(panel(), muted_color(), 0.55)
            }),
            border: Border {
                radius: 4.0.into(),
                ..Border::default()
            },
        },
    };
    scrollable::Style {
        container: iced::widget::container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: Background::Color(panel()),
            border: Border::default(),
            shadow: Shadow::default(),
            icon: text_color(),
        },
    }
}

/// Hue in degrees (0..360), saturation and value 0..1.
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    Color::from_rgb(r + m, g + m, b + m)
}

pub fn rgb_to_hsv(c: Color) -> (f32, f32, f32) {
    let max = c.r.max(c.g).max(c.b);
    let min = c.r.min(c.g).min(c.b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == c.r {
        60.0 * ((c.g - c.b) / d).rem_euclid(6.0)
    } else if max == c.g {
        60.0 * ((c.b - c.r) / d + 2.0)
    } else {
        60.0 * ((c.r - c.g) / d + 4.0)
    };
    (h, if max == 0.0 { 0.0 } else { d / max }, max)
}

/// "#rrggbb" for a colour.
pub fn to_hex(c: Color) -> String {
    let b = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", b(c.r), b(c.g), b(c.b))
}
