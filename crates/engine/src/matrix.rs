//! Mix matrix: one gain and mute switch per (input, output) pair.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};

/// Gains shared between the UI and the audio threads. All access is atomic,
/// so the audio thread never waits on the UI.
#[derive(Debug)]
pub struct MixMatrix {
    inputs: usize,
    outputs: usize,
    /// Linear gain as `f32` bits, row-major by input.
    gains: Box<[AtomicU32]>,
    muted: Box<[AtomicBool]>,
}

impl MixMatrix {
    /// Creates a matrix with every pair at unity gain and unmuted.
    pub fn new(inputs: usize, outputs: usize) -> Self {
        let n = inputs * outputs;
        Self {
            inputs,
            outputs,
            gains: (0..n).map(|_| AtomicU32::new(1.0f32.to_bits())).collect(),
            muted: (0..n).map(|_| AtomicBool::new(false)).collect(),
        }
    }

    pub fn inputs(&self) -> usize {
        self.inputs
    }

    pub fn outputs(&self) -> usize {
        self.outputs
    }

    fn index(&self, input: usize, output: usize) -> usize {
        assert!(
            input < self.inputs && output < self.outputs,
            "matrix index ({input}, {output}) out of range"
        );
        input * self.outputs + output
    }

    /// Sets a linear gain. Negative and non-finite values are treated as 0.
    pub fn set_gain(&self, input: usize, output: usize, linear: f32) {
        let g = if linear.is_finite() {
            linear.max(0.0)
        } else {
            0.0
        };
        self.gains[self.index(input, output)].store(g.to_bits(), Relaxed);
    }

    /// Sets the gain in dB; -inf or anything below -120 dB means silence.
    pub fn set_gain_db(&self, input: usize, output: usize, db: f32) {
        let linear = if db <= -120.0 {
            0.0
        } else {
            10f32.powf(db / 20.0)
        };
        self.set_gain(input, output, linear);
    }

    pub fn gain(&self, input: usize, output: usize) -> f32 {
        f32::from_bits(self.gains[self.index(input, output)].load(Relaxed))
    }

    pub fn set_muted(&self, input: usize, output: usize, muted: bool) {
        self.muted[self.index(input, output)].store(muted, Relaxed);
    }

    pub fn is_muted(&self, input: usize, output: usize) -> bool {
        self.muted[self.index(input, output)].load(Relaxed)
    }

    /// Gain the audio thread applies: 0 when muted. Real-time safe.
    pub fn effective_gain(&self, input: usize, output: usize) -> f32 {
        if self.is_muted(input, output) {
            0.0
        } else {
            self.gain(input, output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_unity() {
        let m = MixMatrix::new(3, 2);
        for i in 0..3 {
            for o in 0..2 {
                assert_eq!(m.effective_gain(i, o), 1.0);
            }
        }
    }

    #[test]
    fn gain_and_mute_are_per_pair() {
        let m = MixMatrix::new(2, 2);
        m.set_gain_db(0, 1, -6.0);
        m.set_muted(1, 0, true);
        assert!((m.gain(0, 1) - 0.501).abs() < 1e-3);
        assert_eq!(m.effective_gain(0, 0), 1.0);
        assert_eq!(m.effective_gain(1, 0), 0.0);
        assert_eq!(m.gain(1, 0), 1.0, "mute keeps the fader position");
        m.set_muted(1, 0, false);
        assert_eq!(m.effective_gain(1, 0), 1.0);
    }

    #[test]
    fn rejects_bad_gains() {
        let m = MixMatrix::new(1, 1);
        m.set_gain(0, 0, f32::NAN);
        assert_eq!(m.gain(0, 0), 0.0);
        m.set_gain(0, 0, -2.0);
        assert_eq!(m.gain(0, 0), 0.0);
        m.set_gain_db(0, 0, f32::NEG_INFINITY);
        assert_eq!(m.gain(0, 0), 0.0);
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn out_of_range_panics() {
        MixMatrix::new(1, 1).gain(1, 0);
    }
}
