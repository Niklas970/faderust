<p align="center"><img src="assets/logo.svg" width="112" alt="Faderust logo"></p>

<h1 align="center">Faderust</h1>

<p align="center">
  Audio-Mixer für Windows, als schlanker, offener Ersatz für Voicemeeter.<br>
  Jeder Ausgang bekommt seine eigene Mischung, Geräte dürfen im Betrieb kommen und gehen, virtuelle Kabel bringt Faderust selbst mit.
</p>

<p align="center">
  <a href="https://github.com/Niklas970/faderust/releases/latest"><img src="https://img.shields.io/github/v/release/Niklas970/faderust?label=Download" alt="Neueste Version"></a>
  <a href="https://github.com/Niklas970/faderust/actions/workflows/ci.yml"><img src="https://github.com/Niklas970/faderust/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/Niklas970/faderust" alt="Lizenz MIT"></a>
</p>

![Faderust im Pult mit Geräten, Programmen und drei Ausgängen](assets/screenshot.png)

<p align="center">
  <a href="assets/faderust-reel.mp4"><img src="assets/faderust-reel.gif" width="270" alt="Faderust in 40 Sekunden"></a><br>
  <sub>Vorschau, <a href="assets/faderust-reel.mp4">Video in voller Qualität</a> (zum Herunterladen)</sub>
</p>

