//! Faderust's own virtual cable: a USB sound card served over USB/IP by
//! [`faderust_usbcable`], so no audio driver of ours is needed. Windows
//! attaches it through usbip-win2 (BSD-2, signed by Microsoft), which a
//! one-time setup installs (without an entry of its own in Windows' app list)
//! together with scheduled tasks that attach the cables without asking for
//! administrator rights again. Faderust can remove it all again.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use faderust_usbcable::device::MAX_CABLES;
use faderust_usbcable::server::{Server, Stopper, DEFAULT_PORT};
use serde::Serialize;

use crate::log;

/// Name of the scheduled task that attaches cable `cable` (counted from 0).
#[cfg_attr(not(windows), allow(dead_code))]
fn task_name(cable: usize) -> String {
    match cable {
        0 => "Faderust Cable".to_string(),
        n => format!("Faderust Cable {}", n + 1),
    }
}

/// How long a look at the scheduled tasks stays valid.
const INSTALLED_FOR: Duration = Duration::from_secs(30);
/// How often the watcher attaches a cable that is not attached.
const REATTACH_EVERY: Duration = Duration::from_secs(10);

/// The server's stopper and the number of cables it offers.
static RUNNING: Mutex<Option<(Stopper, usize)>> = Mutex::new(None);
/// Last answer of [`installed`]: when, for how many cables, and the result.
static INSTALLED: Mutex<Option<(Instant, usize, bool)>> = Mutex::new(None);
/// Number of the server run that is current; a watcher of an older one ends.
static GENERATION: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// Only Windows can attach the cable.
    pub supported: bool,
    /// usbip-win2 and the attach task are installed.
    pub installed: bool,
    /// The cable's server is running.
    pub running: bool,
    /// Number of cables on offer (while running, else the configured one).
    pub count: usize,
    /// For each cable: a host is attached to it.
    pub attached: Vec<bool>,
}

/// Number of cables from the configuration, kept within the limits.
pub fn clamp_count(count: u8) -> usize {
    (count as usize).clamp(1, MAX_CABLES)
}

pub fn status(configured: u8) -> Status {
    let running = RUNNING.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let count = running
        .as_ref()
        .map_or(clamp_count(configured), |(_, n)| *n);
    Status {
        supported: cfg!(windows),
        installed: installed(count),
        running: running.is_some(),
        count,
        attached: (0..count)
            .map(|i| running.as_ref().is_some_and(|(s, _)| s.attached(i)))
            .collect(),
    }
}

