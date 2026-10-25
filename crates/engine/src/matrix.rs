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
    /// Pairs the engine keeps silent whatever the fader says (feedback loops).
    blocked: Box<[AtomicBool]>,
    /// Master gain per output (linear `f32` bits), applied on top of the pairs.
    master: Box<[AtomicU32]>,
    master_muted: Box<[AtomicBool]>,
    /// Mute per input, on top of its routing.
    input_muted: Box<[AtomicBool]>,
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
            blocked: (0..n).map(|_| AtomicBool::new(false)).collect(),
            master: (0..outputs)
                .map(|_| AtomicU32::new(1.0f32.to_bits()))
                .collect(),
            master_muted: (0..outputs).map(|_| AtomicBool::new(false)).collect(),
            input_muted: (0..inputs).map(|_| AtomicBool::new(false)).collect(),
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
        self.gains[self.index(input, output)].store(sane(linear).to_bits(), Relaxed);
    }

    /// Sets the gain in dB; -inf or anything below -120 dB means silence.
    pub fn set_gain_db(&self, input: usize, output: usize, db: f32) {
        self.set_gain(input, output, from_db(db));
    }

    /// Sets an output's master gain in dB (same rules as [`Self::set_gain_db`]).
    pub fn set_master_db(&self, output: usize, db: f32) {
        self.master[self.output(output)].store(sane(from_db(db)).to_bits(), Relaxed);
    }

    pub fn master(&self, output: usize) -> f32 {
        f32::from_bits(self.master[self.output(output)].load(Relaxed))
    }

    pub fn set_master_muted(&self, output: usize, muted: bool) {
        self.master_muted[self.output(output)].store(muted, Relaxed);
    }

    pub fn is_master_muted(&self, output: usize) -> bool {
        self.master_muted[self.output(output)].load(Relaxed)
    }

    pub fn set_input_muted(&self, input: usize, muted: bool) {
        assert!(input < self.inputs, "input {input} out of range");
        self.input_muted[input].store(muted, Relaxed);
    }

    pub fn is_input_muted(&self, input: usize) -> bool {
        assert!(input < self.inputs, "input {input} out of range");
        self.input_muted[input].load(Relaxed)
    }

    fn output(&self, output: usize) -> usize {
        assert!(output < self.outputs, "output {output} out of range");
        output
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

    pub fn set_blocked(&self, input: usize, output: usize, blocked: bool) {
        self.blocked[self.index(input, output)].store(blocked, Relaxed);
    }

    pub fn is_blocked(&self, input: usize, output: usize) -> bool {
        self.blocked[self.index(input, output)].load(Relaxed)
    }

    /// Gain the audio thread applies: the pair's gain times the output's
    /// master, 0 when the pair, the input or the output is muted or the pair
    /// is blocked. Real-time safe.
    pub fn effective_gain(&self, input: usize, output: usize) -> f32 {
        if self.is_muted(input, output)
            || self.is_blocked(input, output)
            || self.is_master_muted(output)
            || self.is_input_muted(input)
        {
            0.0
        } else {
            self.gain(input, output) * self.master(output)
        }
    }
}

fn from_db(db: f32) -> f32 {
    if db <= -120.0 {
        0.0
    } else {
        10f32.powf(db / 20.0)
    }
}

fn sane(linear: f32) -> f32 {
    if linear.is_finite() {
        linear.max(0.0)
    } else {
        0.0
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
    fn blocked_pair_is_silent_but_keeps_its_fader() {
        let m = MixMatrix::new(1, 1);
        m.set_blocked(0, 0, true);
        assert_eq!(m.effective_gain(0, 0), 0.0);
        assert_eq!(m.gain(0, 0), 1.0);
        m.set_blocked(0, 0, false);
        assert_eq!(m.effective_gain(0, 0), 1.0);
    }

    #[test]
    fn master_scales_and_mutes_a_whole_output() {
        let m = MixMatrix::new(2, 2);
        m.set_master_db(1, -6.0);
        assert_eq!(m.effective_gain(0, 0), 1.0);
        assert!((m.effective_gain(0, 1) - 0.501).abs() < 1e-3);
        assert!((m.effective_gain(1, 1) - 0.501).abs() < 1e-3);
        m.set_master_muted(0, true);
        assert_eq!(m.effective_gain(0, 0), 0.0);
        assert_eq!(m.effective_gain(1, 0), 0.0);
        assert_eq!(m.gain(0, 0), 1.0, "the pairs keep their faders");
        m.set_master_db(1, f32::NAN);
        assert_eq!(m.master(1), 0.0);
    }

    #[test]
    fn input_mute_silences_every_output_and_keeps_routing() {
        let m = MixMatrix::new(2, 2);
        m.set_input_muted(0, true);
        assert_eq!(m.effective_gain(0, 0), 0.0);
        assert_eq!(m.effective_gain(0, 1), 0.0);
        assert_eq!(m.effective_gain(1, 0), 1.0);
        assert!(!m.is_muted(0, 0), "routing stays as it was");
        m.set_input_muted(0, false);
        assert_eq!(m.effective_gain(0, 1), 1.0);
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
