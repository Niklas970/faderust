//! English for the window's German texts. A text with no entry stays as it is,
//! so numbers, device names and the like pass through `text` untouched.

use iced::widget::Text;

const TABLE: &[(&str, &str)] = &[
    ("  Anrufe", "  Calls"),
    ("  Standard", "  Default"),
    ("(alt)", "(old)"),
    ("+ Ausgang", "+ Output"),
    ("+ Eingang", "+ Input"),
    ("+ Programm", "+ Program"),
    ("AUS", "OFF"),
    ("AUSGÄNGE", "OUTPUTS"),
    ("Aktualisieren", "Refresh"),
    ("Aktuelle Einstellung speichern", "Save current setting"),
    ("Akzentfarbe", "Accent colour"),
    ("Alles in Ordnung", "Everything is fine"),
    ("Andere", "Others"),
    ("Ansicht", "View"),
    ("Anzahl Kabel", "Number of cables"),
    ("Aufnahme", "Recording"),
    ("Aus", "Off"),
    (
        "Ausgabegeräte pro Programm gibt es nur unter Windows.",
        "Output devices per program exist only on Windows.",
    ),
    ("Ausgänge", "Outputs"),
    ("Bass", "Bass"),
    ("Bericht kopieren", "Copy report"),
    ("Bericht kopiert", "Report copied"),
    (
        "Das Faderust Kabel gibt es nur unter Windows.",
        "The Faderust cable exists only on Windows.",
    ),
    (
        "Das Plugin ist zu alt und darf sich nicht mehr verbinden. Bitte aktualisieren:",
        "The plugin is too old and may no longer connect. Please update:",
    ),
    (
        "Der Balken zeigt den Pegel, der Strich die Schwelle.",
        "The bar shows the level, the line the threshold.",
    ),
    ("Dunkel", "Dark"),
    (
        "Die Windows-Soundeinstellungen gibt es nur unter Windows.",
        "The Windows sound settings exist only on Windows.",
    ),
    (
        "Dieses Faderust enthält kein Stream-Deck-Plugin.",
        "This Faderust has no Stream Deck plugin.",
    ),
    ("Effekte", "Effects"),
    (
        "Ein Preset merkt sich Fader, Stumm und Effekte aller Eingänge.",
        "A preset remembers faders, mute and effects of all inputs.",
    ),
    ("Eingang", "Input"),
    ("Eingänge", "Inputs"),
    ("Entfernen", "Remove"),
    ("Faderust Kabel", "Faderust cable"),
    ("Faderust Kabel einrichten", "Set up Faderust cable"),
    ("Faderust Kabel verwenden", "Use Faderust cable"),
    ("Faderust beendet", "Faderust closed"),
    ("Faderust ist aktuell", "Faderust is up to date"),
    (
        "Faderust nimmt Geräte direkt ab. Der Windows-Standard bestimmt nur, wohin Programme ohne eigene Zuordnung spielen und welches Mikrofon sie nutzen.",
        "Faderust takes devices directly. The Windows default only decides where programs without their own assignment play and which microphone they use.",
    ),
    ("Fertig", "Done"),
    (
        "Filtert Tastatur, Lüfter und Rauschen aus der Stimme.",
        "Filters keyboard, fan and noise out of the voice.",
    ),
    ("Frei nach 2 min", "Free after 2 min"),
    ("Frei nach 30 s", "Free after 30 s"),
    ("Frei nach 5 s", "Free after 5 s"),
    ("Freigegeben", "Released"),
    ("Für Anrufe", "For calls"),
    ("GERÄTE", "DEVICES"),
    (
        "Gerade spielt kein Programm Ton ab. Starte die Wiedergabe und tippe auf „Aktualisieren“.",
        "No program is playing sound right now. Start playback and click “Refresh”.",
    ),
    ("Gerät nicht verbunden", "Device not connected"),
    ("Geräte", "Devices"),
    ("Gespeicherte Presets", "Saved presets"),
    ("Hell", "Light"),
    ("Helligkeit", "Brightness"),
    ("Hintergrundgeräusche entfernen", "Remove background noise"),
    ("Höhen", "Treble"),
    ("Kopieren", "Copy"),
    ("Lade …", "Loading …"),
    ("Laden", "Load"),
    ("Laut", "Loud"),
    ("Links", "Left"),
    ("Lädt …", "Loading …"),
    ("Löschen", "Delete"),
    ("Mit Windows starten (im Tray, ohne Fenster)", "Start with Windows (in the tray, no window)"),
    ("Nach 2 min", "After 2 min"),
    ("Nach 5 s", "After 5 s"),
    ("Nie frei", "Never free"),
    ("Nie", "Never"),
    (
        "Noch kein Faderust Kabel. Mit „+ Eingang“ nimmt Faderust ein Programm direkt auf. Damit es dann nicht zusätzlich über die Boxen läuft, braucht es ein Gerät, das du nicht hörst: das Faderust Kabel (Einrichtung unter Geräte).",
        "No Faderust cable yet. With “+ Input” Faderust records a program directly. So it does not also play through the speakers, it needs a device you do not hear: the Faderust cable (set up under Devices).",
    ),
    ("Noch nichts protokolliert.", "Nothing logged yet."),
    ("Nochmal prüfen", "Check again"),
    ("Ordner öffnen", "Open folder"),
    ("PROGRAMME", "PROGRAMS"),
    ("Plugin aktualisieren", "Update plugin"),
    ("Plugin installieren", "Install plugin"),
    ("Programme", "Programs"),
    ("Protokoll kopiert", "Log copied"),
    ("Protokoll", "Log"),
    ("Präsenz", "Presence"),
    ("Prüfe …", "Checking …"),
    (
        "Prüft Geräte, Puffer, Fernsteuerung und Plugin. Der Bericht lässt sich in ein Issue kopieren.",
        "Checks devices, buffers, remote control and plugin. The report can be copied into an issue.",
    ),
    (
        "Prüft Geräte, Puffer, Fernsteuerung und Plugin.",
        "Checks devices, buffers, remote control and plugin.",
    ),
    ("Pult", "Desk"),
    ("Rauschfilter", "Noise filter"),
    ("Rechts", "Right"),
    ("Richte ein …", "Setting up …"),
    ("Schließen", "Close"),
    (
        "Schneidet Brummen und Trittschall unten ab.",
        "Cuts hum and footstep rumble at the bottom.",
    ),
    ("Schwelle", "Threshold"),
    ("Selbsttest", "Self-test"),
    ("Signalfarbe", "Signal colour"),
    ("Speichern", "Save"),
    ("Standardgerät", "Default device"),
    ("Starten", "Start"),
    ("Stille Programme ausblenden", "Hide silent programs"),
    (
        "Stream Deck nicht verbunden. Plugin einrichten oder neu installieren:",
        "Stream Deck not connected. Set up or reinstall the plugin:",
    ),
    ("Stumm unter der Schwelle", "Mute below the threshold"),
    ("Stumm", "Mute"),
    (
        "Virtuelle Kabel für Programm-Mikro und Programme, die du nicht über die Boxen hören willst. Faderust stellt sie selbst als USB-Soundgerät bereit, ohne zusätzliche Treiber-Apps. Einmalige Einrichtung, dafür sind Administratorrechte nötig.",
        "Virtual cables for the program mic and for programs you don't want on the speakers. Faderust provides them itself as a USB sound device, no extra driver apps. One-time setup that needs administrator rights.",
    ),
    (
        "Was Faderust zuletzt gemeldet hat. Bei einem Fehler bitte den Text kopieren und in ein Issue einfügen.",
        "What Faderust reported last. On an error, please copy the text into an issue.",
    ),
    ("Wie Windows", "Like Windows"),
    ("Wiedergabe", "Playback"),
    (
        "Windows merkt sich die Zuordnung pro Programm. Manche Programme übernehmen sie erst nach einem Neustart.",
        "Windows remembers the assignment per program. Some programs only pick it up after a restart.",
    ),
    ("Windows-Sound", "Windows sound"),
    ("Windows-Standard", "Windows default"),
    ("Zurücksetzen", "Reset"),
    ("verbinde mit Windows …", "connecting to Windows …"),
    ("verbunden", "connected"),
    ("Änderungen werden automatisch gespeichert.", "Changes are saved automatically."),
    ("Ausgang", "Output"),
    ("Sprache", "Language"),
    ("Darstellung", "Rendering"),
    ("Export", "Export"),
    ("Preset importieren …", "Import preset …"),
    ("Preset exportieren", "Export preset"),
    ("Preset importieren", "Import preset"),
    ("Faderust-Preset", "Faderust preset"),
    ("Dateiauswahl gibt es nur unter Windows", "File dialogs are only available on Windows"),
    ("Das Preset braucht einen Namen", "The preset needs a name"),
    ("Farbwähler", "Colour picker"),
    ("Farbwähler schließen", "Close picker"),
    ("„+ Eingang“ nimmt das Programm auf und stellt es vorher auf", "“+ Input” records the program and first moves it to"),
    (", damit du es nicht doppelt hörst.", ", so you do not hear it twice."),
    ("Eigene Farbe", "Custom colour"),
    ("Alle zeigen", "Show all"),
    ("Programm ausblenden", "Hide program"),
    ("Immer anzeigen, auch wenn still", "Always show, even when silent"),
    ("Bleibt auf dem Pult, auch wenn still", "Stays on the desk, even when silent"),
    ("Kompakte Ansicht", "Compact view"),
    ("Normale Ansicht", "Normal view"),
    ("Immer im Vordergrund", "Always on top"),
    ("Nicht mehr im Vordergrund halten", "No longer keep on top"),
    ("wird von keinem Eingang genutzt.", "is used by no input."),
    ("Programm als eigenen Eingang aufnehmen und stellen auf", "Record the program as its own input and move it to"),
    ("Auf", "Move to"),
    (" stellen", ""),
    ("Du hörst", "You hear"),
    (" doppelt: einmal über den Fader, einmal über sein eigenes Gerät.", " twice: once through the fader, once through its own device."),
    ("+ Programm-Mikro", "+ Program microphone"),
    ("Programm-Mikro angelegt.", "Program microphone created."),
    ("Wähle in Discord, Teams usw. als Mikrofon", "In Discord, Teams etc. pick as microphone"),
    ("Die Spalte „Programm-Mikro“ bestimmt, was dort ankommt.", "The column “Program microphone” decides what arrives there."),
    ("Als Windows-Mikrofon für Anrufe setzen", "Set as Windows microphone for calls"),
    ("Ausgang auf einem virtuellen Kabel, den Programme als Mikrofon nutzen", "Output on a virtual cable that programs use as a microphone"),
    ("Für ein Programm-Mikro (Discord, Teams …) braucht es ein freies virtuelles Kabel. Alle Kabel sind schon belegt: erhöhe die Anzahl unter „Faderust Kabel“.", "A program microphone (Discord, Teams …) needs a free virtual cable. All cables are in use: raise the number under “Faderust cable”."),
    ("Für ein Programm-Mikro (Discord, Teams …) braucht es ein virtuelles Kabel.", "A program microphone (Discord, Teams …) needs a virtual cable."),
    ("Update auf", "Update to"),
    ("ausgeblendet", "hidden"),
    ("still, zeigen", "silent, show"),
    ("verfügbar", "available"),
    ("ist aktuell", "is up to date"),
    ("Punkte brauchen einen Blick", "items need a look"),
    ("Name", "Name"),
    ("Programm-Mikro", "Program microphone"),
    ("Mikrofon für Programme", "Microphone for programs"),
    ("Doppelt gehört", "Heard twice"),
    ("Als Eingang anlegen", "Add as input"),
    ("Kupfer", "Copper"),
    ("Bernstein", "Amber"),
    ("Rot", "Red"),
    ("Violett", "Violet"),
    ("Blau", "Blue"),
    ("Grün", "Green"),
    ("Silber", "Silver"),
    ("Türkis", "Teal"),
    ("Gelb", "Yellow"),
    ("Weiß", "White"),
    ("Systemsprache", "System language"),
    ("Deutsch", "German"),
    ("Englisch", "English"),
];

/// The text in the current language.
pub fn tr(de: &str) -> String {
    if !faderust_core::lang::is_english() {
        return de.to_string();
    }
    TABLE
        .iter()
        .find(|(g, _)| *g == de)
        .map(|(_, e)| e.to_string())
        .unwrap_or_else(|| de.to_string())
}

/// Like `iced::widget::text`, but in the current language.
pub fn text<'a>(s: impl Into<String>) -> Text<'a> {
    iced::widget::text(tr(&s.into()))
}

#[cfg(test)]
mod tests {
    use super::TABLE;

    #[test]
    fn table_has_no_duplicates() {
        let mut keys: Vec<_> = TABLE.iter().map(|(g, _)| *g).collect();
        keys.sort();
        let n = keys.len();
        keys.dedup();
        assert_eq!(n, keys.len());
    }
}
