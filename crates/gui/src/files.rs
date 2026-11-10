//! Native open/save dialogs (Windows). Elsewhere there are none and the
//! calls answer that they are not available.

use std::path::PathBuf;

/// Where to save a preset; `None` if the user cancelled.
#[cfg(windows)]
pub fn save_path(name: &str) -> Result<Option<PathBuf>, String> {
    Ok(rfd::FileDialog::new()
        .set_title("Preset exportieren")
        .set_file_name(format!("{name}.json"))
        .add_filter("Faderust-Preset", &["json"])
        .save_file())
}

/// The preset file to import; `None` if the user cancelled.
#[cfg(windows)]
pub fn open_path() -> Result<Option<PathBuf>, String> {
    Ok(rfd::FileDialog::new()
        .set_title("Preset importieren")
        .add_filter("Faderust-Preset", &["json"])
        .pick_file())
}

#[cfg(not(windows))]
pub fn save_path(_: &str) -> Result<Option<PathBuf>, String> {
    Err("Dateiauswahl gibt es nur unter Windows".into())
}

#[cfg(not(windows))]
pub fn open_path() -> Result<Option<PathBuf>, String> {
    Err("Dateiauswahl gibt es nur unter Windows".into())
}

/// WAV and MP3 files for the soundboard; empty if the user cancelled.
#[cfg(windows)]
pub fn sound_paths() -> Result<Vec<PathBuf>, String> {
    Ok(rfd::FileDialog::new()
        .set_title("Sounds hinzufügen")
        .add_filter("WAV, MP3", &["wav", "mp3"])
        .pick_files()
        .unwrap_or_default())
}

#[cfg(not(windows))]
pub fn sound_paths() -> Result<Vec<PathBuf>, String> {
    Err("Dateiauswahl gibt es nur unter Windows".into())
}
