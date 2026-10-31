//! Names of Faderust's own virtual cables ("Speakers (Faderust Cable 2)") and
//! how to pair the two ends of a cable. Mirrors `ui/src/lib/api.js`.

/// Which Faderust USB cable a device name belongs to ("faderust cable 2"), if any.
pub fn own_cable(name: &str) -> Option<String> {
    let n = name.to_lowercase();
    let start = n.rfind("(faderust cable")?;
    let end = n[start..].find(')')? + start;
    Some(n[start + 1..end].to_string())
}

/// One of Faderust's own virtual cables, by name.
pub fn is_cable(name: &str) -> bool {
    own_cable(name).is_some()
}

/// The recording end of the cable that `output` plays into, searched in
/// `recordings` (empty when there is none).
pub fn cable_counterpart(output: &str, recordings: &[String]) -> String {
    if let Some(own) = own_cable(output) {
        return recordings
            .iter()
            .find(|d| own_cable(d).as_deref() == Some(own.as_str()))
            .cloned()
            .unwrap_or_default();
    }
    match output.to_lowercase().find("input") {
        Some(i) => {
            let word = if output[i..].starts_with('I') {
                "Output"
            } else {
                "output"
            };
            format!("{}{}{}", &output[..i], word, &output[i + 5..])
        }
        None => output.to_string(),
    }
}

/// "Speakers (Faderust Cable 2)" reads as "Faderust Kabel 2 (Wiedergabe)":
/// the cable's name first, then which end it is. Other devices keep their name.
pub fn device_label(name: &str) -> String {
    let Some(own) = own_cable(name) else {
        return name.to_string();
    };
    let digits: String = own
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let nr = if digits.is_empty() || digits == "1" {
        String::new()
    } else {
        format!(" {digits}")
    };
    let head = name[..name.to_lowercase().rfind("(faderust cable").unwrap_or(0)]
        .trim()
        .to_lowercase();
    let starts = |words: &[&str]| words.iter().any(|w| head.starts_with(w));
    if starts(&[
        "speakers",
        "lautsprecher",
        "headphones",
        "kopfhörer",
        "line",
        "digital",
        "playback",
    ]) {
        format!("Faderust Kabel{nr} (Wiedergabe)")
    } else if starts(&["microphone", "mikrofon", "recording"]) {
        format!("Faderust Kabel{nr} (Aufnahme)")
    } else {
        format!("Faderust Kabel{nr}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cable_names_pair_up() {
        assert_eq!(
            own_cable("Speakers (Faderust Cable 2)").as_deref(),
            Some("faderust cable 2")
        );
        assert!(!is_cable("Realtek Audio"));
        let rec = vec!["Microphone (Faderust Cable 2)".to_string()];
        assert_eq!(
            cable_counterpart("Lautsprecher (Faderust Cable 2)", &rec),
            rec[0]
        );
        assert_eq!(
            cable_counterpart("CABLE Input (VB-Audio)", &[]),
            "CABLE Output (VB-Audio)"
        );
    }

    #[test]
    fn labels_name_the_cable_and_its_end() {
        assert_eq!(
            device_label("Speakers (Faderust Cable)"),
            "Faderust Kabel (Wiedergabe)"
        );
        assert_eq!(
            device_label("Microphone (Faderust Cable 3)"),
            "Faderust Kabel 3 (Aufnahme)"
        );
        assert_eq!(device_label("Realtek"), "Realtek");
    }
}
