//! Per-device processing run inside the device callbacks.
//!
//! The topology changes while audio runs (devices come and go), so nodes get
//! their lanes through command queues and hand replaced ones back through a
//! garbage queue. That way nothing is allocated or freed on an audio thread.

use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

use rtrb::{Consumer, Producer, RingBuffer};

use crate::fx::{FxChain, InputFx};
use crate::lane::{Lane, LaneStats};
use crate::matrix::MixMatrix;
use crate::meter::PeakMeter;
use crate::BUS_CHANNELS;

/// Largest block an output node mixes at once; bigger callbacks are split.
const MAX_BLOCK_FRAMES: usize = 1024;
/// Capacity of the command and garbage queues of each node.
const QUEUE_LEN: usize = 64;

/// Input end of a lane.
pub type LaneInput = (Producer<f32>, Arc<LaneStats>);

/// Changes to an input node: set or clear the lane to output `usize`.
pub enum InputCommand {
    SetLane(usize, Option<LaneInput>),
}

/// Changes to an output node: set or clear the lane from input `usize`.
pub enum OutputCommand {
    SetLane(usize, Option<Box<Lane>>),
}

/// Values removed on an audio thread, to be dropped on the control thread.
pub enum Garbage {
    LaneInput(LaneInput),
    Lane(Box<Lane>),
}

/// Control-thread ends of a node's queues.
pub struct NodeHandle<C> {
    pub commands: Producer<C>,
    pub garbage: Consumer<Garbage>,
}

impl<C> NodeHandle<C> {
    /// Drops everything the node handed back.
    pub fn collect_garbage(&mut self) {
        while self.garbage.pop().is_ok() {}
    }

    /// Queues a command. Fails only if the node has not run for a while.
    pub fn send(&mut self, command: C) -> Result<(), C> {
        self.commands
            .push(command)
            .map_err(|rtrb::PushError::Full(c)| c)
    }
}

/// Queues shared by a node (audio side) and its handle (control side).
struct Queues<C> {
    commands: Consumer<C>,
    garbage: Producer<Garbage>,
}

fn queues<C>() -> (Queues<C>, NodeHandle<C>) {
    let (cmd_tx, cmd_rx) = RingBuffer::new(QUEUE_LEN);
    let (garbage_tx, garbage_rx) = RingBuffer::new(QUEUE_LEN);
    (
        Queues {
            commands: cmd_rx,
            garbage: garbage_tx,
        },
        NodeHandle {
            commands: cmd_tx,
            garbage: garbage_rx,
        },
    )
}

impl<C> Queues<C> {
    fn dispose(&mut self, item: Garbage) {
        // Never free on the audio thread. If the control thread stalls long
        // enough to fill the queue, leaking one lane is the lesser evil.
        if let Err(rtrb::PushError::Full(item)) = self.garbage.push(item) {
            std::mem::forget(item);
        }
    }
}

/// Frames an input node converts and processes at once.
const INPUT_BLOCK_FRAMES: usize = 64;

/// Input side: converts device frames to the stereo bus, runs the input
/// effects and fans the result out to one ring buffer per output.
pub struct InputNode {
    /// Indexed by output slot.
    lanes: Vec<Option<LaneInput>>,
    meter: Arc<PeakMeter>,
    fx: Arc<InputFx>,
    chain: FxChain,
    queues: Queues<InputCommand>,
}

impl InputNode {
    /// `lanes` must have one entry per output slot.
    pub fn new(
        lanes: Vec<Option<LaneInput>>,
        meter: Arc<PeakMeter>,
        fx: Arc<InputFx>,
        sample_rate: u32,
    ) -> (Self, NodeHandle<InputCommand>) {
        let (queues, handle) = queues();
        (
            Self {
                lanes,
                meter,
                fx,
                chain: FxChain::new(sample_rate),
                queues,
            },
            handle,
        )
    }

    fn apply_commands(&mut self) {
        while let Ok(InputCommand::SetLane(output, lane)) = self.queues.commands.pop() {
            let old = match self.lanes.get_mut(output) {
                Some(slot) => std::mem::replace(slot, lane),
                None => lane,
            };
            if let Some(old) = old {
                self.queues.dispose(Garbage::LaneInput(old));
            }
        }
    }

