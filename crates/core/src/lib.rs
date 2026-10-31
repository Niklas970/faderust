//! Faderust without a window: the configuration, the audio thread that
//! drives the engine, the Stream Deck remote, the USB cable and the commands
//! a front end offers. The front end (the app crate) implements [`Host`].

pub mod audio;
pub mod cable;
pub mod config;
pub mod log;
pub mod remote;
pub mod selftest;
pub mod state;

pub use state::*;

/// What the audio thread needs from the window around it.
pub trait Host: Send + Sync + 'static {
    /// A fresh snapshot of the mixer, about 20 times a second while visible.
    fn state(&self, state: audio::UiState);
    /// A message for the user.
    fn notice(&self, notice: audio::Notice);
    /// Whether the window is on screen; hidden windows get no snapshots.
    fn visible(&self) -> bool;
}
