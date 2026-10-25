//! The self-test in the settings: looks at devices, buffers, remote control
//! and the Stream Deck plugin and lists what is fine and what is not, in
//! words a user can pass on in an issue.

use faderust_engine::lang::tr;
use serde::Serialize;

use crate::remote::Link;

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Ok,
    /// Works, but worth a look.
    Warn,
    Fail,
    /// Nothing to judge, e.g. no Stream Deck in use.
    Info,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub status: Status,
    pub title: String,
    pub detail: String,
}

fn check(status: Status, title: impl Into<String>, detail: impl Into<String>) -> Check {
    Check {
        status,
        title: title.into(),
        detail: detail.into(),
    }
}

/// What state the config file on disk is in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigFile {
    Missing,
    Fine,
    Broken(String),
}

/// One input or output as the self-test sees it.
#[derive(Debug, Clone)]
pub struct Slot {
    pub label: String,
    /// Device name pattern from the config, "app:…" for a program.
    pub device: String,
    pub connected: bool,
    /// Outputs: closed on purpose while silent.
    pub released: bool,
    /// Whether a device matching `device` is plugged in; `None` when the
    /// device list could not be read.
    pub present: Option<bool>,
    /// Outputs: buffer in milliseconds.
    pub buffer_ms: Option<u32>,
}

/// Counters of the lane from one input to one output.
#[derive(Debug, Clone, Default)]
pub struct LaneFacts {
    pub underruns: u64,
    pub dropped_frames: u64,
    pub looped: bool,
}

/// Everything the checks need, gathered by the caller.
#[derive(Debug, Clone)]
pub struct Facts {
    pub config: ConfigFile,
    pub inputs: Vec<Slot>,
    pub outputs: Vec<Slot>,
    /// `lanes[input][output]`; empty while the engine does not run.
    pub lanes: Vec<Vec<LaneFacts>>,
    /// The remote control token exists.
    pub token: bool,
    pub link: Link,
    /// Version of the plugin that ships with this Faderust, if any.
    pub bundled_plugin: Option<String>,
}

fn is_program(device: &str) -> bool {
    device.starts_with("app:")
}

fn slot_check(kind_input: bool, slot: &Slot) -> Check {
    let title = format!(
        "{} „{}“",
        if kind_input {
            tr("Eingang", "Input")
        } else {
            tr("Ausgang", "Output")
        },
        slot.label
    );
    if slot.connected {
        return check(Status::Ok, title, tr("Verbunden", "Connected"));
    }
    if slot.released {
        return check(
            Status::Ok,
            title,
            tr(
                "Bei Stille freigegeben, öffnet sich mit Ton von selbst",
                "Released while silent, opens by itself when sound returns",
            ),
        );
    }
    if is_program(&slot.device) {
        return check(
            Status::Warn,
            title,
            tr(
                "Das Programm spielt gerade keinen Ton ab oder läuft nicht",
                "The program is not playing sound or is not running",
            ),
        );
    }
    match slot.present {
        Some(false) => check(
            Status::Warn,
            title,
            tr(
                "Gerät nicht angeschlossen oder ausgeschaltet. Faderust verbindet von selbst, sobald es da ist",
                "Device not plugged in or switched off. Faderust connects by itself as soon as it is there",
            ),
        ),
        Some(true) => check(
            Status::Fail,
            title,
            tr(
                "Gerät ist da, lässt sich aber nicht öffnen. Meist hält ein anderes Programm es belegt (Discord im Freisprechmodus, DAW, ASIO)",
                "The device is there but cannot be opened. Usually another program holds it (Discord in hands-free mode, a DAW, ASIO)",
            ),
        ),
        None => check(
            Status::Warn,
            title,
            tr(
                "Nicht verbunden; die Geräteliste war nicht lesbar",
                "Not connected; the device list could not be read",
            ),
        ),
    }
}