    /// Handles one device callback of interleaved samples with `channels`
    /// channels. Real-time safe.
    pub fn process(&mut self, data: &[f32], channels: usize) {
        self.apply_commands();
        if channels == 0 {
            return;
        }
        let mut block = [0.0f32; INPUT_BLOCK_FRAMES * BUS_CHANNELS];
        for chunk in data.chunks(INPUT_BLOCK_FRAMES * channels) {
            let frames = chunk.len() / channels;
            let block = &mut block[..frames * BUS_CHANNELS];
            for (frame, out) in chunk
                .chunks_exact(channels)
                .zip(block.chunks_exact_mut(BUS_CHANNELS))
            {
                let (l, r) = to_stereo(frame);
                out[0] = l;
                out[1] = r;
            }
            self.chain.process(&self.fx, block);
            // Metered after the effects, so the gate shows.
            self.meter.update(block);

            for (producer, stats) in self.lanes.iter_mut().flatten() {
                let writable = (producer.slots() / BUS_CHANNELS).min(frames);
                if writable < frames {
                    stats
                        .dropped_frames
                        .fetch_add((frames - writable) as u64, Relaxed);
                }
                let Ok(mut out) = producer.write_chunk(writable * BUS_CHANNELS) else {
                    continue;
                };
                let (a, b) = out.as_mut_slices();
                let (first, rest) = block[..writable * BUS_CHANNELS].split_at(a.len());
                a.copy_from_slice(first);
                b.copy_from_slice(rest);
                out.commit_all();
            }
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
    /// Indexed by input slot.
    lanes: Vec<Option<Box<Lane>>>,
    matrix: Arc<MixMatrix>,
    meter: Arc<PeakMeter>,
    mix: Vec<f32>,
    queues: Queues<OutputCommand>,
}

impl OutputNode {
    /// `index` is this output's slot; `lanes` has one entry per input slot.
    pub fn new(
        index: usize,
        lanes: Vec<Option<Box<Lane>>>,
        matrix: Arc<MixMatrix>,
        meter: Arc<PeakMeter>,
    ) -> (Self, NodeHandle<OutputCommand>) {
        let (queues, handle) = queues();
        (
            Self {
                index,
                lanes,
                matrix,
                meter,
                mix: vec![0.0; MAX_BLOCK_FRAMES * BUS_CHANNELS],
                queues,
            },
            handle,
        )
    }

    fn apply_commands(&mut self) {
        while let Ok(OutputCommand::SetLane(input, lane)) = self.queues.commands.pop() {
            let old = match self.lanes.get_mut(input) {
                Some(slot) => std::mem::replace(slot, lane),
                None => lane,
            };
            if let Some(old) = old {
                self.queues.dispose(Garbage::Lane(old));
            }
        }
    }

    /// Fills one device callback of interleaved samples with `channels`
    /// channels. Real-time safe.
    pub fn render(&mut self, data: &mut [f32], channels: usize) {
        self.apply_commands();
        if channels == 0 {
            return;
        }
        for out in data.chunks_mut(MAX_BLOCK_FRAMES * channels) {
            let frames = out.len() / channels;
            let mix = &mut self.mix[..frames * BUS_CHANNELS];
            mix.fill(0.0);
            for (input, lane) in self.lanes.iter_mut().enumerate() {
                if let Some(lane) = lane {
                    let gain = self.matrix.effective_gain(input, self.index);
                    lane.render_add(mix, gain);
                }
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
        let stats = Arc::new(LaneStats::default());
        let (producer, lane) = lane(
            LaneSpec {
                input_rate: in_rate,
                output_rate: out_rate,
                target_ms,
            },
            Arc::clone(&stats),
        );
        let matrix = Arc::new(MixMatrix::new(1, 1));
        let (input, _) = InputNode::new(
            vec![Some((producer, Arc::clone(&stats)))],
            Arc::new(PeakMeter::new()),
            Arc::new(InputFx::new()),
            in_rate,
        );
        let (output, _) = OutputNode::new(
            0,
            vec![Some(Box::new(lane))],
            Arc::clone(&matrix),
            Arc::new(PeakMeter::new()),
        );
        Sim {
            input,
            output,
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

    #[test]
    fn swaps_lanes_through_commands() {
        let matrix = Arc::new(MixMatrix::new(2, 1));
        let (mut node, mut handle) =
            OutputNode::new(0, vec![None, None], matrix, Arc::new(PeakMeter::new()));
        let spec = LaneSpec {
            input_rate: 48_000,
            output_rate: 48_000,
            target_ms: 10,
        };
        let stats = Arc::new(LaneStats::default());
        let (_p1, l1) = lane(spec, Arc::clone(&stats));
        let (_p2, l2) = lane(spec, Arc::clone(&stats));
        let mut buf = vec![0.0; 960];

        assert!(handle
            .send(OutputCommand::SetLane(1, Some(Box::new(l1))))
            .is_ok());
        node.render(&mut buf, 2);
        assert!(handle.garbage.is_empty());

        // Replacing hands the old lane back instead of freeing it.
        assert!(handle
            .send(OutputCommand::SetLane(1, Some(Box::new(l2))))
            .is_ok());
        node.render(&mut buf, 2);
        assert_eq!(handle.garbage.slots(), 1);
        assert!(handle.send(OutputCommand::SetLane(1, None)).is_ok());
        node.render(&mut buf, 2);
        assert_eq!(handle.garbage.slots(), 2);
        handle.collect_garbage();
        assert!(handle.garbage.is_empty());

        // Out-of-range slots are ignored, the lane still comes back.
        let (_p3, l3) = lane(spec, stats);
        assert!(handle
            .send(OutputCommand::SetLane(7, Some(Box::new(l3))))
            .is_ok());
        node.render(&mut buf, 2);
        assert_eq!(handle.garbage.slots(), 1);
    }
}
