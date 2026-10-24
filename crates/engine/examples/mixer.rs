//! E1 test harness: several inputs, several outputs, one mix per output.
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
//! Runs until Enter is pressed.

use std::io::BufRead;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::Arc;
use std::time::Duration;
use std::{env, io, thread};

use faderust_engine::backend::cpal::CpalBackend;
use faderust_engine::backend::{AudioBackend, DeviceInfo};
use faderust_engine::engine::find_device;
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

fn pick(devices: &[DeviceInfo], pattern: &str, kind: &str) -> Result<DeviceInfo, String> {
    find_device(devices, pattern).ok_or(format!("no {kind} matching \"{pattern}\" (try --list)"))
}

fn run(args: Args) -> Result<(), String> {
    let backend = CpalBackend::new();
    let in_devices = backend.input_devices().map_err(|e| e.to_string())?;
    let out_devices = backend.output_devices().map_err(|e| e.to_string())?;

    if args.list {
        println!("Inputs:");
        for d in &in_devices {
            println!("  {}", d.name);
        }
        println!("Outputs:");
        for d in &out_devices {
            println!("  {}", d.name);
        }
        return Ok(());
    }
    if args.inputs.is_empty() || args.outputs.is_empty() {
        return Err("need at least one --input and one --output".into());
    }

    let mut config = EngineConfig::default();
    let mut names = (vec![], vec![]);
    for p in &args.inputs {
        let d = pick(&in_devices, p, "input")?;
        config.inputs.push(InputConfig { device_id: d.id });
        names.0.push(d.name);
    }
    for (p, ms) in &args.outputs {
        let d = pick(&out_devices, p, "output")?;
        config.outputs.push(OutputConfig {
            device_id: d.id,
            buffer_ms: *ms,
        });
        names.1.push(d.name);
    }

    let engine = Engine::start(&backend, &config).map_err(|e| e.to_string())?;
    let (n_in, n_out) = (config.inputs.len(), config.outputs.len());
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

    for (i, (name, d)) in names.0.iter().zip(&engine.inputs).enumerate() {
        println!(
            "in{i}:  {name} ({} Hz, {} ch)",
            d.format.sample_rate, d.format.channels
        );
    }
    for (o, (name, d)) in names.1.iter().zip(&engine.outputs).enumerate() {
        println!(
            "out{o}: {name} ({} Hz, {} ch, buffer {} ms)",
            d.format.sample_rate, d.format.channels, config.outputs[o].buffer_ms
        );
    }
    println!("Press Enter to stop.\n");

    let stop = Arc::new(AtomicBool::new(false));
    let stop_reader = Arc::clone(&stop);
    thread::spawn(move || {
        let _ = io::stdin().lock().lines().next();
        stop_reader.store(true, Relaxed);
    });

    while !stop.load(Relaxed) {
        thread::sleep(Duration::from_secs(1));
        let mut line = String::new();
        for (i, d) in engine.inputs.iter().enumerate() {
            let [l, r] = d.meter.take();
            line += &format!("in{i} {:>5.1}dB  ", to_db(l.max(r)));
        }
        for (o, d) in engine.outputs.iter().enumerate() {
            let [l, r] = d.meter.take();
            line += &format!("out{o} {:>5.1}dB  ", to_db(l.max(r)));
        }
        println!("{line}");
        for (i, lanes) in engine.lanes.iter().enumerate() {
            for (o, s) in lanes.iter().enumerate() {
                let rate = engine.inputs[i].format.sample_rate as f64;
                println!(
                    "    in{i}->out{o}: fill {:>5.1} ms, drift corr {:>5} ppm, underruns {}, dropped {}",
                    s.fill_frames.load(Relaxed) as f64 * 1000.0 / rate,
                    s.correction_ppm.load(Relaxed),
                    s.underruns.load(Relaxed),
                    s.dropped_frames.load(Relaxed),
                );
            }
        }
        let errors: u64 = engine
            .inputs
            .iter()
            .chain(&engine.outputs)
            .map(|d| d.stream_errors.load(Relaxed))
            .sum();
        if errors > 0 {
            println!("    stream errors: {errors} (device lost? hot-plug comes in E2)");
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
