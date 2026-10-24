//! E0 prototype: route one capture device (e.g. a USB mic) to one playback
//! device (e.g. Bluetooth headphones) and measure latency and clock drift.
//!
//! ```text
//! cargo run --release -p faderust-engine --example e0_passthrough -- --list
//! cargo run --release -p faderust-engine --example e0_passthrough -- \
//!     --input "USB" --output "Buds" --minutes 30 --buffer-ms 60 --csv e0.csv
//! ```
//!
//! There is deliberately no resampler here: the ring buffer fill level shows
//! how far the two device clocks run apart. See `docs/E0.md`.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{env, thread};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{
    Device, FromSample, InputCallbackInfo, OutputCallbackInfo, Sample, SampleFormat, SizedSample,
    Stream, StreamConfig,
};
use rtrb::{Consumer, Producer, RingBuffer};

const USAGE: &str = "\
Usage: e0_passthrough [--list] [--input NAME] [--output NAME]
                      [--minutes N] [--buffer-ms N] [--interval-s N] [--csv PATH]

  --list         list audio devices and exit
  --input NAME   capture device, case-insensitive substring (default: system default)
  --output NAME  playback device, case-insensitive substring (default: system default)
  --minutes N    measurement duration (default: 30)
  --buffer-ms N  target ring buffer fill before playback starts (default: 60)
  --interval-s N seconds between reports (default: 10)
  --csv PATH     also write every report line to a CSV file";

/// Counters shared between the audio callbacks and the reporting thread.
/// Only atomics: the callbacks must not allocate or lock.
#[derive(Default)]
struct Stats {
    in_frames: AtomicU64,
    out_frames: AtomicU64,
    /// Input frames lost because the ring buffer was full.
    dropped: AtomicU64,
    /// Output frames filled with silence because the ring buffer was empty.
    underrun: AtomicU64,
    /// How often playback had to wait for the buffer to refill.
    reprimes: AtomicU64,
    /// Ring buffer fill seen by the last output callback, in frames.
    fill: AtomicU64,
    /// Capture instant -> input callback, last value.
    in_lat_ns: AtomicU64,
    /// Output callback -> playback instant, last value.
    out_lat_ns: AtomicU64,
    stream_errors: AtomicU64,
    timestamps_missing: AtomicBool,
}

struct Args {
    list: bool,
    input: Option<String>,
    output: Option<String>,
    minutes: f64,
    buffer_ms: u32,
    interval_s: u64,
    csv: Option<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        list: false,
        input: None,
        output: None,
        minutes: 30.0,
        buffer_ms: 60,
        interval_s: 10,
        csv: None,
    };
    let mut it = env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or(format!("missing value for {flag}"));
        match flag.as_str() {
            "--list" => args.list = true,
            "--input" => args.input = Some(value()?),
            "--output" => args.output = Some(value()?),
            "--minutes" => args.minutes = value()?.parse().map_err(|e| format!("{flag}: {e}"))?,
            "--buffer-ms" => {
                args.buffer_ms = value()?.parse().map_err(|e| format!("{flag}: {e}"))?
            }
            "--interval-s" => {
                args.interval_s = value()?.parse().map_err(|e| format!("{flag}: {e}"))?
            }
            "--csv" => args.csv = Some(value()?),
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    if args.interval_s == 0 || args.buffer_ms == 0 {
        return Err("--interval-s and --buffer-ms must be > 0".into());
    }
    Ok(args)
}

fn device_name(device: &Device) -> String {
    device
        .description()
        .map(|d| d.name().to_string())
        .unwrap_or_else(|_| "<unknown>".into())
}

fn list_devices(host: &cpal::Host) -> Result<(), String> {
    println!("Inputs:");
    for d in host.input_devices().map_err(|e| e.to_string())? {
        let cfg = d.default_input_config().map(|c| {
            format!(
                "{} Hz, {} ch, {:?}",
                c.sample_rate(),
                c.channels(),
                c.sample_format()
            )
        });
        println!(
            "  {}  ({})",
            device_name(&d),
            cfg.unwrap_or_else(|e| e.to_string())
        );
    }
    println!("Outputs:");
    for d in host.output_devices().map_err(|e| e.to_string())? {
        let cfg = d.default_output_config().map(|c| {
            format!(
                "{} Hz, {} ch, {:?}",
                c.sample_rate(),
                c.channels(),
                c.sample_format()
            )
        });
        println!(
            "  {}  ({})",
            device_name(&d),
            cfg.unwrap_or_else(|e| e.to_string())
        );
    }
    Ok(())
}