/// Binds the server's port; a server that was stopped a moment ago may
/// still hold it.
fn bind(count: usize) -> Result<Server, String> {
    let mut last = None;
    for _ in 0..10 {
        match Server::bind_cables(DEFAULT_PORT, count) {
            Ok(server) => return Ok(server),
            Err(e) => last = Some(e),
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(format!(
        "{} {DEFAULT_PORT}: {}",
        faderust_engine::lang::tr("Port belegt:", "Port in use:"),
        last.map(|e| e.to_string()).unwrap_or_default()
    ))
}

/// Starts the server for `count` cables (and its attach watcher) unless it
/// runs.
pub fn start(count: usize) -> Result<(), String> {
    let count = count.clamp(1, MAX_CABLES);
    let mut running = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
    if running.is_some() {
        return Ok(());
    }
    let server = bind(count)?;
    let stopper = server.stopper();
    *running = Some((stopper.clone(), count));
    log::write(&format!("USB-Kabel gestartet ({count})"));
    std::thread::Builder::new()
        .name("faderust-usbcable".into())
        .spawn(move || {
            if let Err(e) = server.run() {
                log::write(&format!("USB-Kabel beendet: {e}"));
            }
        })
        .map_err(|e| e.to_string())?;
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    std::thread::Builder::new()
        .name("faderust-usbcable-attach".into())
        .spawn(move || {
            // Attaches now, and again whenever the host dropped a cable
            // (sleep, a restart of the audio service).
            while GENERATION.load(Ordering::SeqCst) == generation {
                if installed(count) {
                    for cable in 0..count {
                        if !stopper.attached(cable) {
                            attach(cable);
                        }
                    }
                }
                std::thread::sleep(REATTACH_EVERY);
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn stop() {
    GENERATION.fetch_add(1, Ordering::SeqCst);
    let stopper = RUNNING.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some((stopper, _)) = stopper {
        stopper.stop();
        log::write("USB-Kabel gestoppt");
    }
}

#[cfg(windows)]
mod sys {
    use std::os::windows::process::CommandExt;
    use std::path::PathBuf;
    use std::process::Command;

    use super::{task_name, MAX_CABLES};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    pub fn usbip() -> PathBuf {
        PathBuf::from(
            std::env::var_os("ProgramFiles").unwrap_or_else(|| "C:\\Program Files".into()),
        )
        .join("USBip")
        .join("usbip.exe")
    }

    fn hidden(program: &str) -> Command {
        let mut c = Command::new(program);
        c.creation_flags(CREATE_NO_WINDOW);
        c
    }

    /// usbip-win2's entry in Windows' app list is still shown, i.e. it was
    /// installed before Faderust started hiding it. Running the setup again
    /// marks it as a system component.
    fn entry_visible() -> bool {
        const ROOTS: [&str; 2] = [
            r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
            r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        ];
        ROOTS.iter().any(|root| {
            let Ok(found) = hidden("reg")
                .args(["query", root, "/s", "/f", "USBip", "/d"])
                .output()
            else {
                return false;
            };
            let found = String::from_utf8_lossy(&found.stdout);
            let mut keys: Vec<&str> = found
                .lines()
                .map(str::trim)
                .filter(|l| l.starts_with("HKEY_"))
                .collect();
            keys.dedup();
            keys.into_iter().any(|key| {
                !hidden("reg")
                    .args(["query", key, "/v", "SystemComponent"])
                    .output()
                    .is_ok_and(|o| o.status.success())
            })
        })
    }

    /// usbip-win2 is installed and the tasks of the first `count` cables exist.
    pub fn installed(count: usize) -> bool {
        usbip().exists()
            && !entry_visible()
            && (0..count.min(MAX_CABLES)).all(|i| {
                hidden("schtasks")
                    .args(["/query", "/tn", &task_name(i)])
                    .output()
                    .is_ok_and(|o| o.status.success())
            })
    }

    pub fn attach(cable: usize) {
        let _ = hidden("schtasks")
            .args(["/run", "/tn", &task_name(cable)])
            .output();
    }

    /// Powershell file that installs usbip-win2 and registers the task.
    /// Runs elevated; writes what went wrong to `result.txt` next to it.
    const SCRIPT: &str = include_str!("cable-setup.ps1");

    pub fn run_script(action: &str) -> Result<(), String> {
        let dir = std::env::temp_dir().join("faderust-cable-setup");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let script = dir.join("setup.ps1");
        // Windows PowerShell 5 reads a file without byte order mark as ANSI.
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(SCRIPT.as_bytes());
        std::fs::write(&script, bytes).map_err(|e| e.to_string())?;
        let user = format!(
            "{}\\{}",
            std::env::var("USERDOMAIN").unwrap_or_default(),
            std::env::var("USERNAME").unwrap_or_default()
        );
        let quote = |s: &str| s.replace('\'', "''");
        let command = format!(
            "$p = Start-Process powershell -Verb RunAs -Wait -PassThru -WindowStyle Hidden \
             -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File','\"{}\"','\"{}\"','{}','{}'; \
             exit $p.ExitCode",
            quote(&script.to_string_lossy()),
            quote(&user),
            MAX_CABLES,
            quote(action)
        );
        let output = hidden("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &command])
            .output()
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            return Ok(());
        }
        let why = std::fs::read_to_string(dir.join("result.txt")).unwrap_or_default();
        Err(if why.trim().is_empty() {
            // Declining the administrator prompt ends up here.
            let stderr = String::from_utf8_lossy(&output.stderr);
            stderr.trim().to_string()
        } else {
            why.trim().to_string()
        })
    }
}

#[cfg(not(windows))]
mod sys {
    pub fn installed(_count: usize) -> bool {
        false
    }
    pub fn attach(_cable: usize) {}
    pub fn run_script(_action: &str) -> Result<(), String> {
        Err(faderust_engine::lang::tr(
            "Das USB-Kabel gibt es nur unter Windows.",
            "The USB cable is only available on Windows.",
        ))
    }
}

/// Whether the cables can be attached: usbip-win2 and the tasks are there.
/// Looked at every 30 s at most, since it starts a process per cable.
fn installed(count: usize) -> bool {
    let mut last = INSTALLED.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((at, n, answer)) = *last {
        if n == count && at.elapsed() < INSTALLED_FOR {
            return answer;
        }
    }
    let answer = sys::installed(count);
    *last = Some((Instant::now(), count, answer));
    answer
}

fn attach(cable: usize) {
    sys::attach(cable);
}

/// Installs usbip-win2 and the attach task (one administrator prompt).
/// Blocks until the setup is done.
pub fn setup() -> Result<(), String> {
    log::write("USB-Kabel: Einrichtung gestartet");
    let result = sys::run_script("install");
    // The tasks are new: look again at once.
    *INSTALLED.lock().unwrap_or_else(|e| e.into_inner()) = None;
    match &result {
        Ok(()) => log::write("USB-Kabel: Einrichtung fertig"),
        Err(e) => log::write(&format!("USB-Kabel: Einrichtung fehlgeschlagen: {e}")),
    }
    result
}

/// Removes usbip-win2 and the attach tasks (one administrator prompt).
pub fn remove() -> Result<(), String> {
    log::write("USB-Kabel: Entfernen gestartet");
    let result = sys::run_script("remove");
    *INSTALLED.lock().unwrap_or_else(|e| e.into_inner()) = None;
    match &result {
        Ok(()) => log::write("USB-Kabel: entfernt"),
        Err(e) => log::write(&format!("USB-Kabel: Entfernen fehlgeschlagen: {e}")),
    }
    result
}
