//! The "wire" of the cable: whatever is played comes out of the recording
//! side. A short buffer absorbs the jitter between the two USB clocks.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::device::{FRAME_BYTES, RATE};

/// When more than this is queued the oldest audio is dropped, so the cable
/// does not add more and more delay if the two sides drift apart.
const MAX_QUEUED: usize = RATE as usize / 25 * FRAME_BYTES; // 40 ms
const KEEP_QUEUED: usize = RATE as usize / 100 * FRAME_BYTES; // 10 ms

#[derive(Default)]
pub struct Ring {
    bytes: Mutex<VecDeque<u8>>,
    pub written: AtomicU64,
    pub underruns: AtomicU64,
    pub dropped: AtomicU64,
}

impl Ring {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write(&self, data: &[u8]) {
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
        let mut q = self.bytes.lock().unwrap_or_else(|e| e.into_inner());
        let n = out.len().min(q.len());
        for (slot, byte) in out.iter_mut().zip(q.drain(..n)) {
            *slot = byte;
        }
        if n < out.len() {
            out[n..].fill(0);
            self.underruns.fetch_add(1, Ordering::Relaxed);
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
    fn missing_data_is_silence() {
        let r = Ring::new();
        r.write(&[9, 9]);
        let mut out = [7u8; 4];
        r.read(&mut out);
        assert_eq!(out, [9, 9, 0, 0]);
        assert_eq!(r.underruns.load(Ordering::Relaxed), 1);
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
