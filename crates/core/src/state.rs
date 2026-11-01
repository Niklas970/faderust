//! Everything the window needs from Faderust that does not depend on the
//! window toolkit: shared state, devices, programs, Windows sound, USB cable,
//! presets and the self-test. Each front end calls these.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use faderust_engine::apps::{AppRouter, Flow, Role, SystemAudio};
use faderust_engine::backend::cpal::CpalBackend;
use faderust_engine::backend::AudioBackend;
use serde::Serialize;

use crate::audio::Command;
use crate::config::{Config, InputEntry, OutputEntry};
use crate::{audio, cable, log, remote, selftest, Host};

pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub path: PathBuf,
    pub audio: Mutex<Sender<Command>>,
    /// The connected Stream Deck plugin, see [`remote`].
    pub streamdeck: Mutex<remote::Link>,
    /// The latest snapshot the audio thread sent to the window, for the self-test.
    pub last_state: Arc<Mutex<Option<audio::UiState>>>,
    /// Secret the Stream Deck plugin proves with, see [`remote`]; empty
    /// when the token file could not be written (remote control stays off).
    pub token: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct Device {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct Devices {
    pub inputs: Vec<Device>,
    pub outputs: Vec<Device>,
}

pub fn devices(
    list: Result<Vec<faderust_engine::backend::DeviceInfo>, impl ToString>,
) -> Result<Vec<Device>, String> {
    Ok(list
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|d| Device {
            id: d.id,
            name: d.name,
        })
        .collect())
}

// Async so it runs on a worker thread: the main thread's COM apartment
// belongs to the webview, WASAPI wants its own.
pub fn get_devices() -> Result<Devices, String> {
    let backend = CpalBackend::new();
    Ok(Devices {
        inputs: devices(backend.input_devices())?,
        outputs: devices(backend.output_devices())?,
    })
}

#[derive(Serialize, Clone, Debug)]
pub struct RenderDevice {
    pub id: String,
    pub name: String,
    pub cable: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct App {
    pub name: String,
    pub exe: String,
    pub pids: Vec<u32>,
    pub device: Option<String>,
    pub volume: f32,
    pub muted: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct Apps {
    pub supported: bool,
    pub devices: Vec<RenderDevice>,
    pub apps: Vec<App>,
}

#[cfg(windows)]
pub fn router() -> Option<Box<dyn AppRouter>> {
    Some(Box::new(faderust_engine::apps::wasapi::WasapiRouter::new()))
}

#[cfg(not(windows))]
pub fn router() -> Option<Box<dyn AppRouter>> {
    None
}

/// Programs playing audio and the playback devices they can be pointed at.
pub fn get_apps() -> Result<Apps, String> {
    let Some(router) = router() else {
        return Ok(Apps {
            supported: false,
            devices: vec![],
            apps: vec![],
        });
    };
    let devices = router.render_devices().map_err(|e| e.to_string())?;
    let apps = router.apps().map_err(|e| e.to_string())?;
    Ok(Apps {
        supported: true,
        devices: devices
            .into_iter()
            .map(|d| RenderDevice {
                cable: d.is_virtual_cable(),
                id: d.id,
                name: d.name,
            })
            .collect(),
        apps: apps
            .into_iter()
            .map(|a| App {
                name: a.name,
                exe: a.exe,
                pids: a.pids,
                device: a.device,
                volume: a.volume,
                muted: a.muted,
            })
            .collect(),
    })
}

/// Points a program at a playback device, `None` for the system default.
pub fn set_app_output(pids: Vec<u32>, device: Option<String>) -> Result<(), String> {
    let router = router().ok_or_else(|| {
        faderust_engine::lang::tr("Nur unter Windows verfügbar", "Only available on Windows")
    })?;
    router
        .set_output(&pids, device.as_deref())
        .map_err(|e| e.to_string())
}

/// Sets a program's Windows volume (0…1) and/or mute.
pub fn set_app_volume(
    pids: Vec<u32>,
    volume: Option<f32>,
    muted: Option<bool>,
) -> Result<(), String> {
    let router = router().ok_or_else(|| {
        faderust_engine::lang::tr("Nur unter Windows verfügbar", "Only available on Windows")
    })?;
    router
        .set_app_volume(&pids, volume.map(|v| v.clamp(0.0, 1.0)), muted)
        .map_err(|e| e.to_string())
}

#[derive(Serialize, Clone, Debug)]
pub struct AppLevel {
    pub pid: u32,
    /// Peak in dBFS.
    pub db: f32,
}

/// Peak level of every process playing audio, polled for the program meters.
pub fn get_app_levels() -> Result<Vec<AppLevel>, String> {
    let Some(router) = router() else {
        return Ok(vec![]);
    };
    Ok(router
        .levels()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|(pid, peak)| AppLevel {
            pid,
            db: if peak > 1e-6 {
                20.0 * peak.log10()
            } else {
                -120.0
            },
        })
        .collect())
}

#[derive(Serialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub is_default_communications: bool,
    pub volume: f32,
    pub muted: bool,
}