fn find_device(
    devices: impl Iterator<Item = Device>,
    default: Option<Device>,
    pattern: Option<&str>,
    kind: &str,
) -> Result<Device, String> {
    let Some(pattern) = pattern else {
        return default.ok_or(format!("no default {kind} device"));
    };
    let needle = pattern.to_lowercase();
    devices
        .into_iter()
        .find(|d| device_name(d).to_lowercase().contains(&needle))
        .ok_or(format!(
            "no {kind} device matching \"{pattern}\" (try --list)"
        ))
}

fn input_callback<T>(
    mut producer: Producer<f32>,
    channels: usize,
    stats: Arc<Stats>,
) -> impl FnMut(&[T], &InputCallbackInfo) + Send + 'static
where
    T: Sample,
    f32: FromSample<T>,
{
    move |data, info| {
        let ts = info.timestamp();
        match ts.callback.checked_duration_since(ts.capture) {
            Some(d) => stats.in_lat_ns.store(d.as_nanos() as u64, Relaxed),
            None => stats.timestamps_missing.store(true, Relaxed),
        }
        let mut dropped = 0;
        for frame in data.chunks_exact(channels) {
            // Downmix to mono; the mic is mono in practice.
            let sum: f32 = frame.iter().map(|s| f32::from_sample(*s)).sum();
            if producer.push(sum / channels as f32).is_err() {
                dropped += 1;
            }
        }
        stats
            .in_frames
            .fetch_add((data.len() / channels) as u64, Relaxed);
        if dropped > 0 {
            stats.dropped.fetch_add(dropped, Relaxed);
        }
    }
}

fn output_callback<T>(
    mut consumer: Consumer<f32>,
    channels: usize,
    prefill: usize,
    stats: Arc<Stats>,
) -> impl FnMut(&mut [T], &OutputCallbackInfo) + Send + 'static
where
    T: Sample + FromSample<f32>,
{
    let mut primed = false;
    move |data, info| {
        let ts = info.timestamp();
        match ts.playback.checked_duration_since(ts.callback) {
            Some(d) => stats.out_lat_ns.store(d.as_nanos() as u64, Relaxed),
            None => stats.timestamps_missing.store(true, Relaxed),
        }
        let available = consumer.slots();
        stats.fill.store(available as u64, Relaxed);
        stats
            .out_frames
            .fetch_add((data.len() / channels) as u64, Relaxed);

        if !primed {
            if available < prefill {
                data.fill(T::EQUILIBRIUM);
                return;
            }
            primed = true;
        }
        let mut underrun = 0;
        for frame in data.chunks_exact_mut(channels) {
            let s = consumer.pop().unwrap_or_else(|_| {
                underrun += 1;
                0.0
            });
            frame.fill(T::from_sample(s));
        }
        if underrun > 0 {
            // Wait for the target fill again instead of crackling along empty.
            primed = false;
            stats.underrun.fetch_add(underrun, Relaxed);
            stats.reprimes.fetch_add(1, Relaxed);
        }
    }
}

fn error_callback(stats: &Arc<Stats>) -> impl FnMut(cpal::Error) + Send + 'static {
    let stats = Arc::clone(stats);
    move |_| {
        stats.stream_errors.fetch_add(1, Relaxed);
    }
}

fn build_input(
    device: &Device,
    config: StreamConfig,
    format: SampleFormat,
    producer: Producer<f32>,
    stats: &Arc<Stats>,
) -> Result<Stream, String> {
    fn build<T>(
        device: &Device,
        config: StreamConfig,
        producer: Producer<f32>,
        stats: &Arc<Stats>,
    ) -> Result<Stream, cpal::Error>
    where
        T: SizedSample,
        f32: FromSample<T>,
    {
        let channels = config.channels as usize;
        let cb = input_callback::<T>(producer, channels, Arc::clone(stats));
        device.build_input_stream(config, cb, error_callback(stats), None)
    }
    match format {
        SampleFormat::F32 => build::<f32>(device, config, producer, stats),
        SampleFormat::I16 => build::<i16>(device, config, producer, stats),
        SampleFormat::I32 => build::<i32>(device, config, producer, stats),
        SampleFormat::U16 => build::<u16>(device, config, producer, stats),
        other => return Err(format!("unsupported input sample format {other:?}")),
    }
    .map_err(|e| format!("input stream: {e}"))
}

