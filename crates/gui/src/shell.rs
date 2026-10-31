//! What surrounds the window: one running instance, autostart, the tray icon
//! and the saved window preferences.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Passed by the Windows autostart entry: start in the tray, no window.
pub const MINIMIZED_ARG: &str = "--minimized";

/// Port the running instance listens on, so a second start can ask it to
/// show its window instead of opening the devices again.
const INSTANCE_PORT: u16 = 47812;

/// Becomes the running instance, or asks the running one to show itself.
/// `Err(())` means another Faderust is running and has been told to show its
/// window: this one should exit. `Ok` carries a receiver that fires when a
/// second start asked for the window.
pub fn single_instance() -> Result<Receiver<()>, ()> {
    match TcpListener::bind((Ipv4Addr::LOCALHOST, INSTANCE_PORT)) {
        Ok(listener) => {
            let (tx, rx) = channel();
            let _ = thread::Builder::new()
                .name("faderust-instance".into())
                .spawn(move || {
                    for mut stream in listener.incoming().flatten() {
                        let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                        let mut buf = [0u8; 8];
                        if stream.read(&mut buf).is_ok_and(|n| &buf[..n] == b"show") {
                            let _ = tx.send(());
                        }
                    }
                });
            Ok(rx)
        }
        Err(_) => {
            if let Ok(mut s) = TcpStream::connect((Ipv4Addr::LOCALHOST, INSTANCE_PORT)) {
                let _ = s.write_all(b"show");
                Err(())
            } else {
                // The port is taken by something else: run anyway.
                Ok(channel().1)
            }
        }
    }
}

/// Value name of the autostart entry; the same the old Tauri version used, so an
/// update keeps working autostart.
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "faderust";

/// Whether Faderust starts with Windows.
pub fn autostart_enabled() -> bool {
    if !cfg!(windows) {
        return false;
    }
    faderust_core::proc::hidden("reg")
        .args(["query", RUN_KEY, "/v", RUN_VALUE])
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Starts Faderust with Windows (in the tray) or not.
pub fn set_autostart(on: bool) -> Result<(), String> {
    if !cfg!(windows) {
        return Err("Autostart gibt es nur unter Windows".into());
    }
    let mut cmd = faderust_core::proc::hidden("reg");
    if on {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        cmd.args([
            "add",
            RUN_KEY,
            "/v",
            RUN_VALUE,
            "/t",
            "REG_SZ",
            "/d",
            &format!("\"{}\" {MINIMIZED_ARG}", exe.display()),
            "/f",
        ]);
    } else {
        cmd.args(["delete", RUN_KEY, "/v", RUN_VALUE, "/f"]);
    }
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() || !on {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// Window preferences that outlive a start (the Tauri app kept them in the
/// browser's local storage).
#[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(default)]
pub struct Prefs {
    /// "desk" or "matrix".
    pub view: String,
    pub masters_right: bool,
    /// Seconds of silence after which a program leaves the desk; `None` is the default.
    pub hide_after: Option<u64>,
    /// Programs kept on the desk even when silent (input device ids).
    pub keep: Vec<String>,
    /// Outputs hidden from the desk (labels).
    pub hidden_outputs: Vec<String>,
    /// "dark", "light" or "system"; empty is dark.
    pub mode: String,
    /// Accent and signal colour as "#rrggbb"; empty is the default.
    pub accent: String,
    pub signal: String,
    /// "de", "en" or empty for the system language.
    pub lang: String,
}

/// "de" or "en", whichever the system uses.
pub fn system_lang() -> &'static str {
    #[cfg(windows)]
    {
        let out = faderust_core::proc::hidden("reg")
            .args([
                "query",
                r"HKCU\Control Panel\International",
                "/v",
                "LocaleName",
            ])
            .output();
        if let Ok(o) = out {
            let t = String::from_utf8_lossy(&o.stdout).to_lowercase();
            if t.contains("reg_sz") && t.contains("en-") {
                return "en";
            }
            return "de";
        }
    }
    let l = std::env::var("LANG").unwrap_or_default().to_lowercase();
    if l.starts_with("en") {
        "en"
    } else {
        "de"
    }
}

pub fn prefs_path(dir: &Path) -> PathBuf {
    dir.join("ui.json")
}

pub fn load_prefs(dir: &Path) -> Prefs {
    std::fs::read_to_string(prefs_path(dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save_prefs(dir: &Path, prefs: &Prefs) {
    if let Ok(t) = serde_json::to_string_pretty(prefs) {
        let _ = std::fs::write(prefs_path(dir), t);
    }
}

/// Shared with the audio thread so a hidden window gets no snapshots.
pub type Visible = Arc<AtomicBool>;

pub fn is_visible(v: &Visible) -> bool {
    v.load(Ordering::Relaxed)
}

#[cfg(windows)]
pub mod tray {
    //! Tray icon with "Anzeigen" and "Beenden"; a left click shows the window.

    use tray_icon::menu::{Menu, MenuEvent, MenuItem};
    use tray_icon::{
        Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    };

    pub struct Tray {
        _icon: TrayIcon,
        show: tray_icon::menu::MenuId,
        quit: tray_icon::menu::MenuId,
    }

    pub enum Action {
        Show,
        Quit,
    }

    impl Tray {
        /// `rgba` is the icon as RGBA pixels of `size` × `size`.
        pub fn new(rgba: Vec<u8>, size: u32) -> Option<Tray> {
            let menu = Menu::new();
            let show = MenuItem::new("Anzeigen", true, None);
            let quit = MenuItem::new("Beenden", true, None);
            menu.append_items(&[&show, &quit]).ok()?;
            let icon = TrayIconBuilder::new()
                .with_tooltip("Faderust")
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .with_icon(Icon::from_rgba(rgba, size, size).ok()?)
                .build()
                .ok()?;
            Some(Tray {
                _icon: icon,
                show: show.id().clone(),
                quit: quit.id().clone(),
            })
        }

        /// What the user did with the icon since the last call.
        pub fn poll(&self) -> Vec<Action> {
            let mut out = vec![];
            while let Ok(e) = MenuEvent::receiver().try_recv() {
                if e.id == self.show {
                    out.push(Action::Show);
                } else if e.id == self.quit {
                    out.push(Action::Quit);
                }
            }
            while let Ok(e) = TrayIconEvent::receiver().try_recv() {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = e
                {
                    out.push(Action::Show);
                }
            }
            out
        }
    }
}
