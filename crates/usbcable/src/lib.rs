//! Experimental virtual audio cable for Windows without an own driver: a USB
//! Audio Class 1.0 device that is served over USB/IP. Windows binds its
//! built-in USB audio driver to it, so it shows up as a normal sound card
//! (playback and recording) with no kernel code of ours.

pub mod device;
pub mod ring;
pub mod server;
pub mod wire;
