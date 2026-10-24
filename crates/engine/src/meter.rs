//! Lock-free peak meters.

use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

use crate::BUS_CHANNELS;

/// Stereo peak meter written by the audio thread and read by the UI.
///
/// Non-negative `f32` values keep their order when compared as raw bits, so
/// `fetch_max` on the bit pattern gives a lock-free running maximum.
#[derive(Debug, Default)]
pub struct PeakMeter {
    peak: [AtomicU32; BUS_CHANNELS],
}

impl PeakMeter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the peaks of an interleaved stereo block. Real-time safe.
    pub fn update(&self, interleaved: &[f32]) {
        let mut block = [0.0f32; BUS_CHANNELS];
        for frame in interleaved.chunks_exact(BUS_CHANNELS) {
            for (peak, s) in block.iter_mut().zip(frame) {
                *peak = peak.max(s.abs());
            }
        }
        for (slot, peak) in self.peak.iter().zip(block) {
            // NaN would compare above every finite value; ignore it.
            if peak > 0.0 && peak.is_finite() {
                slot.fetch_max(peak.to_bits(), Relaxed);
            }
        }
    }

    /// Returns the peaks since the last call and resets them.
    pub fn take(&self) -> [f32; BUS_CHANNELS] {
        let mut out = [0.0; BUS_CHANNELS];
        for (o, slot) in out.iter_mut().zip(&self.peak) {
            *o = f32::from_bits(slot.swap(0, Relaxed));
        }
        out
    }
}

/// Converts a linear amplitude to dBFS, clamped at -120 dB.
pub fn to_db(linear: f32) -> f32 {
    if linear <= 1e-6 {
        -120.0
    } else {
        20.0 * linear.log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_maximum_until_taken() {
        let m = PeakMeter::new();
        m.update(&[0.1, -0.5, 0.3, 0.2]);
        m.update(&[-0.2, 0.1]);
        assert_eq!(m.take(), [0.3, 0.5]);
        assert_eq!(m.take(), [0.0, 0.0]);
    }

    #[test]
    fn ignores_nan() {
        let m = PeakMeter::new();
        m.update(&[f32::NAN, 0.25]);
        assert_eq!(m.take(), [0.0, 0.25]);
    }

    #[test]
    fn db_conversion() {
        assert!((to_db(1.0)).abs() < 1e-6);
        assert!((to_db(0.5) + 6.0206).abs() < 1e-3);
        assert_eq!(to_db(0.0), -120.0);
    }
}