/// Runs all checks on `facts`.
pub fn evaluate(facts: &Facts) -> Vec<Check> {
    let mut out = Vec::new();

    out.push(match &facts.config {
        ConfigFile::Fine => check(
            Status::Ok,
            tr("Konfiguration", "Configuration"),
            tr("Datei lesbar", "File readable"),
        ),
        ConfigFile::Missing => check(
            Status::Info,
            tr("Konfiguration", "Configuration"),
            tr(
                "Noch nicht gespeichert (erster Start). Sie entsteht mit der ersten Änderung",
                "Not saved yet (first start). It is created with the first change",
            ),
        ),
        ConfigFile::Broken(e) => check(
            Status::Fail,
            tr("Konfiguration", "Configuration"),
            format!(
                "{}: {e}",
                tr(
                    "Datei nicht lesbar, Faderust nutzt Standardwerte",
                    "File not readable, Faderust uses defaults"
                )
            ),
        ),
    });

    if facts.inputs.is_empty() || facts.outputs.is_empty() {
        out.push(check(
            Status::Fail,
            tr("Geräte", "Devices"),
            tr(
                "Es braucht mindestens einen Eingang und einen Ausgang (unter „Geräte“ anlegen)",
                "At least one input and one output are needed (add them under “Devices”)",
            ),
        ));
    }
    out.extend(facts.inputs.iter().map(|s| slot_check(true, s)));
    out.extend(facts.outputs.iter().map(|s| slot_check(false, s)));

    // Buffers: counters of lanes between connected devices only.
    for (o, output) in facts.outputs.iter().enumerate() {
        let (mut underruns, mut dropped) = (0, 0);
        for row in &facts.lanes {
            if let Some(lane) = row.get(o) {
                underruns += lane.underruns;
                dropped += lane.dropped_frames;
            }
        }
        let title = format!("{} „{}“", tr("Puffer", "Buffer"), output.label);
        let buffer = output.buffer_ms.unwrap_or(0);
        if !output.connected {
            continue;
        }
        if underruns == 0 && dropped == 0 {
            out.push(check(
                Status::Ok,
                title,
                format!("{buffer} ms, {}", tr("keine Aussetzer", "no dropouts")),
            ));
        } else {
            out.push(check(
                Status::Warn,
                title,
                tr(
                    "{buffer} ms: {u} Aussetzer, {d} verworfene Frames. Den Puffer unter „Geräte“ erhöhen (Bluetooth braucht oft 80 ms)",
                    "{buffer} ms: {u} dropouts, {d} dropped frames. Raise the buffer under “Devices” (Bluetooth often needs 80 ms)",
                )
                .replace("{buffer}", &buffer.to_string())
                .replace("{u}", &underruns.to_string())
                .replace("{d}", &dropped.to_string()),
            ));
        }
    }

    let loops: Vec<String> = facts
        .lanes
        .iter()
        .enumerate()
        .flat_map(|(i, row)| {
            row.iter()
                .enumerate()
                .filter_map(move |(o, l)| l.looped.then_some((i, o)))
        })
        .filter_map(|(i, o)| {
            Some(format!(
                "{} → {}",
                facts.inputs.get(i)?.label,
                facts.outputs.get(o)?.label
            ))
        })
        .collect();
    if !loops.is_empty() {
        out.push(check(
            Status::Fail,
            tr("Rückkopplung", "Feedback"),
            format!(
                "{}: {}",
                tr(
                    "Diese Wege sind stumm, weil der Ausgang in den Eingang zurückspielt",
                    "These paths are silent because the output plays back into the input"
                ),
                loops.join(", ")
            ),
        ));
    }

    out.push(if facts.token {
        check(
            Status::Ok,
            tr("Fernsteuerung", "Remote control"),
            tr("Token vorhanden", "Token present"),
        )
    } else {
        check(
            Status::Fail,
            tr("Fernsteuerung", "Remote control"),
            tr(
                "Kein Token: Die Konfigurationsmappe ist nicht beschreibbar, das Stream Deck kann sich nicht verbinden",
                "No token: the configuration folder is not writable, the Stream Deck cannot connect",
            ),
        )
    });

    out.push(plugin_check(&facts.link, facts.bundled_plugin.as_deref()));
    out
}

