//! One input feeding one output: ring buffer, resampler and drift control.

use std::sync::atomic::{AtomicI32, AtomicU64, Ordering::Relaxed};
use std::sync::Arc;

use audioadapter_buffers::direct::InterleavedSlice;
use rtrb::{Consumer, Producer, RingBuffer};
use rubato::{
    Adjustable, Async, FixedAsync, PolynomialDegree, Resampler, SincInterpolationParameters,
    SincInterpolationType, WindowFunction,
};

use crate::drift::{DriftController, MAX_CORRECTION};
use crate::BUS_CHANNELS;

/// Output frames the resampler produces per step.
const CHUNK_FRAMES: usize = 128;
/// Time constant of the gain smoothing that avoids clicks on fader moves.
const GAIN_SMOOTHING_S: f32 = 0.01;
/// Fill beyond this many targets is a backlog (e.g. after the output stalled)
/// that drift control would take minutes to work off: it is cut instead.
const BACKLOG_TARGETS: usize = 3;
/// Below this applied gain a muted lane counts as silent.
const SILENT_GAIN: f32 = 1e-5;

/// Counters for one lane, readable from any thread.
#[derive(Debug, Default)]
pub struct LaneStats {
    /// Ring buffer fill in input frames, as seen by the output side.
    pub fill_frames: AtomicU64,
    /// Current drift correction in ppm (negative: consuming input faster).
    pub correction_ppm: AtomicI32,
    /// Input frames dropped because the ring buffer was full.
    pub dropped_frames: AtomicU64,
    /// Output chunks filled with silence because the buffer ran empty.
    pub underruns: AtomicU64,
}

/// Lane parameters fixed when the topology is built.
#[derive(Debug, Clone, Copy)]
pub struct LaneSpec {
    pub input_rate: u32,
    pub output_rate: u32,
    /// Ring buffer fill the drift control aims for, in milliseconds.
    pub target_ms: u32,
}

/// Creates both ends of a lane. The producer goes to the input callback,
/// the [`Lane`] to the output callback. `stats` outlives reconnections, so
/// the UI can keep one handle per (input, output) pair.
pub fn lane(spec: LaneSpec, stats: Arc<LaneStats>) -> (Producer<f32>, Lane) {
    let target_frames = (spec.input_rate as u64 * spec.target_ms as u64 / 1000).max(1) as usize;
    let capacity_frames = (target_frames * 4).max(spec.input_rate as usize / 2);
    let (producer, consumer) = RingBuffer::new(capacity_frames * BUS_CHANNELS);

    let ratio = spec.output_rate as f64 / spec.input_rate as f64;
    let max_rel = 1.0 + 2.0 * MAX_CORRECTION;
    let resampler = if spec.input_rate == spec.output_rate {
        // Only drift to correct: a polynomial is cheap and transparent here.
        Async::new_poly(
            ratio,
            max_rel,
            PolynomialDegree::Septic,
            CHUNK_FRAMES,
            BUS_CHANNELS,
            FixedAsync::Output,
        )
    } else {
        // Real rate conversion (e.g. 44.1 -> 48 kHz) needs anti-aliasing.
        let params = SincInterpolationParameters::new(64, WindowFunction::Blackman2)
            .oversampling_factor(128)
            .interpolation(SincInterpolationType::Quadratic);
        Async::new_sinc(
            ratio,
            max_rel,
            &params,
            CHUNK_FRAMES,
            BUS_CHANNELS,
            FixedAsync::Output,
        )
    }
    .expect("valid resampler parameters");

    let in_buf = vec![0.0; resampler.input_frames_max() * BUS_CHANNELS];
    let out_buf = vec![0.0; resampler.output_frames_max() * BUS_CHANNELS];
    let lane = Lane {
        consumer,
        resampler,
        controller: DriftController::new(target_frames, spec.input_rate),
        target_frames,
        chunk_dt_s: CHUNK_FRAMES as f64 / spec.output_rate as f64,
        gain_alpha: 1.0 - (-1.0 / (GAIN_SMOOTHING_S * spec.output_rate as f32)).exp(),
        in_buf,
        out_buf,
        out_pos: 0,
        out_len: 0,
        primed: false,
        gain: 0.0,
        stats,
    };
    (producer, lane)
}

/// Output side of a lane. Owned by the output callback; never allocates
/// after construction.
pub struct Lane {
    consumer: Consumer<f32>,
    resampler: Async<f32>,
    controller: DriftController,
    target_frames: usize,
    chunk_dt_s: f64,
    gain_alpha: f32,
    in_buf: Vec<f32>,
    out_buf: Vec<f32>,
    out_pos: usize,
    out_len: usize,
    primed: bool,
    gain: f32,
    stats: Arc<LaneStats>,
}

