//! The system's own audio settings: which program plays on which device,
//! program volumes, the default devices and their volume.
//!
//! Windows lets every program have its own output device (Settings → Sound →
//! Volume mixer). Pointing a program at a virtual cable (VB-Cable) and adding
//! the cable's capture side as an input puts that program on its own fader.
//! Windows remembers the choice per program, so it survives restarts of both
//! the program and Faderust.

#[cfg(windows)]
pub mod capture;
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
#[derive(Debug, Clone, PartialEq)]
pub struct AppAudio {
    /// Executable file name, e.g. `spotify.exe`.
    pub exe: String,
    /// Executable name without extension, for display.
    pub name: String,
    pub pids: Vec<u32>,
    /// Endpoint id the program is pinned to; `None` follows the system default.
    pub device: Option<String>,
    /// Program volume in Windows, 0…1.
    pub volume: f32,
    pub muted: bool,
}

/// One audio session as found on a device.
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub pid: u32,
    pub exe: String,
    pub device: Option<String>,
    pub volume: f32,
    pub muted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Playback,
    Recording,
}

/// Windows keeps two defaults per direction: one for everything and one for
/// calls (Discord, Teams, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Default,
    Communications,
}

/// An active playback or recording device with its Windows settings.
#[derive(Debug, Clone, PartialEq)]
pub struct Endpoint {
    pub id: String,
    pub name: String,
    pub flow: Flow,
    pub is_default: bool,
    pub is_default_communications: bool,
    /// Device volume, 0…1.
    pub volume: f32,
    pub muted: bool,
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

    /// Sets the Windows volume (0…1) and/or mute of every session of `pids`.
    fn set_app_volume(
        &self,
        pids: &[u32],
        volume: Option<f32>,
        muted: Option<bool>,
    ) -> Result<(), RouterError>;

    /// Current peak (0…1) of every process playing audio, as Windows meters
    /// it after the program volume.
    fn levels(&self) -> Result<Vec<(u32, f32)>, RouterError>;
}

/// The system's default devices and device volumes.
pub trait SystemAudio: Send + Sync {
    /// Active playback and recording devices.
    fn endpoints(&self) -> Result<Vec<Endpoint>, RouterError>;

    /// Makes `id` the default device of its direction for `role`.
    fn set_default(&self, id: &str, role: Role) -> Result<(), RouterError>;

    /// Device volume (0…1) and/or mute.
    fn set_endpoint_volume(
        &self,
        id: &str,
        volume: Option<f32>,
        muted: Option<bool>,
    ) -> Result<(), RouterError>;
}

/// True for VB-Audio's virtual cables, by device name. Matches both sides
/// ("CABLE Input" for playback, "CABLE Output" for capture) and the A/B cables.
pub fn is_virtual_cable(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("vb-audio") || n.starts_with("cable")
}

/// Merges sessions into one entry per executable. A program counts as
/// pinned if any of its processes is; volume and mute come from its first
/// session.
pub fn group_sessions(sessions: impl IntoIterator<Item = Session>) -> Vec<AppAudio> {
    let mut apps: Vec<AppAudio> = Vec::new();
    for s in sessions {
        match apps.iter_mut().find(|a| a.exe.eq_ignore_ascii_case(&s.exe)) {
            Some(app) => {
                if !app.pids.contains(&s.pid) {
                    app.pids.push(s.pid);
                }
                if app.device.is_none() {
                    app.device = s.device;
                }
            }
            None => apps.push(AppAudio {
                name: s
                    .exe
                    .rsplit_once('.')
                    .map_or(s.exe.as_str(), |(stem, _)| stem)
                    .to_string(),
                exe: s.exe,
                pids: vec![s.pid],
                device: s.device,
                volume: s.volume,
                muted: s.muted,
            }),
        }
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

/// Processes to record a program from. A program often runs as several
/// processes (browsers, Discord) and Windows records a process together with
/// its children, so each session's process is followed up to the topmost
/// parent with the same executable. `processes` lists `(pid, parent, exe)`.
pub fn capture_roots(pids: &[u32], processes: &[(u32, u32, String)]) -> Vec<u32> {
    let find = |pid: u32| processes.iter().find(|(p, ..)| *p == pid);
    let mut roots = Vec::new();
    for &pid in pids {
        let Some(mut current) = find(pid) else {
            continue;
        };
        // Bounded: a reused pid can make the parent chain loop.
        for _ in 0..processes.len() {
            match find(current.1) {
                Some(parent)
                    if parent.0 != current.0 && parent.2.eq_ignore_ascii_case(&current.2) =>
                {
                    current = parent
                }
                _ => break,
            }
        }
        if !roots.contains(&current.0) {
            roots.push(current.0);
        }
    }
    roots
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
        let session = |pid, exe: &str, device: Option<&str>| Session {
            pid,
            exe: exe.into(),
            device: device.map(Into::into),
            volume: pid as f32 / 100.0,
            muted: false,
        };
        let apps = group_sessions([
            session(10, "spotify.exe", None),
            session(20, "firefox.exe", None),
            session(21, "Firefox.exe", Some("cable")),
            session(20, "firefox.exe", None),
            session(30, "Discord.exe", None),
        ]);
        let names: Vec<_> = apps.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["Discord", "firefox", "spotify"]);
        assert_eq!(apps[1].pids, [20, 21]);
        assert_eq!(apps[1].device.as_deref(), Some("cable"));
        assert_eq!(apps[2].device, None);
        assert_eq!(apps[1].volume, 0.2);
    }

    #[test]
    fn capture_roots_follow_same_program_parents() {
        let p = |pid, parent, exe: &str| (pid, parent, exe.to_string());
        let processes = vec![
            p(1, 0, "explorer.exe"),
            p(10, 1, "chrome.exe"),
            p(11, 10, "chrome.exe"), // audio service
            p(12, 10, "chrome.exe"),
            p(20, 1, "Discord.exe"),
            p(21, 20, "discord.exe"),
            p(30, 31, "loop.exe"), // pid reuse: parents point at each other
            p(31, 30, "loop.exe"),
        ];
        assert_eq!(capture_roots(&[11, 12], &processes), vec![10]);
        assert_eq!(capture_roots(&[21, 20], &processes), vec![20]);
        assert_eq!(capture_roots(&[99], &processes), Vec::<u32>::new());
        assert_eq!(capture_roots(&[30], &processes).len(), 1);
    }
}