fn build_output(
    device: &Device,
    config: StreamConfig,
    format: SampleFormat,
    consumer: Consumer<f32>,
    prefill: usize,
    stats: &Arc<Stats>,
) -> Result<Stream, String> {
    fn build<T>(
        device: &Device,
        config: StreamConfig,
        consumer: Consumer<f32>,
        prefill: usize,
        stats: &Arc<Stats>,
    ) -> Result<Stream, cpal::Error>
    where
        T: SizedSample + FromSample<f32>,
    {
        let channels = config.channels as usize;
        let cb = output_callback::<T>(consumer, channels, prefill, Arc::clone(stats));
        device.build_output_stream(config, cb, error_callback(stats), None)
    }
    match format {
        SampleFormat::F32 => build::<f32>(device, config, consumer, prefill, stats),
        SampleFormat::I16 => build::<i16>(device, config, consumer, prefill, stats),
        SampleFormat::I32 => build::<i32>(device, config, consumer, prefill, stats),
        SampleFormat::U16 => build::<u16>(device, config, consumer, prefill, stats),
        other => return Err(format!("unsupported output sample format {other:?}")),
    }
    .map_err(|e| format!("output stream: {e}"))
}

/// Running min/max/mean of the estimated end-to-end latency.
#[derive(Default)]
struct Summary {
    n: u32,
    sum_ms: f64,
    min_ms: f64,
    max_ms: f64,
}

impl Summary {
    fn add(&mut self, ms: f64) {
        if self.n == 0 || ms < self.min_ms {
            self.min_ms = ms;
        }
        if self.n == 0 || ms > self.max_ms {
            self.max_ms = ms;
        }
        self.n += 1;
        self.sum_ms += ms;
    }
}

