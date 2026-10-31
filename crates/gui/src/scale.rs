//! Gain scale of the faders, same taper as `ui/src/lib/scale.js`.

pub const OFF: f32 = -120.0;

pub const TICKS: [(f32, &str); 8] = [
    (12.0, "+12"),
    (6.0, "+6"),
    (0.0, "0"),
    (-6.0, "-6"),
    (-12.0, "-12"),
    (-24.0, "-24"),
    (-48.0, "-48"),
    (OFF, "−∞"),
];

pub fn to_pos(d: f32) -> f32 {
    if d <= -60.0 {
        0.0
    } else if d < -24.0 {
        0.05 + (d + 60.0) / 36.0 * 0.2
    } else {
        0.25 + (d.min(12.0) + 24.0) / 36.0 * 0.75
    }
}

pub fn from_pos(p: f32) -> f32 {
    if p <= 0.05 {
        OFF
    } else if p < 0.25 {
        -60.0 + (p - 0.05) / 0.2 * 36.0
    } else {
        -24.0 + (p.min(1.0) - 0.25) / 0.75 * 36.0
    }
}

pub fn clamp(d: f32) -> f32 {
    if d <= -60.0 {
        OFF
    } else {
        ((d * 2.0).round() / 2.0).min(12.0)
    }
}

pub fn format(d: f32) -> String {
    if d <= OFF {
        "−∞".into()
    } else {
        format!("{}{:.1}", if d > 0.0 { "+" } else { "" }, d)
    }
}

use faderust_core::config::Gain;

/// Strip view: one fader for an input on all outputs it plays on. It shows
/// the loudest of them; moving it shifts all by the same amount.
pub fn strip_value(row: &[Gain]) -> f32 {
    let on: Vec<&Gain> = row.iter().filter(|g| !g.muted).collect();
    let list: Vec<&Gain> = if on.is_empty() {
        row.iter().collect()
    } else {
        on
    };
    list.iter().fold(OFF, |m, g| m.max(g.db))
}

/// `(output, dB)` pairs that move the strip `row` to `next`.
pub fn strip_changes(row: &[Gain], next: f32) -> Vec<(usize, f32)> {
    let base = strip_value(row);
    let delta = next - base;
    row.iter()
        .enumerate()
        .filter(|(_, g)| !g.muted)
        .map(|(o, g)| {
            if next <= OFF {
                (o, OFF)
            } else if g.db <= OFF || base <= OFF {
                (o, next)
            } else {
                (o, clamp(g.db + delta))
            }
        })
        .collect()
}
