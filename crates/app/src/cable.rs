//! Faderust's own virtual cable: a USB sound card served over USB/IP by
//! [`faderust_usbcable`], so no audio driver of ours is needed. Windows
//! attaches it through usbip-win2 (BSD-2, signed by Microsoft), which a
//! one-time setup installs together with a scheduled task that attaches the
//! cable without asking for administrator rights again.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use faderust_usbcable::server::{Server, Stopper, BUSID, DEFAULT_PORT};
use serde::Serialize;

use crate::log;

const TASK: &str = "Faderust Cable";
/// How often the watcher attaches a cable that is not attached.
const REATTACH_EVERY: Duration = Duration::from_secs(10);

static RUNNING: Mutex<Option<Stopper>> = Mutex::new(None);
/// Ends the watcher of the server that is running.
static WATCH: AtomicBool = AtomicBool::new(false);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// Only Windows can attach the cable.
    pub supported: bool,
    /// usbip-win2 and the attach task are installed.
    pub installed: bool,
    /// The cable's server is running.
    pub running: bool,
    /// A host is attached to it.
    pub attached: bool,
}

pub fn status() -> Status {
    let running = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
    Status {
        supported: cfg!(windows),
        installed: installed(),
        running: running.is_some(),
        attached: running.as_ref().is_some_and(Stopper::attached),
    }
}

/// Starts the cable's server (and its attach watcher) unless it runs.
pub fn start() -> Result<(), String> {
    let mut running = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
    if running.is_some() {
        return Ok(());
    }
    let server = Server::bind(DEFAULT_PORT).map_err(|e| {
        format!(
            "{} {DEFAULT_PORT}: {e}",
            faderust_engine::lang::tr("Port belegt:", "Port in use:")
        )
    })?;
    let stopper = server.stopper();
    *running = Some(stopper.clone());
    log::write("USB-Kabel gestartet");
    std::thread::Builder::new()
        .name("faderust-usbcable".into())
        .spawn(move || {
            if let Err(e) = server.run() {
                log::write(&format!("USB-Kabel beendet: {e}"));
            }
        })
        .map_err(|e| e.to_string())?;
    WATCH.store(true, Ordering::SeqCst);
    std::thread::Builder::new()
        .name("faderust-usbcable-attach".into())
        .spawn(move || {
            // Attaches now, and again whenever the host dropped the cable
            // (sleep, a restart of the audio service).
            while WATCH.load(Ordering::SeqCst) {
                if !stopper.attached() && installed() {
                    attach();
                }
                std::thread::sleep(REATTACH_EVERY);
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn stop() {
    WATCH.store(false, Ordering::SeqCst);
    let stopper = RUNNING.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(stopper) = stopper {
        stopper.stop();
        log::write("USB-Kabel gestoppt");
    }
}

#[cfg(windows)]
mod sys {
    use std::os::windows::process::CommandExt;
    use std::path::PathBuf;
    use std::process::Command;

    use super::TASK;

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

    pub fn installed() -> bool {
        usbip().exists()
            && hidden("schtasks")
                .args(["/query", "/tn", TASK])
                .output()
                .is_ok_and(|o| o.status.success())
    }

    pub fn attach() {
        let _ = hidden("schtasks").args(["/run", "/tn", TASK]).output();
    }

    /// Powershell file that installs usbip-win2 and registers the task.
    /// Runs elevated; writes what went wrong to `result.txt` next to it.
    const SCRIPT: &str = include_str!("cable-setup.ps1");

    pub fn setup(busid: &str) -> Result<(), String> {
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
             -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File','\"{}\"','\"{}\"','{}'; \
             exit $p.ExitCode",
            quote(&script.to_string_lossy()),
            quote(&user),
            quote(busid)
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
    pub fn installed() -> bool {
        false
    }
    pub fn attach() {}
    pub fn setup(_busid: &str) -> Result<(), String> {
        Err(faderust_engine::lang::tr(
            "Das USB-Kabel gibt es nur unter Windows.",
            "The USB cable is only available on Windows.",
        ))
    }
}

fn installed() -> bool {
    sys::installed()
}

fn attach() {
    sys::attach();
}

/// Installs usbip-win2 and the attach task (one administrator prompt).
/// Blocks until the setup is done.
pub fn setup() -> Result<(), String> {
    log::write("USB-Kabel: Einrichtung gestartet");
    let result = sys::setup(BUSID);
    match &result {
        Ok(()) => log::write("USB-Kabel: Einrichtung fertig"),
        Err(e) => log::write(&format!("USB-Kabel: Einrichtung fehlgeschlagen: {e}")),
    }
    result
}
