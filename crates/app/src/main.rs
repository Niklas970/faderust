//! Faderust desktop app: Tauri window and tray around the audio engine.

// No console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod config;
mod log;
mod remote;

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use faderust_engine::apps::{AppRouter, Flow, Role, SystemAudio};
use faderust_engine::backend::cpal::CpalBackend;
use faderust_engine::backend::AudioBackend;
use serde::Serialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, State, WindowEvent};
use tauri_plugin_autostart::ManagerExt;

use audio::Command;
use config::{Config, Fx, InputEntry, OutputEntry};

/// Passed by the Windows autostart entry: start in the tray, no window.
const MINIMIZED_ARG: &str = "--minimized";

struct AppState {
    config: Arc<Mutex<Config>>,
    path: PathBuf,
    audio: Mutex<Sender<Command>>,
    /// The connected Stream Deck plugin, see [`remote`].
    streamdeck: Mutex<remote::Link>,
    /// Secret the Stream Deck plugin proves with, see [`remote`]; empty
    /// when the token file could not be written (remote control stays off).
    token: String,
}

#[derive(Serialize)]
struct Device {
    id: String,
    name: String,
}

#[derive(Serialize)]
struct Devices {
    inputs: Vec<Device>,
    outputs: Vec<Device>,
}

fn devices(
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
#[tauri::command]
async fn get_devices() -> Result<Devices, String> {
    let backend = CpalBackend::new();
    Ok(Devices {
        inputs: devices(backend.input_devices())?,
        outputs: devices(backend.output_devices())?,
    })
}

#[derive(Serialize)]
struct RenderDevice {
    id: String,
    name: String,
    cable: bool,
}

#[derive(Serialize)]
struct App {
    name: String,
    exe: String,
    pids: Vec<u32>,
    device: Option<String>,
    volume: f32,
    muted: bool,
}

#[derive(Serialize)]
struct Apps {
    supported: bool,
    devices: Vec<RenderDevice>,
    apps: Vec<App>,
}

#[cfg(windows)]
fn router() -> Option<Box<dyn AppRouter>> {
    Some(Box::new(faderust_engine::apps::wasapi::WasapiRouter::new()))
}

#[cfg(not(windows))]
fn router() -> Option<Box<dyn AppRouter>> {
    None
}

/// Programs playing audio and the playback devices they can be pointed at.
#[tauri::command]
async fn get_apps() -> Result<Apps, String> {
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
#[tauri::command]
async fn set_app_output(pids: Vec<u32>, device: Option<String>) -> Result<(), String> {
    let router = router().ok_or_else(|| {
        faderust_engine::lang::tr("Nur unter Windows verfügbar", "Only available on Windows")
    })?;
    router
        .set_output(&pids, device.as_deref())
        .map_err(|e| e.to_string())
}

/// Sets a program's Windows volume (0…1) and/or mute.
#[tauri::command]
async fn set_app_volume(
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

#[derive(Serialize)]
struct AppLevel {
    pid: u32,
    /// Peak in dBFS.
    db: f32,
}

/// Peak level of every process playing audio, polled for the program meters.
#[tauri::command]
async fn get_app_levels() -> Result<Vec<AppLevel>, String> {
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

#[derive(Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Endpoint {
    id: String,
    name: String,
    is_default: bool,
    is_default_communications: bool,
    volume: f32,
    muted: bool,
}

#[derive(Serialize, PartialEq)]
struct WindowsAudio {
    supported: bool,
    playback: Vec<Endpoint>,
    recording: Vec<Endpoint>,
}

#[cfg(windows)]
fn system_audio() -> Option<Box<dyn SystemAudio>> {
    Some(Box::new(faderust_engine::apps::wasapi::WasapiRouter::new()))
}

#[cfg(not(windows))]
fn system_audio() -> Option<Box<dyn SystemAudio>> {
    None
}

/// The Windows default devices and device volumes.
#[tauri::command]
async fn get_windows_audio() -> Result<WindowsAudio, String> {
    windows_audio()
}

fn windows_audio() -> Result<WindowsAudio, String> {
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
#[tauri::command]
async fn set_default_device(id: String, communications: bool) -> Result<(), String> {
    set_default(&id, communications)
}

fn set_default(id: &str, communications: bool) -> Result<(), String> {
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
#[tauri::command]
async fn set_device_volume(
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

/// VB-Audio's page with the free VB-Cable download.
const CABLE_PAGE: &str = "https://vb-audio.com/Cable/";

/// Opens the VB-Cable download page in the browser. Faderust must not ship
/// the driver itself.
#[tauri::command]
fn open_cable_page() -> Result<(), String> {
    if cfg!(windows) {
        std::process::Command::new("explorer")
            .arg(CABLE_PAGE)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    } else {
        Err(format!(
            "{} {CABLE_PAGE}",
            faderust_engine::lang::tr("Bitte im Browser öffnen:", "Please open in your browser:")
        ))
    }
}

/// Name of the plugin file in the app's resources (put there by the release).
const PLUGIN_FILE: &str = "com.niklas970.faderust.streamDeckPlugin";
const PLUGIN_VERSION_FILE: &str = "streamdeck-plugin.version";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamDeckStatus {
    /// Version of the plugin shipped with this app; `None` in development builds.
    bundled: Option<String>,
    /// A plugin is connected right now.
    connected: bool,
    /// Its version; `None` for plugins that predate the hello.
    installed: Option<String>,
    /// The shipped plugin is newer than the connected one, or one was turned
    /// away (a plugin from before the token).
    update: bool,
    /// A plugin tried to connect and was turned away.
    refused: bool,
}

fn resource(app: &AppHandle, name: &str) -> Option<PathBuf> {
    app.path()
        .resolve(
            format!("resources/{name}"),
            tauri::path::BaseDirectory::Resource,
        )
        .ok()
        .filter(|p| p.is_file())
}

#[tauri::command]
fn get_streamdeck(app: AppHandle, state: State<AppState>) -> StreamDeckStatus {
    let bundled = resource(&app, PLUGIN_VERSION_FILE)
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

fn plugin_status(bundled: Option<String>, link: remote::Link) -> StreamDeckStatus {
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

/// Hands the shipped plugin to the Stream Deck software, which asks the user
/// to install it (a plugin of the same name is replaced).
#[tauri::command]
fn install_streamdeck(app: AppHandle) -> Result<(), String> {
    let source = resource(&app, PLUGIN_FILE).ok_or_else(|| {
        faderust_engine::lang::tr(
            "Dieses Faderust enthält kein Stream-Deck-Plugin.",
            "This Faderust does not include the Stream Deck plugin.",
        )
    })?;
    let dir = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let target = dir.join(PLUGIN_FILE);
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

/// The interface language ("de" or "en"), for the messages Rust sends.
#[tauri::command]
fn set_language(code: String) {
    faderust_engine::lang::set(&code);
}

#[tauri::command]
fn get_autostart(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_autostart(enabled: bool, app: AppHandle) -> Result<(), String> {
    let autostart = app.autolaunch();
    if enabled {
        autostart.enable()
    } else {
        autostart.disable()
    }
    .map_err(|e| {
        format!(
            "{}: {e}",
            faderust_engine::lang::tr("Autostart nicht geändert", "Autostart not changed")
        )
    })
}

#[tauri::command]
fn get_config(state: State<AppState>) -> Config {
    state
        .config
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

#[tauri::command]
fn save_config(config: Config, state: State<AppState>) -> Result<(), String> {
    let config = config.normalized();
    config.save(&state.path).map_err(|e| {
        format!(
            "{}: {e}",
            faderust_engine::lang::tr("Konfiguration nicht gespeichert", "Configuration not saved")
        )
    })?;
    *state.config.lock().unwrap_or_else(|e| e.into_inner()) = config;
    send(&state, Command::Reconfigure)
}

#[tauri::command]
fn set_gain(input: usize, output: usize, db: f32, state: State<AppState>) -> Result<(), String> {
    send(&state, Command::SetGain(input, output, db))
}

#[tauri::command]
fn set_muted(
    input: usize,
    output: usize,
    muted: bool,
    state: State<AppState>,
) -> Result<(), String> {
    send(&state, Command::SetMuted(input, output, muted))
}

#[tauri::command]
fn set_master_gain(output: usize, db: f32, state: State<AppState>) -> Result<(), String> {
    send(&state, Command::SetMasterGain(output, db))
}

#[tauri::command]
fn set_master_muted(output: usize, muted: bool, state: State<AppState>) -> Result<(), String> {
    send(&state, Command::SetMasterMuted(output, muted))
}

#[tauri::command]
fn set_input_muted(input: usize, muted: bool, state: State<AppState>) -> Result<(), String> {
    send(&state, Command::SetInputMuted(input, muted))
}

/// Changes an input's effects while it plays; saved with the next fader save.
#[tauri::command]
fn set_fx(input: usize, fx: Fx, state: State<AppState>) -> Result<(), String> {
    send(&state, Command::SetFx(input, fx))
}

/// Edits the presets and writes the file right away.
fn edit_presets(state: &AppState, f: impl FnOnce(&mut Config) -> bool) -> Result<bool, String> {
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

#[tauri::command]
fn save_preset(name: String, state: State<AppState>) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(faderust_engine::lang::tr(
            "Das Preset braucht einen Namen",
            "The preset needs a name",
        ));
    }
    edit_presets(&state, |c| {
        c.save_preset(name);
        true
    })
    .map(drop)
}

#[tauri::command]
fn load_preset(name: String, state: State<AppState>) -> Result<(), String> {
    apply_preset(&state, &name)
}

fn apply_preset(state: &AppState, name: &str) -> Result<(), String> {
    if !edit_presets(state, |c| c.load_preset(name))? {
        return Err(format!(
            "{} {name}",
            faderust_engine::lang::tr("Preset gibt es nicht:", "Preset does not exist:")
        ));
    }
    send(state, Command::ApplyGains)
}

#[tauri::command]
fn delete_preset(name: String, state: State<AppState>) -> Result<(), String> {
    edit_presets(&state, |c| {
        c.delete_preset(&name);
        true
    })
    .map(drop)
}

fn send(state: &AppState, cmd: Command) -> Result<(), String> {
    state.audio.lock().unwrap().send(cmd).map_err(|_| {
        faderust_engine::lang::tr(
            "Audio-Engine läuft nicht",
            "The audio engine is not running",
        )
    })
}

/// First start: the system default devices, one mix.
fn initial_config() -> Config {
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
    }
    .normalized()
}

/// Queries devices off the main thread (see [`get_devices`]).
fn first_start() -> Config {
    std::thread::spawn(initial_config)
        .join()
        .unwrap_or_default()
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn main() {
    tauri::Builder::default()
        // Starting Faderust again (e.g. from the Start menu while it runs in
        // the tray) shows the running window instead of opening the devices
        // a second time.
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            show_main(app)
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .arg(MINIMIZED_ARG)
                .build(),
        )
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            let _ = std::fs::create_dir_all(&dir);
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
            let config = Arc::new(Mutex::new(config));
            let audio = audio::spawn(app.handle().clone(), Arc::clone(&config), path.clone());
            let token = app
                .path()
                .app_config_dir()
                .map_err(|e| e.to_string())
                .and_then(|dir| {
                    remote::load_or_create_token(&dir.join(remote::TOKEN_FILE))
                        .map_err(|e| e.to_string())
                })
                .unwrap_or_else(|e| {
                    log::write(&format!("Fernsteuerung aus: Token nicht angelegt: {e}"));
                    String::new()
                });
            app.manage(AppState {
                config,
                path,
                audio: Mutex::new(audio),
                streamdeck: Mutex::default(),
                token,
            });
            remote::spawn(app.handle().clone());

            let show = MenuItem::with_id(app, "show", "Anzeigen", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Beenden", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let mut tray = TrayIconBuilder::with_id("main")
                .tooltip("Faderust")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;

            // The window starts hidden (tauri.conf.json) so autostart does
            // not flash it.
            if !std::env::args().any(|a| a == MINIMIZED_ARG) {
                show_main(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window keeps the mixer running in the tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_devices,
            get_apps,
            set_app_output,
            set_app_volume,
            get_app_levels,
            get_windows_audio,
            set_default_device,
            set_device_volume,
            get_streamdeck,
            install_streamdeck,
            set_language,
            get_autostart,
            set_autostart,
            open_cable_page,
            get_config,
            save_config,
            set_gain,
            set_muted,
            set_master_gain,
            set_master_muted,
            set_input_muted,
            set_fx,
            save_preset,
            load_preset,
            delete_preset
        ])
        .run(tauri::generate_context!())
        .expect("error while running Faderust");
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