#[derive(Serialize, PartialEq, Clone, Debug)]
pub struct WindowsAudio {
    pub supported: bool,
    pub playback: Vec<Endpoint>,
    pub recording: Vec<Endpoint>,
}

#[cfg(windows)]
pub fn system_audio() -> Option<Box<dyn SystemAudio>> {
    Some(Box::new(faderust_engine::apps::wasapi::WasapiRouter::new()))
}

#[cfg(not(windows))]
pub fn system_audio() -> Option<Box<dyn SystemAudio>> {
    None
}

/// The Windows default devices and device volumes.
pub fn get_windows_audio() -> Result<WindowsAudio, String> {
    windows_audio()
}

pub fn windows_audio() -> Result<WindowsAudio, String> {
    let Some(sys) = system_audio() else {
        return Ok(WindowsAudio {
            supported: false,
            playback: vec![],
            recording: vec![],
        });
    };
    let mut out = WindowsAudio {
        supported: true,
        playback: vec![],
        recording: vec![],
    };
    for e in sys.endpoints().map_err(|e| e.to_string())? {
        let list = match e.flow {
            Flow::Playback => &mut out.playback,
            Flow::Recording => &mut out.recording,
        };
        list.push(Endpoint {
            id: e.id,
            name: e.name,
            is_default: e.is_default,
            is_default_communications: e.is_default_communications,
            volume: e.volume,
            muted: e.muted,
        });
    }
    Ok(out)
}

/// Makes a device the Windows default, for everything or only for calls.
pub fn set_default_device(id: String, communications: bool) -> Result<(), String> {
    set_default(&id, communications)
}

pub fn set_default(id: &str, communications: bool) -> Result<(), String> {
    let sys = system_audio().ok_or_else(|| {
        faderust_engine::lang::tr("Nur unter Windows verfügbar", "Only available on Windows")
    })?;
    let role = if communications {
        Role::Communications
    } else {
        Role::Default
    };
    sys.set_default(id, role).map_err(|e| e.to_string())
}

/// Sets a device's Windows volume (0…1) and/or mute.
pub fn set_device_volume(
    id: String,
    volume: Option<f32>,
    muted: Option<bool>,
) -> Result<(), String> {
    let sys = system_audio().ok_or_else(|| {
        faderust_engine::lang::tr("Nur unter Windows verfügbar", "Only available on Windows")
    })?;
    sys.set_endpoint_volume(&id, volume.map(|v| v.clamp(0.0, 1.0)), muted)
        .map_err(|e| e.to_string())
}

pub fn get_usb_cable(state: &AppState) -> cable::Status {
    let configured = state
        .config
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .usb_cables;
    cable::status(configured)
}

/// Turns Faderust's USB cable on or off and remembers the choice.
pub fn set_usb_cable(enabled: bool, state: &AppState) -> Result<(), String> {
    let mut config = state.config.lock().unwrap_or_else(|e| e.into_inner());
    if enabled {
        cable::start(cable::clamp_count(config.usb_cables))?;
    } else {
        cable::stop();
    }
    config.usb_cable = enabled;
    config.save(&state.path).map_err(|e| e.to_string())
}

