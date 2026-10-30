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
