//! Faderust desktop app: Tauri window and tray around the audio engine.

// No console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Arc;

use faderust_core::audio::{Command, Notice, UiState};
use faderust_core::config::{Config, Fx};
use faderust_core::{
    cable, log, selftest, AppLevel, AppState, Apps, Devices, Host, StreamDeckStatus, WindowsAudio,
    PLUGIN_FILE, PLUGIN_VERSION_FILE,
};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_autostart::ManagerExt;

/// Passed by the Windows autostart entry: start in the tray, no window.
const MINIMIZED_ARG: &str = "--minimized";

/// The window as the audio thread sees it.
struct TauriHost(AppHandle);

impl Host for TauriHost {
    fn state(&self, state: UiState) {
        let _ = self.0.emit("state", state);
    }
    fn notice(&self, notice: Notice) {
        let _ = self.0.emit("notice", notice);
    }
    fn visible(&self) -> bool {
        self.0
            .get_webview_window("main")
            .is_none_or(|w| w.is_visible().unwrap_or(true) && !w.is_minimized().unwrap_or(false))
    }
}

type St<'a> = State<'a, Arc<AppState>>;

fn resource(app: &AppHandle, name: &str) -> Option<PathBuf> {
    app.path()
        .resolve(
            format!("resources/{name}"),
            tauri::path::BaseDirectory::Resource,
        )
        .ok()
        .filter(|p| p.is_file())
}

fn bundled_plugin(app: &AppHandle) -> Option<String> {
    resource(app, PLUGIN_VERSION_FILE)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

// Async so it runs on a worker thread: the main thread's COM apartment
// belongs to the webview, WASAPI wants its own.
#[tauri::command]
async fn get_devices() -> Result<Devices, String> {
    faderust_core::get_devices()
}

#[tauri::command]
async fn get_apps() -> Result<Apps, String> {
    faderust_core::get_apps()
}

#[tauri::command]
async fn set_app_output(pids: Vec<u32>, device: Option<String>) -> Result<(), String> {
    faderust_core::set_app_output(pids, device)
}

#[tauri::command]
async fn set_app_volume(
    pids: Vec<u32>,
    volume: Option<f32>,
    muted: Option<bool>,
) -> Result<(), String> {
    faderust_core::set_app_volume(pids, volume, muted)
}

#[tauri::command]
async fn get_app_levels() -> Result<Vec<AppLevel>, String> {
    faderust_core::get_app_levels()
}

#[tauri::command]
async fn get_windows_audio() -> Result<WindowsAudio, String> {
    faderust_core::get_windows_audio()
}

#[tauri::command]
async fn set_default_device(id: String, communications: bool) -> Result<(), String> {
    faderust_core::set_default_device(id, communications)
}

#[tauri::command]
async fn set_device_volume(
    id: String,
    volume: Option<f32>,
    muted: Option<bool>,
) -> Result<(), String> {
    faderust_core::set_device_volume(id, volume, muted)
}

#[tauri::command]
fn get_usb_cable(state: St) -> cable::Status {
    faderust_core::get_usb_cable(&state)
}

#[tauri::command]
fn set_usb_cable(enabled: bool, state: St) -> Result<(), String> {
    faderust_core::set_usb_cable(enabled, &state)
}

#[tauri::command]
fn set_usb_cable_count(count: u8, state: St) -> Result<(), String> {
    faderust_core::set_usb_cable_count(count, &state)
}

#[tauri::command]
async fn setup_usb_cable() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(faderust_core::setup_usb_cable)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn remove_usb_cable(state: St<'_>) -> Result<(), String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || faderust_core::remove_usb_cable(&state))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn get_streamdeck(app: AppHandle, state: St) -> StreamDeckStatus {
    let link = state
        .streamdeck
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    faderust_core::plugin_status(bundled_plugin(&app), link)
}

// Async: lists devices on a worker thread, like `get_devices`.
#[tauri::command]
async fn run_selftest(app: AppHandle, state: St<'_>) -> Result<Vec<selftest::Check>, String> {
    faderust_core::run_selftest(&state, bundled_plugin(&app))
}

#[tauri::command]
fn get_config(state: St) -> Config {
    state
        .config
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

#[tauri::command]
fn save_config(config: Config, state: St) -> Result<(), String> {
    faderust_core::save_config(config, &state)
}

#[tauri::command]
fn set_gain(input: usize, output: usize, db: f32, state: St) -> Result<(), String> {
    faderust_core::send(&state, Command::SetGain(input, output, db))
}

#[tauri::command]
fn set_muted(input: usize, output: usize, muted: bool, state: St) -> Result<(), String> {
    faderust_core::send(&state, Command::SetMuted(input, output, muted))
}

#[tauri::command]
fn set_master_gain(output: usize, db: f32, state: St) -> Result<(), String> {
    faderust_core::send(&state, Command::SetMasterGain(output, db))
}

#[tauri::command]
fn set_master_muted(output: usize, muted: bool, state: St) -> Result<(), String> {
    faderust_core::send(&state, Command::SetMasterMuted(output, muted))
}

#[tauri::command]
fn set_input_muted(input: usize, muted: bool, state: St) -> Result<(), String> {
    faderust_core::send(&state, Command::SetInputMuted(input, muted))
}

/// Changes an input's effects while it plays; saved with the next fader save.
#[tauri::command]
fn set_fx(input: usize, fx: Fx, state: St) -> Result<(), String> {
    faderust_core::send(&state, Command::SetFx(input, fx))
}

#[tauri::command]
fn save_preset(name: String, state: St) -> Result<(), String> {
    faderust_core::save_preset(name, &state)
}

#[tauri::command]
fn load_preset(name: String, state: St) -> Result<(), String> {
    faderust_core::load_preset(name, &state)
}

#[tauri::command]
fn delete_preset(name: String, state: St) -> Result<(), String> {
    faderust_core::delete_preset(name, &state)
}

/// Hands the shipped plugin to the Stream Deck software, which asks the user
/// to install it (a plugin of the same name is replaced).
#[tauri::command]
fn install_streamdeck(app: AppHandle) -> Result<(), String> {
    let dir = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    faderust_core::install_streamdeck(resource(&app, PLUGIN_FILE), &dir)
}

/// The interface language ("de" or "en"), for the messages Rust sends.
#[tauri::command]
fn set_language(code: String) {
    faderust_engine::lang::set(&code);
}

/// The end of `faderust.log` for the log dialog.
#[tauri::command]
fn get_log(lines: Option<usize>) -> String {
    log::tail(lines.unwrap_or(400).min(5000))
}

/// Shows the folder with `faderust.log` in the file manager.
#[tauri::command]
fn open_log_folder() -> Result<(), String> {
    faderust_core::open_log_folder()
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
            let state = faderust_core::start(Arc::new(TauriHost(app.handle().clone())), &dir);
            app.manage(state);

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
            run_selftest,
            get_log,
            open_log_folder,
            get_autostart,
            set_autostart,
            get_usb_cable,
            set_usb_cable,
            set_usb_cable_count,
            setup_usb_cable,
            remove_usb_cable,
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
        .build(tauri::generate_context!())
        .expect("error while building Faderust")
        .run(|_, event| {
            if let tauri::RunEvent::Exit = event {
                log::write("Faderust beendet");
                // A stream that never closes must not keep the process, and
                // with it the audio devices, alive after Quit.
                let _ = std::thread::Builder::new()
                    .name("faderust-exit-guard".into())
                    .spawn(|| {
                        std::thread::sleep(std::time::Duration::from_secs(3));
                        std::process::exit(0);
                    });
            }
        });
}