/// How many USB cables to offer; a running server restarts with the new number.
pub fn set_usb_cable_count(count: u8, state: &AppState) -> Result<(), String> {
    let count = cable::clamp_count(count);
    let mut config = state.config.lock().unwrap_or_else(|e| e.into_inner());
    config.usb_cables = count as u8;
    if config.usb_cable {
        cable::stop();
        cable::start(count)?;
    }
    config.save(&state.path).map_err(|e| e.to_string())
}

/// One-time setup of the USB cable: installs usbip-win2 and the attach task.
pub fn setup_usb_cable() -> Result<(), String> {
    cable::setup()
}

/// Removes usbip-win2 and the attach tasks again; turns the cable off first.
pub fn remove_usb_cable(state: &AppState) -> Result<(), String> {
    cable::stop();
    {
        let mut config = state.config.lock().unwrap_or_else(|e| e.into_inner());
        config.usb_cable = false;
        config.save(&state.path).map_err(|e| e.to_string())?;
    }
    cable::remove()
}

/// Name of the plugin file in the app's resources (put there by the release).
pub const PLUGIN_FILE: &str = "com.niklas970.faderust.streamDeckPlugin";
pub const PLUGIN_VERSION_FILE: &str = "streamdeck-plugin.version";

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StreamDeckStatus {
    /// Version of the plugin shipped with this app; `None` in development builds.
    pub bundled: Option<String>,
    /// A plugin is connected right now.
    pub connected: bool,
    /// Its version; `None` for plugins that predate the hello.
    pub installed: Option<String>,
    /// The shipped plugin is newer than the connected one, or one was turned
    /// away (a plugin from before the token).
    pub update: bool,
    /// A plugin tried to connect and was turned away.
    pub refused: bool,
}

pub fn plugin_status(bundled: Option<String>, link: remote::Link) -> StreamDeckStatus {
    let connected = link.clients > 0;
    let refused = link.refused && !connected;
    let update = match (&bundled, &link.version) {
        (Some(_), _) if refused => true,
        (Some(b), Some(v)) if connected => remote::is_newer(b, v),
        _ => false,
    };
    StreamDeckStatus {
        bundled,
        connected,
        installed: link.version,
        update,
        refused,
    }
}

/// Replaces the configuration (after normalising) and tells the engine.
pub fn save_config(config: Config, state: &AppState) -> Result<(), String> {
    let config = config.normalized();
    config.save(&state.path).map_err(|e| {
        format!(
            "{}: {e}",
            faderust_engine::lang::tr("Konfiguration nicht gespeichert", "Configuration not saved")
        )
    })?;
    *state.config.lock().unwrap_or_else(|e| e.into_inner()) = config;
    send(state, Command::Reconfigure)
}

/// Edits the presets and writes the file right away.
pub fn edit_presets(state: &AppState, f: impl FnOnce(&mut Config) -> bool) -> Result<bool, String> {
    let mut config = state.config.lock().unwrap_or_else(|e| e.into_inner());
    let changed = f(&mut config);
    config.save(&state.path).map_err(|e| {
        format!(
            "{}: {e}",
            faderust_engine::lang::tr("Konfiguration nicht gespeichert", "Configuration not saved")
        )
    })?;
    Ok(changed)
}

pub fn save_preset(name: String, state: &AppState) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(faderust_engine::lang::tr(
            "Das Preset braucht einen Namen",
            "The preset needs a name",
        ));
    }
    edit_presets(state, |c| {
        c.save_preset(name);
        true
    })
    .map(drop)
}

pub fn load_preset(name: String, state: &AppState) -> Result<(), String> {
    apply_preset(state, &name)
}

pub fn apply_preset(state: &AppState, name: &str) -> Result<(), String> {
    if !edit_presets(state, |c| c.load_preset(name))? {
        return Err(format!(
            "{} {name}",
            faderust_engine::lang::tr("Preset gibt es nicht:", "Preset does not exist:")
        ));
    }
    send(state, Command::ApplyGains)
}

pub fn delete_preset(name: String, state: &AppState) -> Result<(), String> {
    edit_presets(state, |c| {
        c.delete_preset(&name);
        true
    })
    .map(drop)
}

