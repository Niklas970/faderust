//! Input effects: noise suppression, low cut, three-band EQ and noise gate,
//! mainly for the microphone.
//!
//! Settings live in [`InputFx`] (atomics, shared with the control thread);
//! [`FxChain`] runs inside the input callback and picks up changes without
//! allocating or locking.

use std::f32::consts::PI;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use nnnoiseless::DenoiseState;

/// Effect settings of one input. The default does nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FxSettings {
    /// RNNoise noise suppression (keyboard, fans). Only at 48 kHz.
    pub denoise: bool,
    /// High-pass corner in Hz, 0 = off.
    pub low_cut_hz: f32,
    /// Shelf at 120 Hz, dB.
    pub low_db: f32,
    /// Bell at 2.5 kHz (speech presence), dB.
    pub mid_db: f32,
    /// Shelf at 8 kHz, dB.
    pub high_db: f32,
    pub gate: bool,
    /// Level below which the gate closes, dBFS.
    pub gate_db: f32,
}

impl Default for FxSettings {
    fn default() -> Self {
        Self {
            denoise: false,
            low_cut_hz: 0.0,
            low_db: 0.0,
            mid_db: 0.0,
            high_db: 0.0,
            gate: false,
            gate_db: -45.0,
        }
    }
}

impl FxSettings {
    /// Clamps to sane ranges; non-finite values fall back to the default.
    pub fn sanitized(self) -> Self {
        let d = Self::default();
        let clamp = |v: f32, def: f32, lo: f32, hi: f32| {
            if v.is_finite() {
                v.clamp(lo, hi)
            } else {
                def
            }
        };
        Self {
            denoise: self.denoise,
            low_cut_hz: clamp(self.low_cut_hz, d.low_cut_hz, 0.0, 400.0),
            low_db: clamp(self.low_db, 0.0, -12.0, 12.0),
            mid_db: clamp(self.mid_db, 0.0, -12.0, 12.0),
            high_db: clamp(self.high_db, 0.0, -12.0, 12.0),
            gate: self.gate,
            gate_db: clamp(self.gate_db, d.gate_db, -90.0, -10.0),
        }
    }
}

/// Settings shared between the control thread and an input callback.
#[derive(Debug)]
pub struct InputFx {
    denoise: AtomicBool,
    low_cut_hz: AtomicU32,
    low_db: AtomicU32,
    mid_db: AtomicU32,
    high_db: AtomicU32,
    gate: AtomicBool,
    gate_db: AtomicU32,
    /// Bumped after every change so the callback knows to reload.
    version: AtomicU32,
}

impl Default for InputFx {
    fn default() -> Self {
        let fx = Self {
            denoise: AtomicBool::new(false),
            low_cut_hz: AtomicU32::new(0),
            low_db: AtomicU32::new(0),
            mid_db: AtomicU32::new(0),
            high_db: AtomicU32::new(0),
            gate: AtomicBool::new(false),
            gate_db: AtomicU32::new(0),
            version: AtomicU32::new(0),
        };
        fx.set(FxSettings::default());
        fx
    }
}

impl InputFx {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&self, s: FxSettings) {
        let s = s.sanitized();
        let r = Ordering::Relaxed;
        self.denoise.store(s.denoise, r);
        self.low_cut_hz.store(s.low_cut_hz.to_bits(), r);
        self.low_db.store(s.low_db.to_bits(), r);
        self.mid_db.store(s.mid_db.to_bits(), r);
        self.high_db.store(s.high_db.to_bits(), r);
        self.gate.store(s.gate, r);
        self.gate_db.store(s.gate_db.to_bits(), r);
        self.version.fetch_add(1, Ordering::Release);
    }

    pub fn get(&self) -> FxSettings {
        let r = Ordering::Relaxed;
        let f = |a: &AtomicU32| f32::from_bits(a.load(r));
        FxSettings {
            denoise: self.denoise.load(r),
            low_cut_hz: f(&self.low_cut_hz),
            low_db: f(&self.low_db),
            mid_db: f(&self.mid_db),
            high_db: f(&self.high_db),
            gate: self.gate.load(r),
            gate_db: f(&self.gate_db),
        }
    }

    fn version(&self) -> u32 {
        self.version.load(Ordering::Acquire)
    }
}

