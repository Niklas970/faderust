//! The "wire" of the cable: whatever is played comes out of the recording
//! side. A short buffer absorbs the jitter between the two USB clocks.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;

use crate::device::{FRAME_BYTES, RATE};

/// When more than this is queued the oldest audio is dropped, so the cable
/// does not add more and more delay if the two sides drift apart.
const MAX_QUEUED: usize = RATE as usize / 25 * FRAME_BYTES; // 40 ms
const KEEP_QUEUED: usize = RATE as usize / 100 * FRAME_BYTES; // 10 ms

/// Reads (1 ms each) over which a gap holds the last frame.
const HOLD_READS: u32 = 5;

#[derive(Default)]
pub struct Ring {
    /// Consecutive reads that found too little data.
    gap: AtomicU32,
    bytes: Mutex<VecDeque<u8>>,
    pub written: AtomicU64,
    pub underruns: AtomicU64,
    pub dropped: AtomicU64,
    /// Loudest sample written and read since [`Ring::take_peaks`], 24 bit
    /// magnitude: what goes into the cable and what the host receives.
    peak_in: AtomicU32,
    peak_out: AtomicU32,
}

/// The largest magnitude among the 24-bit little-endian samples of `data`.
fn peak_of(data: &[u8]) -> u32 {
    data.chunks_exact(3)
        .map(|s| (i32::from_le_bytes([0, s[0], s[1], s[2]]) >> 8).unsigned_abs())
        .max()
        .unwrap_or(0)
}

impl Ring {
    pub fn new() -> Self {
        Self::default()
    }

    /// The loudest samples (in, out) since the last call, 0…2^23.
    pub fn take_peaks(&self) -> (u32, u32) {
        (
            self.peak_in.swap(0, Ordering::Relaxed),
            self.peak_out.swap(0, Ordering::Relaxed),
        )
    }

    pub fn write(&self, data: &[u8]) {
        self.peak_in.fetch_max(peak_of(data), Ordering::Relaxed);
        let mut q = self.bytes.lock().unwrap_or_else(|e| e.into_inner());
        q.extend(data);
        self.written.fetch_add(data.len() as u64, Ordering::Relaxed);
        if q.len() > MAX_QUEUED {
            let excess = (q.len() - KEEP_QUEUED) / FRAME_BYTES * FRAME_BYTES;
            q.drain(..excess);
            self.dropped.fetch_add(excess as u64, Ordering::Relaxed);
        }
    }

    /// Fills `out`; whatever is missing becomes silence.
    pub fn read(&self, out: &mut [u8]) {
        self.read_inner(out);
        self.peak_out.fetch_max(peak_of(out), Ordering::Relaxed);
    }

    fn read_inner(&self, out: &mut [u8]) {
        let mut q = self.bytes.lock().unwrap_or_else(|e| e.into_inner());
        let n = out.len().min(q.len());
        for (slot, byte) in out.iter_mut().zip(q.drain(..n)) {
            *slot = byte;
        }
        if n < out.len() {
            // A short gap holds the last frame instead of dropping to zero: a
            // step to silence is heard as a click. A long one is silence.
            let gap = self.gap.fetch_add(1, Ordering::Relaxed);
            let last = if gap < HOLD_READS && n >= FRAME_BYTES {
                let mut f = [0u8; FRAME_BYTES];
                f.copy_from_slice(&out[n - FRAME_BYTES..n]);
                f
            } else {
                [0u8; FRAME_BYTES]
            };
            for (i, slot) in out[n..].iter_mut().enumerate() {
                *slot = last[i % FRAME_BYTES];
            }
            self.underruns.fetch_add(1, Ordering::Relaxed);
        } else {
            self.gap.store(0, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_written_is_read_back() {
        let r = Ring::new();
        r.write(&[1, 2, 3, 4]);
        let mut out = [0u8; 4];
        r.read(&mut out);
        assert_eq!(out, [1, 2, 3, 4]);
    }

    #[test]
    fn peaks_are_the_loudest_24_bit_samples_in_and_out() {
        let r = Ring::new();
        // +1000 and -3000 (two's complement, little endian), then -1.
        r.write(&[
            0xE8, 0x03, 0x00, 0x48, 0xF4, 0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0,
        ]);
        let mut out = [0u8; 6];
        r.read(&mut out);
        assert_eq!(r.take_peaks(), (3000, 3000));
        assert_eq!(r.take_peaks(), (0, 0));
    }

    #[test]
    fn missing_data_is_silence() {
        let r = Ring::new();
        r.write(&[9, 9]);
        let mut out = [7u8; 4];
        r.read(&mut out);
        // Less than one frame was available: silence, not garbage.
        assert_eq!(out, [9, 9, 0, 0]);
        assert_eq!(r.underruns.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn a_short_gap_holds_the_last_frame_and_a_long_one_is_silent() {
        let r = Ring::new();
        r.write(&[1, 2, 3, 4, 5, 6]);
        let mut out = [0u8; 12];
        r.read(&mut out);
        assert_eq!(out, [1, 2, 3, 4, 5, 6, 1, 2, 3, 4, 5, 6]);
        // Hold lasts a few reads only; with nothing written the last frame is
        // gone, so the following reads start from silence.
        for _ in 0..HOLD_READS {
            r.read(&mut out);
        }
        r.read(&mut out);
        assert_eq!(out, [0; 12]);
    }

    #[test]
    fn backlog_is_trimmed_in_whole_frames() {
        let r = Ring::new();
        r.write(&vec![1u8; MAX_QUEUED + FRAME_BYTES]);
        assert!(r.dropped.load(Ordering::Relaxed) > 0);
        assert_eq!(r.dropped.load(Ordering::Relaxed) as usize % FRAME_BYTES, 0);
        let q = r.bytes.lock().unwrap();
        assert!(q.len() <= MAX_QUEUED);
    }
}
