//! A small log file next to the config (`faderust.log`), so problems on the
//! user's machine leave a trace: engine events, errors and panics. Kept
//! short by starting over when it grows past a limit.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_BYTES: u64 = 512 * 1024;

static PATH: OnceLock<PathBuf> = OnceLock::new();

/// Starts logging to `path` and records panics there too.
pub fn init(path: PathBuf) {
    if fs::metadata(&path).is_ok_and(|m| m.len() > MAX_BYTES) {
        let _ = fs::rename(&path, path.with_extension("log.old"));
    }
    let _ = PATH.set(path);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        write(&format!(
            "panic in {}: {info}",
            thread.name().unwrap_or("unnamed thread")
        ));
        previous(info);
    }));
    write(&format!("Faderust {} gestartet", env!("CARGO_PKG_VERSION")));
}

/// Appends one line with a UTC timestamp, and prints it for development.
pub fn write(message: &str) {
    eprintln!("{message}");
    let Some(path) = PATH.get() else { return };
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{h:02}:{m:02}:{s:02} UTC  {message}");
    }
}
