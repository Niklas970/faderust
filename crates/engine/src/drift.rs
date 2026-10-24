//! Clock drift compensation from the ring buffer fill level.

/// PI controller that turns the fill level of an input's ring buffer into a
/// relative resampling ratio.
///
/// The fill level integrates the rate difference between the two device
/// clocks. Keeping it at the target therefore makes the resampler run at
/// exactly the ratio the clocks need, without measuring them directly.
#[derive(Debug, Clone)]
pub struct DriftController {
    target_frames: f64,
    input_rate: f64,
    /// Smoothed fill level in frames; device callbacks deliver in bursts.
    smoothed: Option<f64>,
    integral: f64,
}

/// Proportional gain: 10 ms fill error -> 500 ppm correction. Kept low because
/// the fill level also wobbles with the phase between the device callbacks.
const KP: f64 = 0.05;
/// Integral gain, critically damped with `KP` (`KI = KP^2 / 4`).
const KI: f64 = KP * KP / 4.0;
/// Time constant of the fill level smoothing, in seconds.
const SMOOTHING_S: f64 = 0.5;
/// Largest correction applied, relative. Real clocks differ by well under 0.1 %.
pub const MAX_CORRECTION: f64 = 0.005;

impl DriftController {
    pub fn new(target_frames: usize, input_rate: u32) -> Self {
        Self {
            target_frames: target_frames as f64,
            input_rate: input_rate as f64,
            smoothed: None,
            integral: 0.0,
        }
    }

    /// Forgets the history, e.g. after an underrun.
    pub fn reset(&mut self) {
        self.smoothed = None;
        self.integral = 0.0;
    }

    /// Feeds one fill level measurement taken `dt_s` seconds after the last
    /// one and returns the relative ratio (output/input) to apply.
    /// Values below 1 consume input faster.
    pub fn update(&mut self, fill_frames: usize, dt_s: f64) -> f64 {
        let fill = fill_frames as f64;
        let smoothed = match self.smoothed {
            None => fill,
            Some(prev) => prev + (fill - prev) * (dt_s / SMOOTHING_S).min(1.0),
        };
        self.smoothed = Some(smoothed);

        let error_s = (smoothed - self.target_frames) / self.input_rate;
        let limit = MAX_CORRECTION / KI;
        self.integral = (self.integral + error_s * dt_s).clamp(-limit, limit);
        let correction = (KP * error_s + KI * self.integral).clamp(-MAX_CORRECTION, MAX_CORRECTION);
        1.0 - correction
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simulates the fill level as a continuous integral of the rate error.
    fn simulate(drift: f64, seconds: f64) -> (f64, f64) {
        let rate = 48_000.0;
        let target = 2_400;
        let mut c = DriftController::new(target, rate as u32);
        let dt = 0.005;
        let mut fill = target as f64;
        let mut ratio = 1.0;
        for _ in 0..(seconds / dt) as usize {
            ratio = c.update(fill.max(0.0) as usize, dt);
            // Input arrives at rate*(1+drift); output consumes rate/ratio input frames per second.
            fill += (rate * (1.0 + drift) - rate / ratio) * dt;
        }
        (fill - target as f64, ratio)
    }

    #[test]
    fn locks_onto_drift() {
        for drift in [-300e-6, 120e-6, 1000e-6] {
            let (err_frames, ratio) = simulate(drift, 300.0);
            assert!(
                err_frames.abs() < 10.0,
                "drift {drift}: fill error {err_frames}"
            );
            // Steady state: rate/ratio == rate*(1+drift).
            let expected = 1.0 / (1.0 + drift);
            assert!(
                (ratio - expected).abs() < 5e-6,
                "drift {drift}: ratio {ratio}"
            );
        }
    }

    #[test]
    fn correction_is_bounded() {
        let mut c = DriftController::new(100, 48_000);
        let r = c.update(1_000_000, 1.0);
        assert!((r - (1.0 - MAX_CORRECTION)).abs() < 1e-12);
        c.reset();
        let r = c.update(0, 1.0);
        assert!(r <= 1.0 + MAX_CORRECTION);
    }
}
