//! Soundboard: short sounds played into a virtual input.
//!
//! The input `sound:board` is a stream without a device: a thread of its own
//! hands the engine 10 ms of audio at a time, silence while nothing plays.
//! Sounds are decoded in advance ([`load`]) and started with [`play`], from
//! any thread. The engine mixes the input like every other one.

use std::io::{Read, Seek};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::backend::InputCallback;

/// Id of the soundboard input.
pub const ID: &str = "sound:board";
pub const RATE: u32 = 48_000;
pub const CHANNELS: u16 = 2;
/// Audio handed to the engine per step.
const STEP: Duration = Duration::from_millis(10);
const STEP_FRAMES: usize = (RATE / 100) as usize;
/// After a stall (sleeping thread) at most this many steps are caught up.
const MAX_CATCH_UP: u32 = 10;
/// Longest sound accepted, in seconds (memory: 48 kHz stereo f32 = 384 kB/s).
const MAX_SECONDS: usize = 300;

/// A decoded sound: interleaved stereo `f32` at [`RATE`].
#[derive(Debug)]
pub struct Sample {
    data: Vec<f32>,
}

impl Sample {
    pub fn seconds(&self) -> f32 {
        self.data.len() as f32 / (2.0 * RATE as f32)
    }
}

struct Voice {
    sample: Arc<Sample>,
    /// Next sample (not frame) to play.
    pos: usize,
    gain: f32,
}

static QUEUE: Mutex<Vec<Voice>> = Mutex::new(Vec::new());
/// Counts [`stop_all`] calls; the stream drops its voices when it moves.
static STOPS: AtomicU64 = AtomicU64::new(0);
/// Number of sounds playing right now.
static PLAYING: AtomicU64 = AtomicU64::new(0);

/// Starts a sound. It is heard once the soundboard input is connected.
pub fn play(sample: Arc<Sample>, gain: f32) {
    if let Ok(mut q) = QUEUE.lock() {
        // A stuck queue (no stream running) must not grow forever.
        if q.len() >= 64 {
            q.remove(0);
        }
        q.push(Voice {
            sample,
            pos: 0,
            gain,
        });
    }
}

/// Stops every sound that plays or waits.
pub fn stop_all() {
    STOPS.fetch_add(1, Relaxed);
    if let Ok(mut q) = QUEUE.lock() {
        q.clear();
    }
}

/// How many sounds play at the moment.
pub fn playing() -> u64 {
    PLAYING.load(Relaxed)
}

/// Adds `voices` into `out` (interleaved stereo) and drops the finished ones.
fn mix(voices: &mut Vec<Voice>, out: &mut [f32]) {
    for v in voices.iter_mut() {
        let rest = &v.sample.data[v.pos..];
        let n = rest.len().min(out.len());
        for (o, s) in out.iter_mut().zip(&rest[..n]) {
            *o += s * v.gain;
        }
        v.pos += n;
    }
    voices.retain(|v| v.pos < v.sample.data.len());
}

/// The running soundboard input. Dropping it stops the thread.
pub struct SoundStream {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for SoundStream {
    fn drop(&mut self) {
        self.stop.store(true, Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Starts the input; `data` gets stereo audio at [`RATE`].
pub fn open(mut data: InputCallback) -> SoundStream {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stop);
    let thread = thread::Builder::new()
        .name("faderust-soundboard".into())
        .spawn(move || {
            let mut voices: Vec<Voice> = Vec::new();
            let mut buf = vec![0.0f32; STEP_FRAMES * 2];
            let mut seen = STOPS.load(Relaxed);
            let mut due = Instant::now();
            while !flag.load(Relaxed) {
                let now = Instant::now();
                if due > now {
                    thread::sleep((due - now).min(STEP));
                    continue;
                }
                // Behind schedule: catch up, but never in an endless burst.
                let late = now.duration_since(due).as_millis() / STEP.as_millis();
                if late as u32 > MAX_CATCH_UP {
                    due = now;
                }
                due += STEP;

                let stops = STOPS.load(Relaxed);
                if stops != seen {
                    seen = stops;
                    voices.clear();
                }
                if let Ok(mut q) = QUEUE.try_lock() {
                    voices.append(&mut q);
                }
                buf.fill(0.0);
                mix(&mut voices, &mut buf);
                PLAYING.store(voices.len() as u64, Relaxed);
                data(&buf);
            }
            PLAYING.store(0, Relaxed);
        })
        .expect("soundboard thread");
    SoundStream {
        stop,
        thread: Some(thread),
    }
}

/// Reads a WAV or MP3 file (by its extension) and converts it to stereo at
/// [`RATE`]. WAV: 8 to 32 bit integer or float, any rate, mono or more
/// channels. MP3: MPEG Layer III.
pub fn load(path: &Path) -> Result<Sample, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let reader = std::io::BufReader::new(file);
    let mp3 = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("mp3"));
    if mp3 {
        decode_mp3(reader)
    } else {
        decode(reader)
    }
}

fn decode<R: Read + Seek>(reader: R) -> Result<Sample, String> {
    let mut wav = hound::WavReader::new(reader).map_err(|e| e.to_string())?;
    let spec = wav.spec();
    let channels = spec.channels.max(1) as usize;
    if spec.sample_rate == 0 {
        return Err("invalid sample rate".into());
    }
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => wav
            .samples::<f32>()
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?,
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1u64 << (spec.bits_per_sample - 1)) as f32;
            wav.samples::<i32>()
                .map(|s| s.map(|v| v as f32 * scale))
                .collect::<Result<_, _>>()
                .map_err(|e| e.to_string())?
        }
    };
    // To stereo frames.
    let stereo: Vec<[f32; 2]> = samples
        .chunks_exact(channels)
        .map(|f| match channels {
            1 => [f[0], f[0]],
            _ => [f[0], f[1]],
        })
        .collect();
    finish(stereo, spec.sample_rate)
}