fn run(args: Args) -> Result<(), String> {
    let host = cpal::default_host();
    if args.list {
        return list_devices(&host);
    }

    let input = find_device(
        host.input_devices().map_err(|e| e.to_string())?,
        host.default_input_device(),
        args.input.as_deref(),
        "input",
    )?;
    let output = find_device(
        host.output_devices().map_err(|e| e.to_string())?,
        host.default_output_device(),
        args.output.as_deref(),
        "output",
    )?;

    let in_cfg = input.default_input_config().map_err(|e| e.to_string())?;
    let out_cfg = output.default_output_config().map_err(|e| e.to_string())?;
    let rate = in_cfg.sample_rate();
    // Run the output at the input's nominal rate. WASAPI converts if the
    // device mix format differs, so only the real clock drift remains.
    let out_stream_cfg = StreamConfig {
        channels: out_cfg.channels(),
        sample_rate: rate,
        buffer_size: cpal::BufferSize::Default,
    };

    let prefill = (rate as u64 * args.buffer_ms as u64 / 1000) as usize;
    let capacity = (prefill * 4).max(rate as usize / 2);
    let (producer, consumer) = RingBuffer::<f32>::new(capacity);
    let stats = Arc::new(Stats::default());

    println!(
        "Input:  {} ({} Hz, {} ch, {:?})",
        device_name(&input),
        rate,
        in_cfg.channels(),
        in_cfg.sample_format()
    );
    println!(
        "Output: {} ({} Hz -> device {} Hz, {} ch, {:?})",
        device_name(&output),
        rate,
        out_cfg.sample_rate(),
        out_cfg.channels(),
        out_cfg.sample_format()
    );
    println!(
        "Target buffer {} ms, running {} min, report every {} s\n",
        args.buffer_ms, args.minutes, args.interval_s
    );

    let in_stream = build_input(
        &input,
        in_cfg.config(),
        in_cfg.sample_format(),
        producer,
        &stats,
    )?;
    let out_stream = build_output(
        &output,
        out_stream_cfg,
        out_cfg.sample_format(),
        consumer,
        prefill,
        &stats,
    )?;
    in_stream.play().map_err(|e| e.to_string())?;
    out_stream.play().map_err(|e| e.to_string())?;

    let mut csv = match &args.csv {
        Some(path) => {
            let mut w = BufWriter::new(File::create(path).map_err(|e| format!("{path}: {e}"))?);
            writeln!(
                w,
                "t_s,in_hz,out_hz,drift_ppm,fill_ms,in_lat_ms,out_lat_ms,total_ms,dropped,underrun,reprimes,errors"
            )
            .map_err(|e| e.to_string())?;
            Some(w)
        }
        None => None,
    };

    println!(
        "{:>7} {:>10} {:>10} {:>9} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>6}",
        "t[s]",
        "in[Hz]",
        "out[Hz]",
        "drift",
        "fill",
        "in_lat",
        "out_lat",
        "total",
        "dropped",
        "underrun",
        "errors"
    );

    // Measure rates from a baseline taken after the streams settled.
    let warmup = Duration::from_secs(3);
    thread::sleep(warmup);
    let t0 = Instant::now();
    let in0 = stats.in_frames.load(Relaxed);
    let out0 = stats.out_frames.load(Relaxed);
    let end = t0 + Duration::from_secs_f64(args.minutes * 60.0);
    let interval = Duration::from_secs(args.interval_s);
    let mut summary = Summary::default();
    let mut drift_ppm = 0.0;
    let mut next = t0 + interval;

    while next <= end {
        thread::sleep(next.saturating_duration_since(Instant::now()));
        next += interval;

        let elapsed = t0.elapsed().as_secs_f64();
        let in_hz = (stats.in_frames.load(Relaxed) - in0) as f64 / elapsed;
        let out_hz = (stats.out_frames.load(Relaxed) - out0) as f64 / elapsed;
        // Positive: the input clock runs faster than the output clock.
        drift_ppm = (in_hz - out_hz) / out_hz * 1e6;
        let fill_ms = stats.fill.load(Relaxed) as f64 * 1000.0 / rate as f64;
        let in_lat_ms = stats.in_lat_ns.load(Relaxed) as f64 / 1e6;
        let out_lat_ms = stats.out_lat_ns.load(Relaxed) as f64 / 1e6;
        let total_ms = in_lat_ms + fill_ms + out_lat_ms;
        summary.add(total_ms);
        let dropped = stats.dropped.load(Relaxed);
        let underrun = stats.underrun.load(Relaxed);
        let reprimes = stats.reprimes.load(Relaxed);
        let errors = stats.stream_errors.load(Relaxed);

        println!(
            "{elapsed:>7.0} {in_hz:>10.2} {out_hz:>10.2} {drift_ppm:>7.1}pp {fill_ms:>6.1}ms {in_lat_ms:>6.1}ms {out_lat_ms:>6.1}ms {total_ms:>6.1}ms {dropped:>8} {underrun:>8} {errors:>6}"
        );
        if let Some(w) = csv.as_mut() {
            writeln!(
                w,
                "{elapsed:.1},{in_hz:.3},{out_hz:.3},{drift_ppm:.2},{fill_ms:.2},{in_lat_ms:.2},{out_lat_ms:.2},{total_ms:.2},{dropped},{underrun},{reprimes},{errors}"
            )
            .map_err(|e| e.to_string())?;
            w.flush().map_err(|e| e.to_string())?;
        }
    }

    drop(in_stream);
    drop(out_stream);

    let mean_ms = summary.sum_ms / summary.n.max(1) as f64;
    println!("\nSummary");
    println!(
        "  drift:        {drift_ppm:.1} ppm  (= {:.1} ms per hour)",
        drift_ppm.abs() * 3.6
    );
    println!(
        "  latency est.: mean {mean_ms:.1} ms, min {:.1} ms, max {:.1} ms",
        summary.min_ms, summary.max_ms
    );
    println!(
        "  dropped {} frames, underrun {} frames, {} re-primes, {} stream errors",
        stats.dropped.load(Relaxed),
        stats.underrun.load(Relaxed),
        stats.reprimes.load(Relaxed),
        stats.stream_errors.load(Relaxed)
    );
    if stats.timestamps_missing.load(Relaxed) {
        println!("  note: the host did not report all stream timestamps; device latency may be 0");
    }
    println!("  (device latency excludes Bluetooth codec delay; see docs/E0.md)");
    Ok(())
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("error: {e}\n");
            }
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
