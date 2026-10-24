<p align="center"><img src="assets/logo.svg" width="128" alt="Faderust logo"></p>

# Faderust

Audio-Mixer für Windows in Rust, gedacht als Ersatz für Voicemeeter Potato.
Jeder Ausgang bekommt seine eigene Mischung, Geräte (vor allem Bluetooth) kommen und gehen ohne Neustart der Audio-Engine.

> Status: frühe Entwicklung, noch nicht nutzbar. Planung und Fahrplan: [`docs/PLAN.md`](docs/PLAN.md).

## Aufbau

| Pfad | Inhalt |
|---|---|
| `crates/engine` | Audio-Kern: WASAPI, Ringpuffer, Drift-Ausgleich, Mix-Matrix |
| `crates/app` | Oberfläche (später Tauri 2 mit Svelte) |

## Bauen

```sh
cargo build
cargo test
```

Prototyp E0 (Mikrofon auf Bluetooth, Latenz und Drift messen): siehe [`docs/E0.md`](docs/E0.md).

Engine ausprobieren (mehrere Ein- und Ausgänge, eine Mischung pro Ausgang, Geräte dürfen im Betrieb kommen und gehen):

```sh
cargo run --release -p faderust-engine --example mixer -- --list
cargo run --release -p faderust-engine --example mixer -- \
    --input "USB" --input "CABLE Output" --output "PS5:30" --output "Buds:80" --gain 1:0:-6
```

`--output NAME:MS` setzt den Puffer pro Ausgang (Bluetooth braucht mehr), `--gain I:O:DB` und `--mute I:O` stellen die Matrix ein. Geräte werden über Teile des Namens gefunden. Fehlt eins beim Start oder geht es verloren, verbindet es sich automatisch, sobald es wieder da ist, ohne die anderen zu unterbrechen.

Virtuelle Geräte kommen von [VB-Cable](https://vb-audio.com/Cable/) (separat installieren, eigene Lizenz).

## App (Oberfläche)

Voraussetzungen: Rust, Node.js 22, WebView2 (bei Windows 10/11 dabei).

```sh
cd crates/app/ui
npm ci
npm run build              # baut die Oberfläche nach crates/app/ui/dist
cd ../../..
cargo run --release -p faderust
```

Zum Entwickeln mit Live-Neuladen: in `crates/app/ui` `npm run dev` starten und parallel `cargo run -p faderust` (Debug-Builds laden die Oberfläche von `localhost:5173`). `npm run dev` allein zeigt die Oberfläche im Browser mit simulierten Geräten.

Installer bauen: `cd crates/app && ui/node_modules/.bin/tauri build` (nach `npm run build`), ergibt das Setup unter `target/release/bundle/nsis/`. Es folgt der Windows-Sprache (Deutsch oder Englisch). Fertige Setups liegen unter [Releases](https://github.com/Niklas970/faderust/releases); ein Release entsteht über den Workflow „Release“ (Tag `v*` oder von Hand).

Die Einstellungen (Geräte, Pegel) liegen in `%APPDATA%\io.github.niklas970.faderust\config.toml`. Das Fenster schließen lässt den Mixer im Tray weiterlaufen; beenden über das Tray-Menü. Unter **Geräte → Start** lässt sich Faderust mit Windows starten (direkt in den Tray). Ein zweiter Start holt nur das laufende Fenster nach vorn.

## Effekte, Presets, Meldungen

- **Effekte pro Eingang:** Klick auf einen Eingang oben in der Leiste öffnet Low-Cut, einen Drei-Band-EQ (Bass, Präsenz, Höhen) und ein Gate. Änderungen wirken sofort; aktive Effekte zeigt ein „FX“ am Eingang.
- **Presets:** Unter **Presets** die aktuellen Fader unter einem Namen speichern und später mit einem Klick laden.
- **Meldungen:** Getrennte, wieder verbundene oder blockierte Geräte meldet Faderust unten rechts.

## Programme auf eigene Fader

Ein Programm (Spotify, Discord, Browser …) bekommt einen eigenen Fader, wenn es auf ein virtuelles Kabel spielt und Faderust die andere Seite des Kabels als Eingang aufnimmt:

1. [VB-Cable](https://vb-audio.com/Cable/) installieren (kostenlos, wird nicht mitgeliefert) und neu starten.
2. In Faderust unter **Programme** bei „CABLE Output … ist noch kein Eingang“ auf **Als Eingang anlegen** tippen.
3. Im selben Dialog das Programm auf **CABLE Input** stellen.

Windows merkt sich das Gerät pro Programm, auch nach einem Neustart. Mehrere Programme getrennt brauchen mehrere Kabel (VB-Cable A+B). Faderust setzt die Zuordnung über dieselbe nicht dokumentierte Windows-Schnittstelle wie die Einstellungen-App („Lautstärkemixer“), dort lässt sie sich auch von Hand ändern.

## Lizenz

[MIT](LICENSE)