/// Second-order IIR filter (RBJ cookbook), transposed direct form II.
#[derive(Debug, Clone, Copy)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    /// Filter state per channel.
    z: [[f32; 2]; 2],
}

impl Biquad {
    const IDENTITY: Self = Self {
        b0: 1.0,
        b1: 0.0,
        b2: 0.0,
        a1: 0.0,
        a2: 0.0,
        z: [[0.0; 2]; 2],
    };

    /// Builds from unnormalised coefficients, keeping the channel state so a
    /// settings change does not click.
    fn with(self, b: [f32; 3], a: [f32; 3]) -> Self {
        Self {
            b0: b[0] / a[0],
            b1: b[1] / a[0],
            b2: b[2] / a[0],
            a1: a[1] / a[0],
            a2: a[2] / a[0],
            z: self.z,
        }
    }

    fn highpass(self, rate: f32, hz: f32) -> Self {
        let (cos, alpha) = Self::omega(rate, hz, std::f32::consts::FRAC_1_SQRT_2);
        self.with(
            [(1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0],
            [1.0 + alpha, -2.0 * cos, 1.0 - alpha],
        )
    }

    fn peak(self, rate: f32, hz: f32, db: f32) -> Self {
        let a = 10f32.powf(db / 40.0);
        let (cos, alpha) = Self::omega(rate, hz, 1.0);
        self.with(
            [1.0 + alpha * a, -2.0 * cos, 1.0 - alpha * a],
            [1.0 + alpha / a, -2.0 * cos, 1.0 - alpha / a],
        )
    }

    fn shelf(self, rate: f32, hz: f32, db: f32, high: bool) -> Self {
        let a = 10f32.powf(db / 40.0);
        let (cos, alpha) = Self::omega(rate, hz, std::f32::consts::FRAC_1_SQRT_2);
        let s = 2.0 * a.sqrt() * alpha;
        let (ap, am) = (a + 1.0, a - 1.0);
        if high {
            self.with(
                [
                    a * (ap + am * cos + s),
                    -2.0 * a * (am + ap * cos),
                    a * (ap + am * cos - s),
                ],
                [ap - am * cos + s, 2.0 * (am - ap * cos), ap - am * cos - s],
            )
        } else {
            self.with(
                [
                    a * (ap - am * cos + s),
                    2.0 * a * (am - ap * cos),
                    a * (ap - am * cos - s),
                ],
                [ap + am * cos + s, -2.0 * (am + ap * cos), ap + am * cos - s],
            )
        }
    }

    /// cos(w0) and alpha for corner `hz` and quality `q`. Corners above
    /// Nyquist are pulled just below it.
    fn omega(rate: f32, hz: f32, q: f32) -> (f32, f32) {
        let w0 = 2.0 * PI * hz.min(rate * 0.45) / rate;
        (w0.cos(), w0.sin() / (2.0 * q))
    }

    #[inline]
    fn run(&mut self, ch: usize, x: f32) -> f32 {
        let z = &mut self.z[ch];
        let y = self.b0 * x + z[0];
        z[0] = self.b1 * x - self.a1 * y + z[1];
        z[1] = self.b2 * x - self.a2 * y;
        y
    }
}

/// Noise gate with hysteresis, hold time and smooth gain ramps.
#[derive(Debug, Clone, Copy)]
struct Gate {
    envelope: f32,
    gain: f32,
    open: bool,
    hold_left: u32,
    // Per-sample coefficients, set from the sample rate.
    env_release: f32,
    attack_step: f32,
    release_step: f32,
    hold: u32,
}

/// The gate closes this far below its threshold, so it does not chatter.
const GATE_HYSTERESIS_DB: f32 = 6.0;

impl Gate {
    fn new(rate: f32) -> Self {
        Self {
            envelope: 0.0,
            gain: 0.0,
            open: false,
            hold_left: 0,
            // Slow enough envelope, hold and fade that word endings and
            // soft consonants are not cut off.
            env_release: (-1.0 / (0.05 * rate)).exp(),
            attack_step: 1.0 / (0.002 * rate),
            release_step: 1.0 / (0.200 * rate),
            hold: (0.25 * rate) as u32,
        }
    }

