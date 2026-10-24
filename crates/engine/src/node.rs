//! Per-device processing run inside the device callbacks.

use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

use rtrb::Producer;

use crate::lane::{Lane, LaneStats};
use crate::matrix::MixMatrix;
use crate::meter::PeakMeter;
use crate::BUS_CHANNELS;

/// Largest block an output node mixes at once; bigger callbacks are split.
const MAX_BLOCK_FRAMES: usize = 1024;

/// Input side: converts device frames to the stereo bus and fans them out to
/// one ring buffer per output.
pub struct InputNode {
    lanes: Vec<(Producer<f32>, Arc<LaneStats>)>,
    meter: Arc<PeakMeter>,
}

impl InputNode {
    pub fn new(lanes: Vec<(Producer<f32>, Arc<LaneStats>)>, meter: Arc<PeakMeter>) -> Self {
        Self { lanes, meter }
    }

    /// Handles one device callback of interleaved samples with `channels`
    /// channels. Real-time safe.
    pub fn process(&mut self, data: &[f32], channels: usize) {
        if channels == 0 {
            return;
        }
        let frames = data.len() / channels;
        // Meter on the bus signal, in small blocks to stay allocation-free.
        let mut block = [0.0f32; 64 * BUS_CHANNELS];
        for chunk in data.chunks(64 * channels) {
            let n = chunk.len() / channels;
            for (frame, out) in chunk
                .chunks_exact(channels)
                .zip(block.chunks_exact_mut(BUS_CHANNELS))
            {
                let (l, r) = to_stereo(frame);
                out[0] = l;
                out[1] = r;
            }
            self.meter.update(&block[..n * BUS_CHANNELS]);
        }

        for (producer, stats) in &mut self.lanes {
            let writable = (producer.slots() / BUS_CHANNELS).min(frames);
            if writable < frames {
                stats
                    .dropped_frames
                    .fetch_add((frames - writable) as u64, Relaxed);
            }
            let Ok(mut chunk) = producer.write_chunk(writable * BUS_CHANNELS) else {
                continue;
            };
            let (a, b) = chunk.as_mut_slices();
            let mut frames_in = data.chunks_exact(channels);
            for slot in a
                .chunks_exact_mut(BUS_CHANNELS)
                .chain(b.chunks_exact_mut(BUS_CHANNELS))
            {
                let (l, r) = to_stereo(frames_in.next().expect("writable <= frames"));
                slot[0] = l;
                slot[1] = r;
            }
            chunk.commit_all();
        }
    }
}

/// Mono is copied to both sides; extra channels beyond stereo are ignored.
fn to_stereo(frame: &[f32]) -> (f32, f32) {
    match frame {
        [m] => (*m, *m),
        [l, r, ..] => (*l, *r),
        [] => (0.0, 0.0),
    }
}

/// Output side: mixes every input's lane with the matrix gains.
pub struct OutputNode {
    index: usize,
    /// `(input index, lane)`.
    lanes: Vec<(usize, Lane)>,
    matrix: Arc<MixMatrix>,
    meter: Arc<PeakMeter>,
    mix: Vec<f32>,
}

impl OutputNode {
    pub fn new(
        index: usize,
        lanes: Vec<(usize, Lane)>,
        matrix: Arc<MixMatrix>,
        meter: Arc<PeakMeter>,
    ) -> Self {
        Self {
            index,
            lanes,
            matrix,
            meter,
            mix: vec![0.0; MAX_BLOCK_FRAMES * BUS_CHANNELS],
        }
    }

