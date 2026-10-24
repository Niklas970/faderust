//! User configuration, stored as TOML in the app's config directory.

use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputEntry {
    pub label: String,
    /// Device id or part of its name.
    pub device: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputEntry {
    pub label: String,
    /// Device id or part of its name.
    pub device: String,
    #[serde(default = "default_buffer_ms")]
    pub buffer_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Gain {
    pub db: f32,
    pub muted: bool,
}

impl Default for Gain {
    fn default() -> Self {
        Self {
            db: 0.0,
            muted: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default)]
    pub inputs: Vec<InputEntry>,
    #[serde(default)]
    pub outputs: Vec<OutputEntry>,
    /// `gains[input][output]`.
    #[serde(default)]
    pub gains: Vec<Vec<Gain>>,
}

fn default_buffer_ms() -> u32 {
    60
}

/// Buffers below this underrun constantly; above it latency gets silly.
pub const BUFFER_RANGE: std::ops::RangeInclusive<u32> = 10..=500;

impl Config {
    /// Sizes the gain matrix to the device lists and clamps values.
    pub fn normalized(mut self) -> Self {
        let outputs = self.outputs.len();
        self.gains.resize_with(self.inputs.len(), Vec::new);
        for row in &mut self.gains {
            row.resize_with(outputs, Gain::default);
            for g in row {
                if !g.db.is_finite() {
                    g.db = -120.0;
                }
                g.db = g.db.clamp(-120.0, 12.0);
            }
        }
        for o in &mut self.outputs {
            o.buffer_ms = o
                .buffer_ms
                .clamp(*BUFFER_RANGE.start(), *BUFFER_RANGE.end());
        }
        self
    }

    pub fn load(path: &Path) -> io::Result<Option<Self>> {
        match fs::read_to_string(path) {
            Ok(text) => toml::from_str::<Config>(&text)
                .map(|c| Some(c.normalized()))
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Writes atomically, so a crash never leaves half a file.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let text = toml::to_string_pretty(self).map_err(io::Error::other)?;
        let tmp = path.with_extension("toml.tmp");
        fs::write(&tmp, text)?;
        fs::rename(&tmp, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Config {
        Config {
            inputs: vec![InputEntry {
                label: "Mikro".into(),
                device: "USB".into(),
            }],
            outputs: vec![
                OutputEntry {
                    label: "PS5".into(),
                    device: "USB Sound".into(),
                    buffer_ms: 30,
                },
                OutputEntry {
                    label: "Kopfhörer".into(),
                    device: "Buds".into(),
                    buffer_ms: 80,
                },
            ],
            gains: vec![],
        }
        .normalized()
    }

    #[test]
    fn normalizes_gain_matrix() {
        let mut c = sample();
        assert_eq!(c.gains, vec![vec![Gain::default(); 2]]);
        c.gains[0][1].db = f32::NAN;
        c.outputs[0].buffer_ms = 1;
        c.inputs.push(c.inputs[0].clone());
        let c = c.normalized();
        assert_eq!(c.gains.len(), 2);
        assert_eq!(c.gains[0][1].db, -120.0);
        assert_eq!(c.outputs[0].buffer_ms, 10);
    }

    #[test]
    fn round_trips_through_toml() {
        let dir = std::env::temp_dir().join(format!("faderust-test-{}", std::process::id()));
        let path = dir.join("config.toml");
        let mut c = sample();
        c.gains[0][0] = Gain {
            db: -6.5,
            muted: true,
        };
        c.save(&path).unwrap();
        assert_eq!(Config::load(&path).unwrap(), Some(c));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_file_is_none_and_garbage_is_error() {
        let dir = std::env::temp_dir().join(format!("faderust-test2-{}", std::process::id()));
        let path = dir.join("config.toml");
        assert_eq!(Config::load(&path).unwrap(), None);
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, "inputs = 5").unwrap();
        assert!(Config::load(&path).is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn fills_defaults() {
        let c: Config = toml::from_str("[[outputs]]\nlabel = \"A\"\ndevice = \"x\"\n").unwrap();
        assert_eq!(c.outputs[0].buffer_ms, 60);
        assert!(c.inputs.is_empty());
    }
}