/// Decodes an MP3 (MPEG Layer III) with a pure Rust decoder.
fn decode_mp3<R: Read>(reader: R) -> Result<Sample, String> {
    let (header, samples) = puremp3::read_mp3(reader).map_err(|e| e.to_string())?;
    let rate = header.sample_rate.hz();
    // Stop early on a file far beyond the limit instead of decoding it all.
    let cap = MAX_SECONDS * rate as usize + 1;
    let stereo: Vec<[f32; 2]> = samples.take(cap).map(|(l, r)| [l, r]).collect();
    finish(stereo, rate)
}

/// Resamples to [`RATE`] and checks the length.
fn finish(mut stereo: Vec<[f32; 2]>, rate: u32) -> Result<Sample, String> {
    if stereo.is_empty() {
        return Err("empty file".into());
    }
    if rate != RATE {
        stereo = resample(&stereo, rate);
    }
    if stereo.len() > MAX_SECONDS * RATE as usize {
        return Err(format!("longer than {MAX_SECONDS} s"));
    }
    Ok(Sample {
        data: stereo.into_iter().flatten().collect(),
    })
}

/// Linear interpolation to [`RATE`]: plenty for short effects.
fn resample(frames: &[[f32; 2]], from: u32) -> Vec<[f32; 2]> {
    let n = (frames.len() as u64 * RATE as u64 / from as u64) as usize;
    let step = from as f64 / RATE as f64;
    (0..n)
        .map(|i| {
            let x = i as f64 * step;
            let a = x as usize;
            let t = (x - a as f64) as f32;
            let p = frames[a.min(frames.len() - 1)];
            let q = frames[(a + 1).min(frames.len() - 1)];
            [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn wav(rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        let spec = hound::WavSpec {
            channels,
            sample_rate: rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::new(&mut out, spec).unwrap();
        for s in samples {
            w.write_sample(*s).unwrap();
        }
        w.finalize().unwrap();
        out.into_inner()
    }

    #[test]
    fn mono_becomes_stereo() {
        let s = decode(Cursor::new(wav(RATE, 1, &[16384, -16384]))).unwrap();
        assert_eq!(s.data, vec![0.5, 0.5, -0.5, -0.5]);
    }

    #[test]
    fn other_rates_are_converted() {
        let src = vec![1000i16; 24_000];
        let s = decode(Cursor::new(wav(24_000, 1, &src))).unwrap();
        // One second at 24 kHz is one second at 48 kHz.
        assert!((s.seconds() - 1.0).abs() < 0.01, "{}", s.seconds());
    }

    #[test]
    fn garbage_is_an_error() {
        assert!(decode(Cursor::new(b"not a wav".to_vec())).is_err());
    }

    #[test]
    fn decodes_an_mp3_to_stereo_at_the_engine_rate() {
        let bytes = include_bytes!("../testdata/tone.mp3");
        let s = decode_mp3(Cursor::new(&bytes[..])).unwrap();
        // About half a second of a 440 Hz tone, left equals right.
        let frames = s.data.len() / 2;
        assert!(frames > RATE as usize * 4 / 10 && frames < RATE as usize * 7 / 10);
        assert!(s.data.chunks_exact(2).all(|f| f[0] == f[1]));
        let peak = s.data.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
        assert!(peak > 0.1 && peak <= 1.01, "peak {peak}");
    }

    #[test]
    fn rejects_a_broken_mp3() {
        assert!(decode_mp3(Cursor::new(b"not an mp3 at all".to_vec())).is_err());
    }

    #[test]
    fn voices_add_up_and_finish() {
        let s = Arc::new(Sample {
            data: vec![0.25; 6],
        });
        let mut voices = vec![
            Voice {
                sample: Arc::clone(&s),
                pos: 0,
                gain: 1.0,
            },
            Voice {
                sample: Arc::clone(&s),
                pos: 2,
                gain: 2.0,
            },
        ];
        let mut out = [0.0f32; 4];
        mix(&mut voices, &mut out);
        assert_eq!(out, [0.75; 4]);
        assert_eq!(voices.len(), 1);
        let mut out = [0.0f32; 4];
        mix(&mut voices, &mut out);
        // Two samples are left of the first voice, none of the second.
        assert_eq!(out, [0.25, 0.25, 0.0, 0.0]);
        assert!(voices.is_empty());
    }
}