pub fn send(state: &AppState, cmd: Command) -> Result<(), String> {
    state.audio.lock().unwrap().send(cmd).map_err(|_| {
        faderust_engine::lang::tr(
            "Audio-Engine läuft nicht",
            "The audio engine is not running",
        )
    })
}

/// First start: the system default devices, one mix.
pub fn initial_config() -> Config {
    let backend = CpalBackend::new();
    Config {
        inputs: backend
            .default_input()
            .map(|d| InputEntry {
                label: "Mikrofon".into(),
                device: d.name,
                fx: Default::default(),
                muted: false,
            })
            .into_iter()
            .collect(),
        outputs: backend
            .default_output()
            .map(|d| OutputEntry {
                label: "Ausgang".into(),
                device: d.name,
                buffer_ms: 40,
                master: Default::default(),
                release_secs: 30,
            })
            .into_iter()
            .collect(),
        gains: vec![],
        presets: vec![],
        usb_cable: false,
        usb_cables: 1,
    }
    .normalized()
}

/// Queries devices off the main thread (see [`get_devices`]).
pub fn first_start() -> Config {
    std::thread::spawn(initial_config)
        .join()
        .unwrap_or_default()
}

/// Looks at devices, buffers, remote control and plugin, see [`selftest`].
/// `bundled_plugin` is the version of the Stream Deck plugin shipped with
/// the app, if any. Lists devices, so call it off the UI thread.
pub fn run_selftest(
    state: &AppState,
    bundled_plugin: Option<String>,
) -> Result<Vec<selftest::Check>, String> {
    use selftest::{ConfigFile, Facts, LaneFacts, Slot};

    let backend = CpalBackend::new();
    let names = |list: Result<Vec<faderust_engine::backend::DeviceInfo>, _>| {
        list.ok()
            .map(|l: Vec<faderust_engine::backend::DeviceInfo>| l)
    };
    let (inputs_list, outputs_list) = (
        names(backend.input_devices()),
        names(backend.output_devices()),
    );
    let present = |list: &Option<Vec<faderust_engine::backend::DeviceInfo>>, device: &str| {
        list.as_ref()
            .map(|l| faderust_engine::engine::find_device(l, device).is_some())
    };

    let config = match Config::load(&state.path) {
        Ok(Some(_)) => ConfigFile::Fine,
        Ok(None) => ConfigFile::Missing,
        Err(e) => ConfigFile::Broken(e.to_string()),
    };
    let snapshot = state
        .last_state
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let slots = |devices: Option<&Vec<audio::UiDevice>>, list: &Option<Vec<_>>| -> Vec<Slot> {
        devices
            .into_iter()
            .flatten()
            .map(|d| Slot {
                label: d.label.clone(),
                device: d.device.clone(),
                connected: d.connected,
                released: d.released,
                present: present(list, &d.device),
                buffer_ms: d.buffer_ms,
            })
            .collect()
    };
    let facts = Facts {
        config,
        inputs: slots(snapshot.as_ref().map(|s| &s.inputs), &inputs_list),
        outputs: slots(snapshot.as_ref().map(|s| &s.outputs), &outputs_list),
        lanes: snapshot
            .iter()
            .flat_map(|s| s.lanes.iter())
            .map(|row| {
                row.iter()
                    .map(|l| LaneFacts {
                        underruns: l.underruns,
                        dropped_frames: l.dropped_frames,
                        looped: l.looped,
                    })
                    .collect()
            })
            .collect(),
        token: !state.token.is_empty(),
        link: state
            .streamdeck
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone(),
        bundled_plugin,
    };
    Ok(selftest::evaluate(&facts))
}

/// Hands the shipped plugin to the Stream Deck software, which asks the user
/// to install it (a plugin of the same name is replaced). `source` is the
/// plugin file in the app's resources, `cache_dir` where a copy is made.
pub fn install_streamdeck(source: Option<PathBuf>, cache_dir: &Path) -> Result<(), String> {
    let source = source.ok_or_else(|| {
        faderust_engine::lang::tr(
            "Dieses Faderust enthält kein Stream-Deck-Plugin.",
            "This Faderust does not include the Stream Deck plugin.",
        )
    })?;
    std::fs::create_dir_all(cache_dir).map_err(|e| e.to_string())?;
    let target = cache_dir.join(PLUGIN_FILE);
    std::fs::copy(&source, &target).map_err(|e| {
        format!(
            "{}: {e}",
            faderust_engine::lang::tr("Plugin nicht kopiert", "Plugin not copied")
        )
    })?;
    if cfg!(windows) {
        std::process::Command::new("explorer")
            .arg(&target)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    } else {
        Err(format!(
            "{} {}",
            faderust_engine::lang::tr("Bitte öffnen:", "Please open:"),
            target.display()
        ))
    }
}