    /// Fills one device callback of interleaved samples with `channels`
    /// channels. Real-time safe.
    pub fn render(&mut self, data: &mut [f32], channels: usize) {
        if channels == 0 {
            return;
        }
        for out in data.chunks_mut(MAX_BLOCK_FRAMES * channels) {
            let frames = out.len() / channels;
            let mix = &mut self.mix[..frames * BUS_CHANNELS];
            mix.fill(0.0);
            for (input, lane) in &mut self.lanes {
                let gain = self.matrix.effective_gain(*input, self.index);
                lane.render_add(mix, gain);
            }
            self.meter.update(mix);
            for (dst, src) in out
                .chunks_exact_mut(channels)
                .zip(mix.chunks_exact(BUS_CHANNELS))
            {
                let (l, r) = (src[0].clamp(-1.0, 1.0), src[1].clamp(-1.0, 1.0));
                match dst {
                    [m] => *m = 0.5 * (l + r),
                    [dl, dr, rest @ ..] => {
                        *dl = l;
                        *dr = r;
                        rest.fill(0.0);
                    }
                    [] => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lane::{lane, LaneSpec};

    struct Sim {
        input: InputNode,
        output: OutputNode,
        stats: Arc<LaneStats>,
        matrix: Arc<MixMatrix>,
        /// Simulated device clocks in seconds.
        t_in: f64,
        t_out: f64,
    }

    fn sim(in_rate: u32, out_rate: u32, target_ms: u32) -> Sim {
        let (producer, lane) = lane(LaneSpec {
            input_rate: in_rate,
            output_rate: out_rate,
            target_ms,
        });
        let stats = Arc::clone(lane.stats());
        let matrix = Arc::new(MixMatrix::new(1, 1));
        Sim {
            input: InputNode::new(
                vec![(producer, Arc::clone(&stats))],
                Arc::new(PeakMeter::new()),
            ),
            output: OutputNode::new(
                0,
                vec![(0, lane)],
                Arc::clone(&matrix),
                Arc::new(PeakMeter::new()),
            ),
            stats,
            matrix,
            t_in: 0.0,
            t_out: 0.003,
        }
    }

    /// Runs both devices for `seconds` more with 10 ms periods on their own
    /// (drifting) clocks. Input is a constant 0.5 on a mono device, output is
    /// stereo. Returns the last output block.
    fn run(s: &mut Sim, in_rate: f64, out_rate: f64, seconds: f64) -> Vec<f32> {
        let in_period = (in_rate / 100.0).round() as usize;
        let out_period = (out_rate / 100.0).round() as usize;
        let in_block = vec![0.5f32; in_period];
        let mut out_block = vec![0.0f32; out_period * 2];
        let end = s.t_in.min(s.t_out) + seconds;
        while s.t_in.min(s.t_out) < end {
            if s.t_in <= s.t_out {
                s.input.process(&in_block, 1);
                s.t_in += in_period as f64 / in_rate;
            } else {
                s.output.render(&mut out_block, 2);
                s.t_out += out_period as f64 / out_rate;
            }
        }
        out_block
    }

    #[test]
    fn passes_signal_with_gain() {
        let mut s = sim(48_000, 48_000, 40);
        s.matrix.set_gain_db(0, 0, -6.0);
        let out = run(&mut s, 48_000.0, 48_000.0, 3.0);
        for v in &out {
            assert!((v - 0.5 * 0.501).abs() < 2e-3, "got {v}");
        }
        assert_eq!(s.stats.underruns.load(Relaxed), 0);
    }

    #[test]
    fn mute_silences_smoothly() {
        let mut s = sim(48_000, 48_000, 40);
        run(&mut s, 48_000.0, 48_000.0, 2.0);
        s.matrix.set_muted(0, 0, true);
        let out = run(&mut s, 48_000.0, 48_000.0, 0.5);
        assert!(out.iter().all(|v| v.abs() < 1e-4));
    }

    #[test]
    fn compensates_clock_drift() {
        // Input clock 400 ppm fast: without compensation the buffer would
        // grow by 19 frames per second until it overflows.
        let mut s = sim(48_000, 48_000, 40);
        let in_rate = 48_000.0 * 1.0004;
        run(&mut s, in_rate, 48_000.0, 120.0);
        // The fill level wobbles with the callback phase, so judge averages.
        let samples = 60;
        let (mut fill, mut ppm) = (0.0, 0.0);
        for _ in 0..samples {
            run(&mut s, in_rate, 48_000.0, 2.0);
            fill += s.stats.fill_frames.load(Relaxed) as f64 / samples as f64;
            ppm += s.stats.correction_ppm.load(Relaxed) as f64 / samples as f64;
        }
        let target = 48_000.0 * 0.040;
        assert!(
            (fill - target).abs() < 480.0,
            "fill {fill}, target {target}"
        );
        assert!((ppm + 400.0).abs() < 100.0, "correction {ppm} ppm");
        assert_eq!(s.stats.underruns.load(Relaxed), 0);
        assert_eq!(s.stats.dropped_frames.load(Relaxed), 0);
    }

    #[test]
    fn converts_sample_rate() {
        let mut s = sim(44_100, 48_000, 40);
        let out = run(&mut s, 44_100.0, 48_000.0, 5.0);
        for v in &out {
            assert!((v - 0.5).abs() < 1e-2, "got {v}");
        }
        assert_eq!(s.stats.underruns.load(Relaxed), 0);
    }
}