    #[inline]
    fn gain(&mut self, peak: f32, open_at: f32, close_at: f32) -> f32 {
        // Instant attack, 50 ms release.
        self.envelope = peak.max(self.envelope * self.env_release);
        if self.envelope >= open_at {
            self.open = true;
            self.hold_left = self.hold;
        } else if self.envelope < close_at {
            if self.hold_left > 0 {
                self.hold_left -= 1;
            } else {
                self.open = false;
            }
        }
        self.gain = if self.open {
            (self.gain + self.attack_step).min(1.0)
        } else {
            (self.gain - self.release_step).max(0.0)
        };
        self.gain
    }
}

/// RNNoise works on 480 samples (10 ms) at 48 kHz.
const RNN_FRAME: usize = DenoiseState::FRAME_SIZE;
const RNN_RATE: u32 = 48_000;
/// RNNoise expects 16-bit sized values instead of -1…1.
const RNN_SCALE: f32 = 32768.0;

/// One channel of RNNoise. Collects a frame, then plays the frame it
/// processed before: 10 ms of delay, no allocation while running.
struct Denoiser {
    state: Box<DenoiseState<'static>>,
    input: [f32; RNN_FRAME],
    output: [f32; RNN_FRAME],
    pos: usize,
}

impl Denoiser {
    fn new() -> Self {
        Self {
            state: DenoiseState::new(),
            input: [0.0; RNN_FRAME],
            output: [0.0; RNN_FRAME],
            pos: 0,
        }
    }

    /// Starts over, so nothing stale plays when the effect is switched on.
    fn clear(&mut self) {
        self.input.fill(0.0);
        self.output.fill(0.0);
        self.pos = 0;
    }

