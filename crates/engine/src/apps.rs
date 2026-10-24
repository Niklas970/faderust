//! Which program plays on which device.
//!
//! Windows lets every program have its own output device (Settings → Sound →
//! Volume mixer). Pointing a program at a virtual cable (VB-Cable) and adding
//! the cable's capture side as an input puts that program on its own fader.
//! Windows remembers the choice per program, so it survives restarts of both
//! the program and Faderust.

#[cfg(windows)]
pub mod wasapi;

use std::fmt;

/// A playback device a program can be pointed at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderDevice {
    /// Endpoint id, stable across reboots.
    pub id: String,
    pub name: String,
}

impl RenderDevice {
    /// True for VB-Audio's virtual cables ("CABLE Input", "CABLE-A Input", …).
    pub fn is_virtual_cable(&self) -> bool {
        is_virtual_cable(&self.name)
    }
}

/// A program that has opened audio, with all its processes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppAudio {
    /// Executable file name, e.g. `spotify.exe`.
    pub exe: String,
    /// Executable name without extension, for display.
    pub name: String,
    pub pids: Vec<u32>,
    /// Endpoint id the program is pinned to; `None` follows the system default.
    pub device: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RouterError(pub String);

impl fmt::Display for RouterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RouterError {}

/// Per-program output routing. Implemented for WASAPI; a PipeWire version
/// would move streams between sinks instead.
pub trait AppRouter: Send + Sync {
    fn render_devices(&self) -> Result<Vec<RenderDevice>, RouterError>;

    /// Programs with an audio session on any active playback device, sorted
    /// by name. System sounds are left out.
    fn apps(&self) -> Result<Vec<AppAudio>, RouterError>;

    /// Pins all `pids` to `device`, or back to the system default for `None`.
    fn set_output(&self, pids: &[u32], device: Option<&str>) -> Result<(), RouterError>;
}

/// True for VB-Audio's virtual cables, by device name. Matches both sides
/// ("CABLE Input" for playback, "CABLE Output" for capture) and the A/B cables.
pub fn is_virtual_cable(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("vb-audio") || n.starts_with("cable")
}

/// Merges `(pid, exe, device)` sessions into one entry per executable.
/// A program counts as pinned if any of its processes is.
pub fn group_sessions(
    sessions: impl IntoIterator<Item = (u32, String, Option<String>)>,
) -> Vec<AppAudio> {
    let mut apps: Vec<AppAudio> = Vec::new();
    for (pid, exe, device) in sessions {
        match apps.iter_mut().find(|a| a.exe.eq_ignore_ascii_case(&exe)) {
            Some(app) => {
                if !app.pids.contains(&pid) {
                    app.pids.push(pid);
                }
                if app.device.is_none() {
                    app.device = device;
                }
            }
            None => apps.push(AppAudio {
                name: exe
                    .rsplit_once('.')
                    .map_or(exe.as_str(), |(stem, _)| stem)
                    .to_string(),
                exe,
                pids: vec![pid],
                device,
            }),
        }
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_vb_cables() {
        assert!(is_virtual_cable("CABLE Input (VB-Audio Virtual Cable)"));
        assert!(is_virtual_cable("CABLE-A Output (VB-Audio Cable A)"));
        assert!(is_virtual_cable("cable input"));
        assert!(!is_virtual_cable("Lautsprecher (USB Sound Card)"));
        assert!(!is_virtual_cable("Kopfhörer (Cable Guy Headset)"));
    }

    #[test]
    fn groups_processes_by_exe() {
        let apps = group_sessions([
            (10, "spotify.exe".into(), None),
            (20, "firefox.exe".into(), None),
            (21, "Firefox.exe".into(), Some("cable".into())),
            (20, "firefox.exe".into(), None),
            (30, "Discord.exe".into(), None),
        ]);
        let names: Vec<_> = apps.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["Discord", "firefox", "spotify"]);
        assert_eq!(apps[1].pids, [20, 21]);
        assert_eq!(apps[1].device.as_deref(), Some("cable"));
        assert_eq!(apps[2].device, None);
    }
}