fn plugin_check(link: &Link, bundled: Option<&str>) -> Check {
    let title = "Stream Deck";
    if link.clients > 0 {
        let installed = link.version.as_deref().unwrap_or("?");
        if let Some(bundled) = bundled {
            if crate::remote::is_newer(bundled, installed) {
                return check(
                    Status::Warn,
                    title,
                    tr(
                        "Plugin {i} ist verbunden, {b} liegt bereit (unter „Geräte“ aktualisieren)",
                        "Plugin {i} is connected, {b} is available (update it under “Devices”)",
                    )
                    .replace("{i}", installed)
                    .replace("{b}", bundled),
                );
            }
        }
        return check(
            Status::Ok,
            title,
            tr("Plugin {i} ist verbunden", "Plugin {i} is connected").replace("{i}", installed),
        );
    }
    if link.refused {
        return check(
            Status::Fail,
            title,
            tr(
                "Ein Plugin wurde abgewiesen (zu alt oder falsches Token). Unter „Geräte“ aktualisieren",
                "A plugin was turned away (too old or wrong token). Update it under “Devices”",
            ),
        );
    }
    check(
        Status::Info,
        title,
        tr(
            "Kein Plugin verbunden (in Ordnung, wenn du kein Stream Deck nutzt)",
            "No plugin connected (fine if you do not use a Stream Deck)",
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(label: &str, device: &str, connected: bool) -> Slot {
        Slot {
            label: label.into(),
            device: device.into(),
            connected,
            released: false,
            present: Some(true),
            buffer_ms: None,
        }
    }

    fn facts() -> Facts {
        let mut out = slot("PS5", "PS5", true);
        out.buffer_ms = Some(40);
        Facts {
            config: ConfigFile::Fine,
            inputs: vec![slot("Mikro", "USB Mic", true)],
            outputs: vec![out],
            lanes: vec![vec![LaneFacts::default()]],
            token: true,
            link: Link::default(),
            bundled_plugin: Some("0.3.1.0".into()),
        }
    }

    fn find<'a>(checks: &'a [Check], needle: &str) -> &'a Check {
        checks
            .iter()
            .find(|c| c.title.contains(needle))
            .unwrap_or_else(|| panic!("no check {needle:?} in {checks:?}"))
    }

    #[test]
    fn a_healthy_setup_has_no_warnings() {
        let checks = evaluate(&facts());
        assert!(
            checks
                .iter()
                .all(|c| matches!(c.status, Status::Ok | Status::Info)),
            "{checks:?}"
        );
        assert_eq!(find(&checks, "Stream Deck").status, Status::Info);
    }

    #[test]
    fn an_unplugged_device_is_a_warning_a_busy_one_a_failure() {
        let mut f = facts();
        f.inputs[0].connected = false;
        f.inputs[0].present = Some(false);
        f.outputs[0].connected = false;
        f.outputs[0].present = Some(true);
        let checks = evaluate(&f);
        assert_eq!(find(&checks, "Mikro").status, Status::Warn);
        assert_eq!(find(&checks, "PS5").status, Status::Fail);
        assert!(find(&checks, "PS5").detail.len() > 20);
    }

    #[test]
    fn programs_and_released_outputs_are_not_failures() {
        let mut f = facts();
        f.inputs.push(slot("Discord", "app:discord.exe", false));
        f.outputs[0].connected = false;
        f.outputs[0].released = true;
        let checks = evaluate(&f);
        assert_eq!(find(&checks, "Discord").status, Status::Warn);
        assert_eq!(find(&checks, "PS5").status, Status::Ok);
        // A released output has no buffer verdict.
        assert!(checks.iter().all(|c| !c.title.starts_with("Puffer")));
    }

    #[test]
    fn dropouts_point_to_the_buffer() {
        let mut f = facts();
        f.lanes[0][0] = LaneFacts {
            underruns: 3,
            dropped_frames: 960,
            looped: false,
        };
        let checks = evaluate(&f);
        let buffer = find(&checks, "Puffer");
        assert_eq!(buffer.status, Status::Warn);
        assert!(buffer.detail.contains("40 ms"), "{}", buffer.detail);
        assert!(buffer.detail.contains('3') && buffer.detail.contains("960"));
    }

    #[test]
    fn feedback_names_the_path() {
        let mut f = facts();
        f.lanes[0][0].looped = true;
        let checks = evaluate(&f);
        let c = find(&checks, "Rückkopplung");
        assert_eq!(c.status, Status::Fail);
        assert!(c.detail.contains("Mikro → PS5"), "{}", c.detail);
    }

    #[test]
    fn broken_config_and_missing_token_fail() {
        let mut f = facts();
        f.config = ConfigFile::Broken("expected a table".into());
        f.token = false;
        let checks = evaluate(&f);
        assert_eq!(find(&checks, "Konfiguration").status, Status::Fail);
        assert!(find(&checks, "Konfiguration")
            .detail
            .contains("expected a table"));
        assert_eq!(find(&checks, "Fernsteuerung").status, Status::Fail);
        f.config = ConfigFile::Missing;
        assert_eq!(find(&evaluate(&f), "Konfiguration").status, Status::Info);
    }

    #[test]
    fn no_devices_configured_is_a_failure() {
        let mut f = facts();
        f.inputs.clear();
        assert_eq!(find(&evaluate(&f), "Geräte").status, Status::Fail);
    }

    #[test]
    fn plugin_states() {
        let link = |clients, version: Option<&str>, refused| Link {
            clients,
            version: version.map(Into::into),
            refused,
        };
        let status = |link, bundled: Option<&str>| plugin_check(&link, bundled).status;
        assert_eq!(
            status(link(1, Some("0.3.1.0"), false), Some("0.3.1.0")),
            Status::Ok
        );
        assert_eq!(
            status(link(1, Some("0.3.0.0"), false), Some("0.3.1.0")),
            Status::Warn
        );
        assert_eq!(status(link(1, Some("0.3.0.0"), false), None), Status::Ok);
        assert_eq!(status(link(0, None, true), Some("0.3.1.0")), Status::Fail);
        assert_eq!(status(link(0, None, false), Some("0.3.1.0")), Status::Info);
    }
}