    #[inline]
    fn run(&mut self, x: f32) -> f32 {
        let y = self.output[self.pos];
        self.input[self.pos] = x * RNN_SCALE;
        self.pos += 1;
        if self.pos == RNN_FRAME {
            self.pos = 0;
            self.state.process_frame(&mut self.output, &self.input);
            for v in &mut self.output {
                *v = (*v / RNN_SCALE).clamp(-1.0, 1.0);
            }
        }
        y
    }
}

/// The effects of one input, run in its callback. Real-time safe.
pub struct FxChain {
    /// Left and right; only at [`RNN_RATE`].
    denoise: Option<[Denoiser; 2]>,
    denoising: bool,
    rate: f32,
    seen: Option<u32>,
    settings: FxSettings,
    low_cut: Biquad,
    low: Biquad,
    mid: Biquad,
    high: Biquad,
    gate: Gate,
    open_at: f32,
    close_at: f32,
}

fn linear(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

impl FxChain {
    pub fn new(sample_rate: u32) -> Self {
        let rate = sample_rate.max(1) as f32;
        Self {
            denoise: (sample_rate == RNN_RATE).then(|| [Denoiser::new(), Denoiser::new()]),
            denoising: false,
            rate,
            seen: None,
            settings: FxSettings::default(),
            low_cut: Biquad::IDENTITY,
            low: Biquad::IDENTITY,
            mid: Biquad::IDENTITY,
            high: Biquad::IDENTITY,
            gate: Gate::new(rate),
            open_at: 0.0,
            close_at: 0.0,
        }
    }

    fn reload(&mut self, s: FxSettings) {
        let rate = self.rate;
        self.settings = s;
        if s.low_cut_hz > 0.0 {
            self.low_cut = self.low_cut.highpass(rate, s.low_cut_hz);
        }
        self.low = self.low.shelf(rate, 120.0, s.low_db, false);
        self.mid = self.mid.peak(rate, 2500.0, s.mid_db);
        self.high = self.high.shelf(rate, 8000.0, s.high_db, true);
        self.open_at = linear(s.gate_db);
        self.close_at = linear(s.gate_db - GATE_HYSTERESIS_DB);
    }

    /// Processes interleaved stereo frames in place.
    pub fn process(&mut self, fx: &InputFx, frames: &mut [f32]) {
        let version = fx.version();
        if self.seen != Some(version) {
            self.seen = Some(version);
            self.reload(fx.get());
        }
        let s = self.settings;
        let denoise = s.denoise.then_some(self.denoise.as_mut()).flatten();
        if let Some([left, right]) = denoise {
            if !self.denoising {
                left.clear();
                right.clear();
            }
            for frame in frames.chunks_exact_mut(2) {
                frame[0] = left.run(frame[0]);
                frame[1] = right.run(frame[1]);
            }
            self.denoising = true;
        } else {
            self.denoising = false;
        }
        let filters = [
            (s.low_cut_hz > 0.0, &mut self.low_cut),
            (s.low_db != 0.0, &mut self.low),
            (s.mid_db != 0.0, &mut self.mid),
            (s.high_db != 0.0, &mut self.high),
        ];
        for (on, filter) in filters {
            if on {
                for frame in frames.chunks_exact_mut(2) {
                    frame[0] = filter.run(0, frame[0]);
                    frame[1] = filter.run(1, frame[1]);
                }
            }
        }
        if s.gate {
            for frame in frames.chunks_exact_mut(2) {
                let peak = frame[0].abs().max(frame[1].abs());
                let g = self.gate.gain(peak, self.open_at, self.close_at);
                frame[0] *= g;
                frame[1] *= g;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;

    fn sine(hz: f32, amp: f32, frames: usize) -> Vec<f32> {
        (0..frames)
            .flat_map(|n| {
                let v = amp * (2.0 * PI * hz * n as f32 / RATE as f32).sin();
                [v, v]
            })
            .collect()
    }

    /// Peak of the second half, after the filters settled.
    fn peak(buf: &[f32]) -> f32 {
        buf[buf.len() / 2..].iter().fold(0.0, |m, v| m.max(v.abs()))
    }

    fn run(settings: FxSettings, mut buf: Vec<f32>) -> Vec<f32> {
        let fx = InputFx::new();
        fx.set(settings);
        let mut chain = FxChain::new(RATE);
        for block in buf.chunks_mut(128) {
            chain.process(&fx, block);
        }
        buf
    }

    #[test]
    fn default_is_transparent() {
        let input = sine(440.0, 0.5, 4800);
        assert_eq!(run(FxSettings::default(), input.clone()), input);
    }

    /// Steady noise from a fixed sequence, on both sides.
    fn noise(amp: f32, frames: usize) -> Vec<f32> {
        let mut x: u32 = 12345;
        (0..frames)
            .flat_map(|_| {
                x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                let v = amp * ((x >> 8) as f32 / (1u32 << 23) as f32 - 1.0);
                [v, v]
            })
            .collect()
    }

    fn rms(buf: &[f32]) -> f32 {
        (buf.iter().map(|v| v * v).sum::<f32>() / buf.len() as f32).sqrt()
    }

    /// What RNNoise does to real noise needs a listen (it is trained on
    /// speech and everyday noise, not synthetic signals); here it must not
    /// damage a clean signal or overshoot.
    #[test]
    fn denoise_passes_a_clean_tone_without_overshoot() {
        let input = sine(440.0, 0.5, 48_000);
        let s = FxSettings {
            denoise: true,
            ..Default::default()
        };
        let out = run(s, input.clone());
        let (before, after) = (rms(&input[48_000..]), rms(&out[48_000..]));
        assert!(
            (after / before - 1.0).abs() < 0.1,
            "tone {before} -> {after}"
        );
        assert!(out.iter().all(|v| v.abs() <= 1.0));
    }

    #[test]
    fn denoise_delays_by_one_frame_and_starts_silent() {
        let s = FxSettings {
            denoise: true,
            ..Default::default()
        };
        let out = run(s, sine(440.0, 0.5, 4800));
        assert!(out[..RNN_FRAME * 2].iter().all(|v| *v == 0.0));
    }

    #[test]
    fn denoise_is_skipped_off_48_khz() {
        let fx = InputFx::new();
        fx.set(FxSettings {
            denoise: true,
            ..Default::default()
        });
        let input = sine(440.0, 0.5, 4410);
        let mut out = input.clone();
        let mut chain = FxChain::new(44_100);
        for block in out.chunks_mut(128) {
            chain.process(&fx, block);
        }
        assert_eq!(out, input);
    }

    #[test]
    fn switching_denoise_off_restores_the_plain_signal() {
        let fx = InputFx::new();
        let mut chain = FxChain::new(RATE);
        fx.set(FxSettings {
            denoise: true,
            ..Default::default()
        });
        let mut buf = noise(0.1, 4800);
        chain.process(&fx, &mut buf);
        fx.set(FxSettings::default());
        let input = sine(440.0, 0.5, 1000);
        let mut out = input.clone();
        chain.process(&fx, &mut out);
        assert_eq!(out, input);
    }

    #[test]
    fn low_cut_removes_rumble_keeps_voice() {
        let s = FxSettings {
            low_cut_hz: 100.0,
            ..Default::default()
        };
        assert!(peak(&run(s, sine(30.0, 0.5, 9600))) < 0.06);
        assert!(peak(&run(s, sine(1000.0, 0.5, 9600))) > 0.49);
    }

    #[test]
    fn eq_bands_boost_their_range() {
        let boost = |s: FxSettings, hz| peak(&run(s, sine(hz, 0.1, 9600))) / 0.1;
        let low = FxSettings {
            low_db: 6.0,
            ..Default::default()
        };
        assert!((boost(low, 40.0) - 2.0).abs() < 0.1);
        assert!((boost(low, 5000.0) - 1.0).abs() < 0.05);
        let mid = FxSettings {
            mid_db: -6.0,
            ..Default::default()
        };
        assert!((boost(mid, 2500.0) - 0.5).abs() < 0.03);
        let high = FxSettings {
            high_db: 6.0,
            ..Default::default()
        };
        assert!((boost(high, 18_000.0) - 2.0).abs() < 0.15);
        assert!((boost(high, 200.0) - 1.0).abs() < 0.05);
    }

    #[test]
    fn gate_mutes_quiet_and_passes_loud() {
        let s = FxSettings {
            gate: true,
            gate_db: -40.0,
            ..Default::default()
        };
        // -60 dBFS noise floor: closed.
        assert!(peak(&run(s, sine(440.0, 0.001, 9600))) < 1e-6);
        // -12 dBFS speech: open and untouched.
        let loud = run(s, sine(440.0, 0.25, 9600));
        assert!((peak(&loud) - 0.25).abs() < 1e-3);
    }

    #[test]
    fn gate_holds_then_fades() {
        let s = FxSettings {
            gate: true,
            gate_db: -40.0,
            ..Default::default()
        };
        let mut buf = sine(440.0, 0.25, 4800);
        buf.extend(sine(440.0, 0.001, 60_000));
        let out = run(s, buf);
        let at = |ms: usize| {
            let i = (4800 + ms * 48) * 2;
            out[i..i + 960].iter().fold(0.0f32, |m, v| m.max(v.abs()))
        };
        // Still open while a word fades out, closed well after it.
        assert!(at(50) > 0.0009);
        assert!(at(350) > 0.0009);
        assert!(at(900) < 1e-6);
    }

    #[test]
    fn sanitizes_settings() {
        let s = FxSettings {
            low_cut_hz: f32::NAN,
            low_db: 40.0,
            gate_db: -500.0,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(s.low_cut_hz, 0.0);
        assert_eq!(s.low_db, 12.0);
        assert_eq!(s.gate_db, -90.0);
    }
}
