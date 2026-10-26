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
    use super::tail_of;

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
}