/// Shows the folder with `faderust.log` in the file manager.
pub fn open_log_folder() -> Result<(), String> {
    let dir = log::path()
        .and_then(|p| p.parent())
        .ok_or_else(|| "Kein Protokoll vorhanden / No log yet".to_owned())?;
    if cfg!(windows) {
        std::process::Command::new("explorer")
            .arg(dir)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    } else {
        Err(dir.display().to_string())
    }
}

/// The stream deck plugin's state for a link, with the bundled version read
/// from `version_file` (the shipped `streamdeck-plugin.version`).
pub fn streamdeck_status(state: &AppState, version_file: Option<PathBuf>) -> StreamDeckStatus {
    let bundled = version_file
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let link = state
        .streamdeck
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    plugin_status(bundled, link)
}

/// Starts everything that runs without a window: log, configuration, USB
/// cable, audio thread and Stream Deck remote. `dir` is the app's config
/// folder (it is created).
pub fn start(host: Arc<dyn Host>, dir: &Path) -> Arc<AppState> {
    let _ = std::fs::create_dir_all(dir);
    log::init(dir.join("faderust.log"));
    #[cfg(windows)]
    faderust_engine::apps::capture::set_crash_marker(dir.join("capture-starting"));
    let path = dir.join("config.toml");
    let config = match Config::load(&path) {
        Ok(Some(c)) => c,
        Ok(None) => first_start(),
        Err(e) => {
            // Keep the broken file for the user, start fresh.
            log::write(&format!("{}: {e}", path.display()));
            let _ = std::fs::rename(&path, path.with_extension("toml.broken"));
            first_start()
        }
    };
    if config.usb_cable {
        if let Err(e) = cable::start(cable::clamp_count(config.usb_cables)) {
            log::write(&format!("USB-Kabel: {e}"));
        }
    }
    let config = Arc::new(Mutex::new(config));
    let last_state = Arc::default();
    let audio = audio::spawn(
        host,
        Arc::clone(&config),
        path.clone(),
        Arc::clone(&last_state),
    );
    let token = remote::load_or_create_token(&dir.join(remote::TOKEN_FILE)).unwrap_or_else(|e| {
        log::write(&format!("Fernsteuerung aus: Token nicht angelegt: {e}"));
        String::new()
    });
    let state = Arc::new(AppState {
        config,
        path,
        audio: Mutex::new(audio),
        streamdeck: Mutex::default(),
        last_state,
        token,
    });
    remote::spawn(Arc::clone(&state));
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Gain;
    use std::sync::mpsc::{channel, Receiver};

    fn config() -> Config {
        let input = |label: &str| InputEntry {
            label: label.into(),
            device: label.into(),
            fx: Default::default(),
            muted: false,
        };
        Config {
            inputs: vec![input("Mikro"), input("PC")],
            outputs: vec![OutputEntry {
                label: "PS5".into(),
                device: "PS5".into(),
                buffer_ms: 40,
                master: Default::default(),
                release_secs: 30,
            }],
            gains: vec![],
            presets: vec![],
            usb_cable: false,
            usb_cables: 1,
        }
        .normalized()
    }

    /// App state with a throwaway config file; the receiver sees what the
    /// audio engine would be sent.
    fn app(name: &str) -> (AppState, Receiver<Command>, PathBuf) {
        let dir = std::env::temp_dir().join(format!("faderust-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("config.toml");
        let (tx, rx) = channel();
        let state = AppState {
            config: Arc::new(Mutex::new(config())),
            path: path.clone(),
            audio: Mutex::new(tx),
            streamdeck: Mutex::default(),
            last_state: Default::default(),
            token: "secret".into(),
        };
        (state, rx, path)
    }

    fn set_db(state: &AppState, input: usize, db: f32) {
        state.config.lock().unwrap().gains[input][0] = Gain { db, muted: false };
    }

    #[test]
    fn loading_a_preset_restores_the_faders_and_tells_the_engine() {
        let (state, rx, path) = app("preset-load");
        set_db(&state, 0, -6.0);
        edit_presets(&state, |c| {
            c.save_preset("Spielen");
            true
        })
        .unwrap();
        set_db(&state, 0, 0.0);

        apply_preset(&state, "Spielen").unwrap();
        assert_eq!(state.config.lock().unwrap().gains[0][0].db, -6.0);
        assert!(matches!(rx.try_recv(), Ok(Command::ApplyGains)));
        // Written to disk right away, presets included.
        let saved = Config::load(&path).unwrap().unwrap();
        assert_eq!(saved.presets.len(), 1);
        assert_eq!(saved.gains[0][0].db, -6.0);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn unknown_preset_changes_nothing() {
        let (state, rx, path) = app("preset-unknown");
        set_db(&state, 1, -3.0);

        let err = apply_preset(&state, "Gibt es nicht").unwrap_err();
        assert!(err.contains("Gibt es nicht"), "{err}");
        assert_eq!(state.config.lock().unwrap().gains[1][0].db, -3.0);
        assert!(rx.try_recv().is_err(), "the engine must not hear of it");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn failed_save_is_reported_and_the_change_stays_in_memory() {
        let (mut state, _rx, path) = app("save-fails");
        // A file where the folder should be: the save cannot succeed.
        let dir = path.parent().unwrap().to_path_buf();
        std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
        std::fs::write(&dir, "in the way").unwrap();
        state.path = path;

        let err = edit_presets(&state, |c| {
            c.save_preset("Chat");
            true
        })
        .unwrap_err();
        assert!(err.starts_with("Konfiguration nicht gespeichert"), "{err}");
        assert_eq!(state.config.lock().unwrap().presets.len(), 1);
        let _ = std::fs::remove_file(&dir);
    }

    #[test]
    fn commands_fail_clearly_without_an_engine() {
        let (state, rx, path) = app("no-engine");
        assert!(send(&state, Command::Reconfigure).is_ok());
        assert!(matches!(rx.try_recv(), Ok(Command::Reconfigure)));

        drop(rx);
        assert_eq!(
            send(&state, Command::Reconfigure).unwrap_err(),
            "Audio-Engine läuft nicht"
        );
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    fn link(clients: usize, version: Option<&str>, refused: bool) -> remote::Link {
        remote::Link {
            clients,
            version: version.map(Into::into),
            refused,
        }
    }

    #[test]
    fn plugin_update_is_offered_only_when_it_helps() {
        let status = |bundled: Option<&str>, link| plugin_status(bundled.map(Into::into), link);

        // Older plugin connected: update.
        let s = status(Some("0.3.1"), link(1, Some("0.3.0.0"), false));
        assert!(s.connected && s.update && !s.refused);
        // Same version (the plugin writes four numbers): nothing to do.
        let s = status(Some("0.3.1.0"), link(1, Some("0.3.1"), false));
        assert!(s.connected && !s.update);
        // Newer than shipped (plugin installed from a later release).
        assert!(!status(Some("0.3.1"), link(1, Some("0.4.0.0"), false)).update);
        // Not connected: no verdict, the button offers to install.
        let s = status(Some("0.3.1"), link(0, None, false));
        assert!(!s.connected && !s.update);
        // Development build without a shipped plugin: never an update.
        assert!(!status(None, link(1, Some("0.1.0"), false)).update);
    }

    #[test]
    fn a_turned_away_plugin_must_be_updated() {
        let s = plugin_status(Some("0.3.1".into()), link(0, None, true));
        assert!(!s.connected && s.refused && s.update);
        // As soon as one signs in, the refusal no longer counts.
        let s = plugin_status(Some("0.3.1".into()), link(1, Some("0.3.1.0"), true));
        assert!(s.connected && !s.refused && !s.update);
    }
}
