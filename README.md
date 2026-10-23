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

Virtuelle Geräte kommen von [VB-Cable](https://vb-audio.com/Cable/) (separat installieren, eigene Lizenz).

## Lizenz

[MIT](LICENSE)