impl Lane {
    pub fn stats(&self) -> &Arc<LaneStats> {
        &self.stats
    }

    /// Mixes the next `out.len() / 2` frames into `out` (interleaved stereo),
    /// moving the applied gain smoothly towards `target_gain`.
    pub fn render_add(&mut self, out: &mut [f32], target_gain: f32) {
        if target_gain == 0.0 && self.gain < SILENT_GAIN {
            // Muted or off: skip the resampler, it is most of the cost.
            self.gain = 0.0;
            return self.idle();
        }
        let frames = out.len() / BUS_CHANNELS;
        let mut done = 0;
        while done < frames {
            if self.out_pos == self.out_len {
                self.next_chunk();
            }
            let n = (self.out_len - self.out_pos).min(frames - done);
            let src = &self.out_buf[self.out_pos * BUS_CHANNELS..(self.out_pos + n) * BUS_CHANNELS];
            let dst = &mut out[done * BUS_CHANNELS..(done + n) * BUS_CHANNELS];
            for (d, s) in dst
                .chunks_exact_mut(BUS_CHANNELS)
                .zip(src.chunks_exact(BUS_CHANNELS))
            {
                self.gain += (target_gain - self.gain) * self.gain_alpha;
                for (d, s) in d.iter_mut().zip(s) {
                    *d += s * self.gain;
                }
            }
            self.out_pos += n;
            done += n;
        }
    }

    /// Drops input beyond the target fill, so the lane stays in step and
    /// plays on without delay once it is unmuted.
    fn idle(&mut self) {
        let fill = self.consumer.slots() / BUS_CHANNELS;
        self.stats.fill_frames.store(fill as u64, Relaxed);
        if fill > self.target_frames {
            if let Ok(chunk) = self
                .consumer
                .read_chunk((fill - self.target_frames) * BUS_CHANNELS)
            {
                chunk.commit_all();
            }
        }
        // Whatever was resampled before is stale now.
        self.out_pos = self.out_len;
    }

    /// Drops the oldest input beyond the target fill.
    fn trim_to_target(&mut self, fill: usize) {
        let excess = fill - self.target_frames;
        if let Ok(chunk) = self.consumer.read_chunk(excess * BUS_CHANNELS) {
            chunk.commit_all();
            self.stats.dropped_frames.fetch_add(excess as u64, Relaxed);
        }
    }

    fn silence(&mut self) {
        self.out_buf[..CHUNK_FRAMES * BUS_CHANNELS].fill(0.0);
        self.out_pos = 0;
        self.out_len = CHUNK_FRAMES;
    }

    fn next_chunk(&mut self) {
        let mut fill = self.consumer.slots() / BUS_CHANNELS;
        if fill > self.target_frames * BACKLOG_TARGETS {
            self.trim_to_target(fill);
            fill = self.target_frames;
            self.controller.reset();
        }
        self.stats.fill_frames.store(fill as u64, Relaxed);

        if !self.primed {
            if fill < self.target_frames {
                return self.silence();
            }
            self.primed = true;
            self.controller.reset();
        }

        let rel = self.controller.update(fill, self.chunk_dt_s);
        self.stats
            .correction_ppm
            .store(((rel - 1.0) * 1e6).round() as i32, Relaxed);
        // Cannot fail: |rel - 1| <= MAX_CORRECTION is within the configured range.
        let _ = self.resampler.set_resample_ratio_relative(rel, true);

        let needed = self.resampler.input_frames_next();
        if fill < needed {
            // Ran dry: play silence until the target fill is reached again.
            self.primed = false;
            self.stats.underruns.fetch_add(1, Relaxed);
            return self.silence();
        }

        let samples = needed * BUS_CHANNELS;
        let chunk = self
            .consumer
            .read_chunk(samples)
            .expect("checked available slots");
        let (a, b) = chunk.as_slices();
        self.in_buf[..a.len()].copy_from_slice(a);
        self.in_buf[a.len()..samples].copy_from_slice(b);
        chunk.commit_all();

        let input = InterleavedSlice::new(&self.in_buf[..samples], BUS_CHANNELS, needed)
            .expect("buffer sized for frames");
        let out_frames = self.out_buf.len() / BUS_CHANNELS;
        let mut output = InterleavedSlice::new_mut(&mut self.out_buf, BUS_CHANNELS, out_frames)
            .expect("buffer sized for frames");
        match self
            .resampler
            .process_into_buffer(&input, &mut output, None)
        {
            Ok((_, produced)) => {
                self.out_pos = 0;
                self.out_len = produced;
            }
            Err(_) => self.silence(),
        }
    }
}
