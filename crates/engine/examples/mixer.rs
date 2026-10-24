//! Engine test harness: several inputs, several outputs, one mix per output,
//! devices plugged in and out at runtime.
//!
//! ```text
//! cargo run --release -p faderust-engine --example mixer -- --list
//! cargo run --release -p faderust-engine --example mixer -- \
//!     --input "USB" --input "CABLE Output" \
//!     --output "PS5:30" --output "Buds:80" \
//!     --gain 1:0:-6 --mute 0:0
//! ```
//!
//! Inputs and outputs are numbered in the order given, starting at 0.
//! Devices may be missing at start and may come and go while it runs.
//! Runs until Enter is pressed.

use std::io::BufRead;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::Arc;
use std::time::Duration;
use std::{env, io, thread};

use faderust_engine::backend::cpal::CpalBackend;
use faderust_engine::backend::AudioBackend;
use faderust_engine::meter::to_db;
use faderust_engine::{Engine, EngineConfig, InputConfig, OutputConfig};

const USAGE: &str = "\
Usage: mixer --list
       mixer --input NAME... --output NAME[:BUFFER_MS]... [--gain I:O:DB]... [--mute I:O]...

  --input NAME           capture device (substring of the name), repeatable
  --output NAME[:MS]     playback device and its buffer in ms (default 40), repeatable
  --gain I:O:DB          gain of input I on output O in dB (default 0)
  --mute I:O             mute input I on output O";

const DEFAULT_BUFFER_MS: u32 = 40;

struct Args {
    list: bool,
    inputs: Vec<String>,
    outputs: Vec<(String, u32)>,
    gains: Vec<(usize, usize, f32)>,
    mutes: Vec<(usize, usize)>,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        list: false,
        inputs: vec![],
        outputs: vec![],
        gains: vec![],
        mutes: vec![],
    };
    let mut it = env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or(format!("missing value for {flag}"));
        match flag.as_str() {
            "--list" => a.list = true,
            "--input" => a.inputs.push(value()?),
            "--output" => {
                let v = value()?;
                a.outputs.push(match v.rsplit_once(':') {
                    Some((name, ms)) if ms.parse::<u32>().is_ok() => {
                        (name.to_string(), ms.parse().unwrap())
                    }
                    _ => (v, DEFAULT_BUFFER_MS),
                });
            }
            "--gain" => {
                let v = value()?;
                let p: Vec<_> = v.splitn(3, ':').collect();
                let parsed = match p[..] {
                    [i, o, db] => i
                        .parse()
                        .ok()
                        .zip(o.parse().ok())
                        .zip(db.parse().ok())
                        .map(|((i, o), db)| (i, o, db)),
                    _ => None,
                };
                a.gains
                    .push(parsed.ok_or(format!("--gain expects I:O:DB, got {v}"))?);
            }
            "--mute" => {
                let v = value()?;
                let parsed = v
                    .split_once(':')
                    .and_then(|(i, o)| i.parse().ok().zip(o.parse().ok()));
                a.mutes
                    .push(parsed.ok_or(format!("--mute expects I:O, got {v}"))?);
            }
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(a)
}

fn run(args: Args) -> Result<(), String> {
    let backend = CpalBackend::new();
    if args.list {
        println!("Inputs:");
        for d in backend.input_devices().map_err(|e| e.to_string())? {
            println!("  {}", d.name);
        }
        println!("Outputs:");
        for d in backend.output_devices().map_err(|e| e.to_string())? {
            println!("  {}", d.name);
        }
        return Ok(());
    }
    if args.inputs.is_empty() || args.outputs.is_empty() {
        return Err("need at least one --input and one --output".into());
    }

    // Devices are matched by name on every check, so a Bluetooth headset
    // that is off now connects as soon as it shows up.
    let config = EngineConfig {
        inputs: args
            .inputs
            .iter()
            .map(|d| InputConfig { device: d.clone() })
            .collect(),
        outputs: args
            .outputs
            .iter()
            .map(|(d, ms)| OutputConfig {
                device: d.clone(),
                buffer_ms: *ms,
            })
            .collect(),
    };
    let (n_in, n_out) = (config.inputs.len(), config.outputs.len());
    let mut engine = Engine::new(backend, config).map_err(|e| e.to_string())?;
    for &(i, o, db) in &args.gains {
        if i >= n_in || o >= n_out {
            return Err(format!("--gain {i}:{o}: no such pair"));
        }
        engine.matrix.set_gain_db(i, o, db);
    }
    for &(i, o) in &args.mutes {
        if i >= n_in || o >= n_out {
            return Err(format!("--mute {i}:{o}: no such pair"));
        }
        engine.matrix.set_muted(i, o, true);
    }
    println!("Press Enter to stop.\n");

    let stop = Arc::new(AtomicBool::new(false));
    let stop_reader = Arc::clone(&stop);
    thread::spawn(move || {
        let _ = io::stdin().lock().lines().next();
        stop_reader.store(true, Relaxed);
    });

    while !stop.load(Relaxed) {
        for event in engine.maintain() {
            println!("* {event}");
        }
        thread::sleep(Duration::from_secs(1));

        let mut line = String::new();
        for (dir, slots) in [("in", engine.inputs()), ("out", engine.outputs())] {
            for (n, s) in slots.iter().enumerate() {
                let [l, r] = s.meter.take();
                match s.device {
                    Some(_) => line += &format!("{dir}{n} {:>6.1}dB  ", to_db(l.max(r))),
                    None => line += &format!("{dir}{n}  offline  "),
                }
            }
        }
        println!("{line}");
        for (i, input) in engine.inputs().iter().enumerate() {
            let Some((_, format)) = &input.device else {
                continue;
            };
            for (o, output) in engine.outputs().iter().enumerate() {
                if output.device.is_none() {
                    continue;
                }
                let s = engine.lane_stats(i, o);
                println!(
                    "    in{i}->out{o}: fill {:>5.1} ms, drift corr {:>5} ppm, underruns {}, dropped {}",
                    s.fill_frames.load(Relaxed) as f64 * 1000.0 / format.sample_rate as f64,
                    s.correction_ppm.load(Relaxed),
                    s.underruns.load(Relaxed),
                    s.dropped_frames.load(Relaxed),
                );
            }
        }
    }
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
