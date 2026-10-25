//! User configuration, stored as TOML in the app's config directory.

use std::fs;
use std::io;
use std::path::Path;

use faderust_engine::FxSettings;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputEntry {
    pub label: String,
    /// Device id or part of its name.
    pub device: String,
    #[serde(default)]
    pub fx: Fx,
}

/// Input effects, see [`faderust_engine::FxSettings`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Fx {
    /// High-pass corner in Hz, 0 = off.
    pub low_cut_hz: f32,
    pub low_db: f32,
    pub mid_db: f32,
    pub high_db: f32,
    pub gate: bool,
    pub gate_db: f32,
}

impl Default for Fx {
    fn default() -> Self {
        FxSettings::default().into()
    }
}

impl From<FxSettings> for Fx {
    fn from(s: FxSettings) -> Self {
        Self {
            low_cut_hz: s.low_cut_hz,
            low_db: s.low_db,
            mid_db: s.mid_db,
            high_db: s.high_db,
            gate: s.gate,
            gate_db: s.gate_db,
        }
    }
}

impl From<Fx> for FxSettings {
    fn from(f: Fx) -> Self {
        Self {
            low_cut_hz: f.low_cut_hz,
            low_db: f.low_db,
            mid_db: f.mid_db,
            high_db: f.high_db,
            gate: f.gate,
            gate_db: f.gate_db,
        }
    }
}

/// A named set of fader positions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub name: String,
    /// `gains[input][output]`, like [`Config::gains`].
    pub gains: Vec<Vec<Gain>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputEntry {
    pub label: String,
    /// Device id or part of its name.
    pub device: String,
    #[serde(default = "default_buffer_ms")]
    pub buffer_ms: u32,
    /// Master fader of the whole output.
    #[serde(default)]
    pub master: Gain,
    /// Seconds of silence before a Bluetooth output is released for
    /// multipoint; 0 keeps it open.
    #[serde(default = "default_release_secs")]
    pub release_secs: u32,
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
    #[serde(default)]
    pub presets: Vec<Preset>,
}

/// Resizes `gains` to `inputs` × `outputs` (new pairs at 0 dB) and clamps.
fn fit_gains(gains: &mut Vec<Vec<Gain>>, inputs: usize, outputs: usize) {
    gains.resize_with(inputs, Vec::new);
    for row in gains {
        row.resize_with(outputs, Gain::default);
        for g in row {
            if !g.db.is_finite() {
                g.db = -120.0;
            }
            g.db = g.db.clamp(-120.0, 12.0);
        }
    }
}

fn default_buffer_ms() -> u32 {
    60
}

fn default_release_secs() -> u32 {
    30
}

/// Buffers below this underrun constantly; above it latency gets silly.
pub const BUFFER_RANGE: std::ops::RangeInclusive<u32> = 10..=500;

impl Config {
    /// Sizes the gain matrices to the device lists and clamps values.
    pub fn normalized(mut self) -> Self {
        let (inputs, outputs) = (self.inputs.len(), self.outputs.len());
        fit_gains(&mut self.gains, inputs, outputs);
        for p in &mut self.presets {
            fit_gains(&mut p.gains, inputs, outputs);
        }
        for i in &mut self.inputs {
            i.fx = FxSettings::from(i.fx).sanitized().into();
        }
        for o in &mut self.outputs {
            o.buffer_ms = o
                .buffer_ms
                .clamp(*BUFFER_RANGE.start(), *BUFFER_RANGE.end());
            o.release_secs = o.release_secs.min(3600);
        }
        self
    }

    /// Stores the current faders as preset `name`, replacing one of that name.
    pub fn save_preset(&mut self, name: &str) {
        let preset = Preset {
            name: name.to_string(),
            gains: self.gains.clone(),
        };
        match self.presets.iter_mut().find(|p| p.name == name) {
            Some(p) => *p = preset,
            None => self.presets.push(preset),
        }
    }

    /// Sets the faders from preset `name`. False if there is none.
    pub fn load_preset(&mut self, name: &str) -> bool {
        let Some(p) = self.presets.iter().find(|p| p.name == name) else {
            return false;
        };
        self.gains = p.gains.clone();
        true
    }

    pub fn delete_preset(&mut self, name: &str) {
        self.presets.retain(|p| p.name != name);
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
                fx: Fx::default(),
            }],
            outputs: vec![
                OutputEntry {
                    label: "PS5".into(),
                    device: "USB Sound".into(),
                    buffer_ms: 30,
                    master: Default::default(),
                    release_secs: 30,
                },
                OutputEntry {
                    label: "Kopfhörer".into(),
                    device: "Buds".into(),
                    buffer_ms: 80,
                    master: Default::default(),
                    release_secs: 30,
                },
            ],
            gains: vec![],
            presets: vec![],
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
        c.inputs[0].fx.gate = true;
        c.inputs[0].fx.low_cut_hz = 80.0;
        c.save_preset("Abend");
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
    fn presets_save_load_and_follow_device_lists() {
        let mut c = sample();
        c.gains[0][1].db = -10.0;
        c.save_preset("Zocken");
        c.gains[0][1].db = 0.0;
        c.save_preset("Zocken");
        assert_eq!(c.presets.len(), 1);
        c.gains[0][1].db = 3.0;
        assert!(c.load_preset("Zocken"));
        assert_eq!(c.gains[0][1].db, 0.0);
        assert!(!c.load_preset("Nix"));

        // A new output gets 0 dB in the preset too.
        c.outputs.push(c.outputs[0].clone());
        let mut c = c.normalized();
        assert_eq!(c.presets[0].gains[0].len(), 3);
        c.delete_preset("Zocken");
        assert!(c.presets.is_empty());
    }

    #[test]
    fn fills_defaults() {
        let c: Config = toml::from_str("[[outputs]]\nlabel = \"A\"\ndevice = \"x\"\n").unwrap();
        assert_eq!(c.outputs[0].buffer_ms, 60);
        assert_eq!(c.outputs[0].release_secs, 30);
        assert_eq!(c.outputs[0].master, Gain::default());
        assert!(c.inputs.is_empty());
        let c: Config =
            toml::from_str("[[inputs]]\nlabel = \"A\"\ndevice = \"x\"\n[inputs.fx]\ngate = true\n")
                .unwrap();
        assert!(c.inputs[0].fx.gate);
        assert_eq!(c.inputs[0].fx.gate_db, -45.0);
        // 0.3.8 had a per-output setting; old files still load.
        let c: Config =
            toml::from_str("[[outputs]]\nlabel = \"A\"\ndevice = \"x\"\nreleaseAfterS = 30\n")
                .unwrap();
        assert_eq!(c.outputs[0].device, "x");
    }
}
