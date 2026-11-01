//! The language of messages the engine and the app show to the user. German
//! unless the interface asked for English; log lines stay as they are.

use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

static ENGLISH: AtomicBool = AtomicBool::new(false);

/// Sets the language from a code such as "en" or "de-DE".
pub fn set(code: &str) {
    ENGLISH.store(code.to_lowercase().starts_with("en"), Relaxed);
}

/// Whether the interface is in English.
pub fn is_english() -> bool {
    ENGLISH.load(Relaxed)
}

/// `german` or `english`, whichever the interface uses.
pub fn tr(german: &str, english: &str) -> String {
    pick(ENGLISH.load(Relaxed), german, english).to_string()
}

fn pick<'a>(english_wanted: bool, german: &'a str, english: &'a str) -> &'a str {
    if english_wanted {
        english
    } else {
        german
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_wanted_language() {
        assert_eq!(pick(false, "Hallo", "Hello"), "Hallo");
        assert_eq!(pick(true, "Hallo", "Hello"), "Hello");
    }

    #[test]
    fn language_codes() {
        // Not through the global switch, which other tests share.
        let english = |code: &str| code.to_lowercase().starts_with("en");
        assert!(english("en"));
        assert!(english("en-GB"));
        assert!(!english("de"));
        assert!(!english("de-DE"));
    }
}
