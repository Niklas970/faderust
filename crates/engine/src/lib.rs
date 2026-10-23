//! Faderust audio engine.
//!
//! Will contain the WASAPI backend (behind a trait, so a PipeWire backend stays
//! possible), lock-free ring buffers, drift compensation and the mix matrix.

/// Crate version, exposed for the app's about screen and logs.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!VERSION.is_empty());
    }
}
