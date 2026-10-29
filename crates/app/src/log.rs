//! A small log file next to the config (`faderust.log`), so problems on the
//! user's machine leave a trace: engine events, errors and panics. Kept
//! short by starting over when it grows past a limit, also while running.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_BYTES: u64 = 512 * 1024;

static PATH: OnceLock<PathBuf> = OnceLock::new();
/// Bytes in the current file, to start over while the app keeps running.
static SIZE: Mutex<u64> = Mutex::new(0);

/// Starts logging to `path` and records panics there too.
pub fn init(path: PathBuf) {
    let mut size = fs::metadata(&path).map_or(0, |m| m.len());
    if size > MAX_BYTES {
        rotate(&path);
        size = 0;
    }
    *SIZE.lock().unwrap_or_else(|e| e.into_inner()) = size;
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
    write(&format!(
        "Faderust {} gestartet (pid {}, {} {})",
        env!("CARGO_PKG_VERSION"),
        std::process::id(),
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
}

/// Appends one line with a UTC timestamp, and prints it for development.
pub fn write(message: &str) {
    eprintln!("{message}");
    let Some(path) = PATH.get() else { return };
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
    let line = format!("{h:02}:{m:02}:{s:02} UTC  {message}\n");
    let mut size = SIZE.lock().unwrap_or_else(|e| e.into_inner());
    append(path, &line, &mut size);
}

/// Appends `line` to the file at `path` and starts a new file once it has
/// grown past the limit (`size` counts the bytes in it).
fn append(path: &std::path::Path, line: &str, size: &mut u64) {
    if *size > MAX_BYTES {
        rotate(path);
        *size = 0;
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        if file.write_all(line.as_bytes()).is_ok() {
            *size += line.len() as u64;
        }
    }
}

/// The full file becomes the `.old` one, replacing the one before.
fn rotate(path: &std::path::Path) {
    let _ = fs::rename(path, path.with_extension("log.old"));
}

/// The log file, once logging has started.
pub fn path() -> Option<&'static PathBuf> {
    PATH.get()
}

/// The last `lines` lines of the log (and of the previous file when this
/// one is short), oldest first. Empty when nothing was logged yet.
pub fn tail(lines: usize) -> String {
    PATH.get().map_or_else(String::new, |p| tail_of(p, lines))
}

fn tail_of(path: &std::path::Path, lines: usize) -> String {
    let read = |p: &std::path::Path| fs::read_to_string(p).unwrap_or_default();
    let mut all: Vec<String> = read(path).lines().map(str::to_owned).collect();
    if all.len() < lines {
        let mut older: Vec<String> = read(&path.with_extension("log.old"))
            .lines()
            .map(str::to_owned)
            .collect();
        older.append(&mut all);
        all = older;
    }
    let skip = all.len().saturating_sub(lines);
    all.split_off(skip).join("\n")
}

#[cfg(test)]
mod tests {
    use super::{append, tail_of, MAX_BYTES};

    #[test]
    fn tail_takes_the_newest_lines_and_reaches_into_the_old_file() {
        let dir = std::env::temp_dir().join(format!("faderust-log-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("faderust.log");
        std::fs::write(&log, "c\nd\n").unwrap();
        std::fs::write(log.with_extension("log.old"), "a\nb\n").unwrap();
        assert_eq!(tail_of(&log, 2), "c\nd");
        assert_eq!(tail_of(&log, 3), "b\nc\nd");
        assert_eq!(tail_of(&dir.join("missing.log"), 5), "");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_running_log_starts_over_past_the_limit() {
        let dir = std::env::temp_dir().join(format!("faderust-log-roll-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("faderust.log");
        let line = "x".repeat(1023) + "\n";
        let mut size = 0;
        for _ in 0..(MAX_BYTES / 1024 + 10) {
            append(&log, &line, &mut size);
        }
        let now = std::fs::metadata(&log).unwrap().len();
        assert!(now <= MAX_BYTES, "{now}");
        assert_eq!(now, size);
        assert!(log.with_extension("log.old").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
