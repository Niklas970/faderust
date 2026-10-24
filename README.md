<p align="center"><img src="assets/logo.svg" width="112" alt="Faderust logo"></p>

<h1 align="center">Faderust</h1>

<p align="center">
  Audio-Mixer für Windows, als schlanker Ersatz für Voicemeeter.<br>
  Jeder Ausgang bekommt seine eigene Mischung, Geräte dürfen im Betrieb kommen und gehen.
</p>

<p align="center">
  <a href="https://github.com/Niklas970/faderust/releases/latest"><img src="https://img.shields.io/github/v/release/Niklas970/faderust?label=Download" alt="Neueste Version"></a>
  <a href="https://github.com/Niklas970/faderust/actions/workflows/ci.yml"><img src="https://github.com/Niklas970/faderust/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/Niklas970/faderust" alt="Lizenz MIT"></a>
</p>

![Faderust mit drei Ausgängen und der Programm-Spalte](assets/screenshot.png)

> **Alpha.** Faderust läuft, ist aber noch kaum auf echter Hardware erprobt. Fehler bitte als [Issue](https://github.com/Niklas970/faderust/issues) melden.

## Was Faderust kann

- **Eine Spalte pro Ausgang, ein Fader pro Eingang.** Mikrofon, PC-Ton und Konsole lassen sich für Kopfhörer, Lautsprecher und Stream getrennt mischen.
- **Geräte kommen und gehen.** Bluetooth-Kopfhörer aus- und wieder einschalten, USB-Gerät abziehen: Faderust verbindet neu, ohne die anderen Geräte zu unterbrechen.
- **Effekte pro Eingang:** Low-Cut, Drei-Band-EQ und Gate, zum Beispiel gegen Tastaturgeräusche am Mikro.
- **Programme auf der Hauptseite:** Lautstärke, Stumm und Pegel für jedes Programm, das gerade Ton abspielt.
- **Programme auf eigene Fader:** Mit [VB-Cable](https://vb-audio.com/Cable/) läuft ein Programm durch Faderust und landet in der Mischung.
- **Windows-Sound direkt aus Faderust:** Standardgeräte für Wiedergabe, Aufnahme und Anrufe sowie die Lautstärke pro Gerät.
- **Presets** für Fader-Stellungen, **Autostart** mit Windows (im Tray), **Updates** in der App mit Signaturprüfung.
- **Eigene Farben:** Akzent- und Signalfarbe frei wählbar, hell oder dunkel.

## Installation

1. Unter [Releases](https://github.com/Niklas970/faderust/releases/latest) die Datei `Faderust_*_x64-setup.exe` laden und starten. Das Setup braucht keine Admin-Rechte und spricht Deutsch oder Englisch, je nach Windows-Sprache.
2. Das Setup ist nicht signiert, deshalb warnt Windows SmartScreen. Klicke auf „Weitere Informationen“ und dann auf „Trotzdem ausführen“.
3. Faderust aus dem Startmenü öffnen. Neue Versionen meldet die App selbst oben in der Titelleiste („Update auf …“).

Voraussetzung ist Windows 10 oder 11. WebView2 ist dort meist schon da, sonst lädt das Setup es nach.

## Erste Schritte

1. **Geräte** öffnen und die Eingänge (Mikrofon, Line-In, virtuelles Kabel) und Ausgänge (Kopfhörer, Lautsprecher …) anlegen. Mit den Pfeilen lässt sich die Reihenfolge ändern.
2. Für Bluetooth-Ausgänge den Puffer höher stellen (80 ms oder mehr), für USB reichen meist 30 bis 60 ms. Zählt ein Ausgang immer wieder Aussetzer, den Puffer erhöhen.
3. Fader bedienen: ziehen, auf die Bahn klicken, Mausrad (mit Shift feiner), Doppelklick setzt auf 0 dB. Der Bereich geht von −∞ bis +12 dB.
4. Ein Klick auf einen Eingang oben in der Leiste öffnet seine Effekte.

### Programme durch Faderust schicken

Damit ein Programm (Spotify, Discord, Browser …) in der Mischung landet, spielt es auf ein virtuelles Kabel, und Faderust nimmt die andere Seite des Kabels als Eingang:

1. [VB-Cable](https://vb-audio.com/Cable/) installieren (kostenlos, wird nicht mitgeliefert) und Windows neu starten.
2. In Faderust unter **Programme** bei „CABLE Output … ist noch kein Eingang“ auf **Als Eingang anlegen** klicken.
3. Im selben Dialog das Programm auf **CABLE Input** stellen.

Windows merkt sich das Gerät pro Programm. Für mehrere getrennte Programm-Fader braucht es mehrere Kabel (VB-Cable A+B).

## Grenzen

- Faderust bringt keinen eigenen Treiber mit. Virtuelle Geräte kommen von VB-Cable.
- Windows liefert den Ton pro Programm, nicht pro Browser-Tab. Alle Tabs eines Browsers laufen über denselben Audio-Prozess und lassen sich deshalb nicht einzeln trennen.
- Die Zuordnung „Programm → Gerät“ nutzt dieselbe nicht dokumentierte Windows-Schnittstelle wie der Lautstärkemixer in den Einstellungen. Ein großes Windows-Update könnte sie ändern.

Die Einstellungen liegen in `%APPDATA%\io.github.niklas970.faderust\config.toml`.

## Entwicklung

Faderust ist in Rust geschrieben, die Oberfläche in Svelte mit Tauri 2. Planung und Fahrplan stehen in [`docs/PLAN.md`](docs/PLAN.md).

| Pfad | Inhalt |
|---|---|
| `crates/engine` | Audio-Kern: Geräte (WASAPI über cpal), Ringpuffer, Drift-Ausgleich, Mix-Matrix, Effekte, Programm-Steuerung |
| `crates/app` | Tauri-App: Befehle, Konfiguration, Tray, Updater |
| `crates/app/ui` | Oberfläche in Svelte 5 |

Voraussetzungen: Rust (stable), Node.js 22, unter Windows WebView2.

```sh
cargo test                             # Engine und App
cd crates/app/ui && npm ci && npm run build && cd ../../..
cargo run --release -p faderust        # App starten
```

- `npm run dev` in `crates/app/ui` zeigt die Oberfläche im Browser mit simulierten Geräten. Parallel `cargo run -p faderust` lädt sie mit Live-Neuladen in die App.
- Setup bauen: in `crates/app` den Befehl `ui/node_modules/.bin/tauri build` ausführen, das Ergebnis liegt unter `target/release/bundle/nsis/`. Releases entstehen über den Workflow „Release“.
- Engine ohne Oberfläche ausprobieren:

  ```sh
  cargo run --release -p faderust-engine --example mixer -- --list
  cargo run --release -p faderust-engine --example mixer -- \
      --input "USB" --input "CABLE Output" --output "PS5:30" --output "Buds:80" --gain 1:0:-6
  ```

  `--output NAME:MS` setzt den Puffer pro Ausgang, `--gain I:O:DB` und `--mute I:O` stellen die Matrix ein. Geräte werden über Teile des Namens gefunden.

Vor einem Commit laufen `cargo fmt`, `cargo clippy -- -D warnings` und `cargo test`. Neue Abhängigkeiten brauchen eine freizügige Lizenz (`cargo deny check`).

## Lizenz

[MIT](LICENSE)