> **Alpha.** Faderust läuft, ist aber noch kaum auf echter Hardware erprobt. Fehler bitte als [Issue](https://github.com/Niklas970/faderust/issues) melden.

## Was Faderust kann

- **Pult wie bei Voicemeeter.** Jedes Gerät und jedes Programm bekommt einen Streifen mit Fader, MUTE und den Knöpfen A1, A2, A3 für die Ausgänge. Jeder Ausgang hat einen eigenen Master-Fader.
- **Matrix für Feinarbeit.** Alternativ als Tabelle: jeder Eingang auf jedem Ausgang einzeln geregelt, z. B. das Mikro leiser auf dem Kopfhörer als im Stream.
- **Programme direkt aufnehmen.** Discord, Spotify oder der Browser bekommen einen eigenen Streifen mit Effekten, ganz ohne virtuelles Kabel pro Programm. Stille Programme blendet Faderust aus und holt sie auf Wunsch einzeln zurück („N still, zeigen“).
- **Eigene virtuelle Kabel, kein Zusatz-Treiber.** Das „Faderust Kabel“ (bis zu vier) richtet die App selbst ein, zum Beispiel eines für das Mikrofon und eines für Programme. Details unter [Faderust Kabel](#faderust-kabel).
- **Aufgeräumt.** Ausgänge lassen sich mit dem × ausblenden und kommen von selbst zurück, sobald wieder Ton auf ihnen spielt.
- **Geräte kommen und gehen.** Bluetooth-Kopfhörer aus- und wieder einschalten, USB-Gerät abziehen: Faderust verbindet neu, ohne die anderen Geräte zu unterbrechen.
- **Multipoint-freundlich.** Bluetooth-Ausgänge gibt Faderust bei Stille frei (nach 30 s, pro Ausgang einstellbar), damit die Kopfhörer zum Handy wechseln können. Kommt wieder Ton, öffnen sie sich von selbst.
- **Effekte pro Eingang:** Rauschunterdrückung ([RNNoise](https://jmvalin.ca/demo/rnnoise/), gegen Tastatur und Lüfter), Low-Cut, Drei-Band-EQ und Gate. Die Rauschunterdrückung läuft bei Geräten mit 48 kHz und verzögert den Ton um 10 ms.
- **Deutsch und Englisch.** Die Sprache folgt Windows und lässt sich unter „Design“ umstellen.
- **Einstellungen speichern sich selbst.** Der Geräte-Dialog übernimmt Änderungen kurz nach der letzten Eingabe.
- **Selbsttest.** Unter „Geräte“ prüft Faderust Geräte, Puffer, Rückkopplung, Fernsteuerung und Stream-Deck-Plugin und zeigt, was in Ordnung ist und was nicht. Der Bericht lässt sich für ein Issue kopieren.
- **Windows-Sound aus einer Hand:** Standardgeräte für Wiedergabe, Aufnahme und Anrufe sowie die Lautstärke pro Gerät.
- **Presets, Autostart im Tray, Updates in der App** (mit Signaturprüfung) und ein **Stream-Deck-Plugin**.
- **Kompakt und im Vordergrund.** Zwei Knöpfe in der Titelleiste: schmale Ansicht ohne Beschriftungen und „Immer im Vordergrund“, z. B. neben einem Spiel. Beides bleibt gespeichert.
- **Eigene Farben:** Akzent- und Signalfarbe frei wählbar, dunkel (OLED-Schwarz) oder hell.

## Ansichten

<table>
  <tr>
    <td width="68%"><img src="assets/matrix.png" alt="Matrix: jeder Eingang auf jedem Ausgang einzeln geregelt"></td>
    <td><img src="assets/design.png" alt="Design-Dialog mit Ansicht, Farben und Darstellung"></td>
  </tr>
  <tr>
    <td><b>Matrix:</b> jeder Eingang auf jedem Ausgang einzeln, z. B. das Mikro leiser auf dem Kopfhörer als im Stream.</td>
    <td><b>Design:</b> Pult oder Matrix, Seite der Ausgänge, Farben, dunkel oder hell.</td>
  </tr>
</table>

## Installation

1. Unter [Releases](https://github.com/Niklas970/faderust/releases/latest) die Datei `Faderust_*_x64-setup.exe` laden und starten. Das Setup braucht keine Admin-Rechte und spricht Deutsch oder Englisch.
2. Das Setup ist nicht signiert, deshalb warnt Windows SmartScreen: „Weitere Informationen“ → „Trotzdem ausführen“.
3. Faderust aus dem Startmenü öffnen. Neue Versionen meldet die App oben in der Titelleiste, ein Klick auf die Versionsnummer sucht sofort danach.

Voraussetzung: Windows 10 (ab Version 2004) oder Windows 11.

## Erste Schritte

1. **Geräte** öffnen und Eingänge (Mikrofon, Line-In, Faderust Kabel) und Ausgänge (Kopfhörer, Lautsprecher, Stream) anlegen. Bluetooth braucht etwa 80 ms Puffer, USB meist 30 bis 60 ms.
2. Im **Pult** mit A1–A3 wählen, wohin ein Eingang spielt. Der Fader regelt alle eingeschalteten Ausgänge gemeinsam.
3. Fader bedienen: ziehen, Mausrad (mit Shift feiner), Doppelklick auf 0 dB, oder auf die dB-Zahl darunter klicken und einen Wert eintippen.
4. **Effekte** unter einem Streifen öffnet Low-Cut, EQ und Gate.
5. Unter **Design** zwischen Pult und Matrix wechseln, die Seite der Ausgänge und die Farben wählen.

### Programme

**+ Programm** im Pult zeigt alle Programme, die gerade Ton abspielen. **+ Eingang** nimmt eines direkt auf. Damit du es nicht doppelt hörst, stellt Faderust das Programm zuerst auf ein freies Faderust Kabel, das du nicht hörst (falls eingerichtet), und nimmt es dann auf. Ein Programm, das gerade still ist, blendet Faderust aus; **N still, zeigen** holt es zurück.

### Mikrofon für Discord, Teams & Co.

Unter **Geräte** legt **+ Programm-Mikro** einen Ausgang auf einem freien Faderust Kabel an, auf dem nur das Mikrofon mit seinen Effekten liegt. Im Programm wählst du die Aufnahme-Seite desselben Kabels als Mikrofon. Rückkopplungen (ein Ausgang spielt auf das Kabel, von dem ein Eingang kommt) schaltet Faderust stumm und zeigt „Schleife“.

### Faderust Kabel

Das Faderust Kabel ist ein virtuelles USB-Soundgerät (Stereo, 48 kHz, 24 Bit), das Faderust selbst bereitstellt. Faderust zeigt es als „Faderust Kabel (Wiedergabe)“ und „Faderust Kabel (Aufnahme)“; was ein Programm auf die Wiedergabe-Seite spielt, kommt auf der Aufnahme-Seite wieder heraus.

1. Unter **Geräte** → **Faderust Kabel** auf **Einrichten** klicken (einmalig, mit einer Administrator-Abfrage). Faderust lädt dabei [usbip-win2](https://github.com/vadimgrn/usbip-win2) herunter, prüft die Prüfsumme und installiert es im Hintergrund. In der Windows-App-Liste erscheint es nicht extra.
2. **Faderust Kabel** einschalten und die Anzahl wählen (1 bis 4), zum Beispiel eines für das Mikrofon und eines für Programme. Windows zeigt sie in seinen Einstellungen als „Faderust Cable“, „Faderust Cable 2“ und so weiter.
3. **Entfernen** nimmt die Einrichtung samt usbip-win2 wieder zurück.

Hinweis: Das Kabel ist noch ein Test und auf wenig Hardware erprobt.

## Grenzen

- Programme merken sich Fader, Ausgänge und Effekte, solange ihr Streifen im Pult bleibt: Startet Discord neu, kommt es mit seinen alten Einstellungen zurück. Entfernst du den Streifen unter „Geräte“, sind sie weg.
- Kein eigener Kernel-Treiber. Die virtuellen Kabel („Faderust Kabel“, bis zu vier) sind USB-Soundgeräte, die Faderust selbst per USB/IP bereitstellt. Windows bindet dafür seinen eingebauten USB-Audiotreiber; der USB/IP-Teil ist [usbip-win2](https://github.com/vadimgrn/usbip-win2) (BSD-2, von Microsoft signiert), das die Einrichtung einmalig installiert.
- Windows liefert den Ton pro Programm, nicht pro Browser-Tab oder Web-App. Alles aus einem Browser läuft über denselben Audio-Prozess.
- Bluetooth braucht nach einer Freigabe etwa eine Sekunde, bis wieder Ton kommt. Wer das nicht will, stellt den Ausgang auf „Nie frei“.
- Die Zuordnung „Programm → Gerät“ nutzt dieselbe nicht dokumentierte Windows-Schnittstelle wie der Lautstärkemixer in den Einstellungen.

Einstellungen: `%APPDATA%\io.github.niklas970.faderust\config.toml`, daneben das Protokoll `faderust.log`, das bei Fehlern hilft.

## Entwicklung

Rust für den Audio-Kern, Svelte 5 mit Tauri 2 für die Oberfläche.

| Pfad | Inhalt |
|---|---|
| `crates/engine` | Audio-Kern: Geräte (WASAPI über cpal), Ringpuffer, Drift-Ausgleich, Mix-Matrix, Effekte, Programm-Aufnahme |
| `crates/usbcable` | Faderust Kabel: virtuelles USB-Audiogerät über USB/IP |
| `crates/app` | Tauri-App: Befehle, Konfiguration, Tray, Updater |
| `crates/app/ui` | Oberfläche |
| `streamdeck` | Stream-Deck-Plugin |

Voraussetzungen: Rust (stable), Node.js 22, unter Windows WebView2.

```sh
cargo test
cd crates/app/ui && npm ci && npm run build && cd ../../..
cargo run --release -p faderust
```

- `npm run dev` in `crates/app/ui` zeigt die Oberfläche im Browser mit simulierten Geräten.
- Setup bauen: in `crates/app` `ui/node_modules/.bin/tauri build`, Ergebnis unter `target/release/bundle/nsis/`. Releases entstehen über den Workflow „Release“.
- Engine ohne Oberfläche: `cargo run --release -p faderust-engine --example mixer -- --list`

Vor einem Commit: `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`. Neue Abhängigkeiten brauchen eine freizügige Lizenz (`cargo deny check`).

## Stream Deck

Das Plugin in [`streamdeck/`](streamdeck/README.md) steuert Faderust vom Elgato Stream Deck: Fader per Drehregler oder Taste, Stummschalten, Presets und Windows-Standardgerät. `com.niklas970.faderust.streamDeckPlugin` aus den [Releases](https://github.com/Niklas970/faderust/releases) doppelklicken (Stream Deck 7.1 oder neuer). Ab da liefert das Setup das Plugin mit: unter „Geräte“ zeigt Faderust, ob eine neuere Plugin-Version bereitliegt, und öffnet sie mit einem Klick in der Stream-Deck-Software, die dann nur noch um Bestätigung bittet.

## Lizenz

[MIT](LICENSE)
