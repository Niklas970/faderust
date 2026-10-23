# Faderust

Windows-Audio-Mixer in Rust (Ersatz für Voicemeeter Potato). Volle Planung: `docs/PLAN.md`. Lies sie zuerst.

## Regeln für die Arbeit

- Antworte auf Deutsch, kurz und knapp. Code, Kommentare und Commit-Messages dürfen Englisch sein.
- Lizenz: MIT. Neue Abhängigkeiten nur mit freizügigen Lizenzen (Prüfung per `cargo deny check`, Liste in `deny.toml`).
- Kein eigener Kernel-Treiber. Virtuelle Geräte über VB-Cable, nicht mitliefern.
- Im Audio-Thread: keine Speicherzuweisung, keine Sperren, kein Logging über Mutex. Kommunikation über `rtrb` oder Atomics.
- Plattform: Windows 10/11, WASAPI. Audio-Code hinter einem Trait kapseln, damit später ein PipeWire-Backend möglich bleibt.
- Vor jedem Commit: `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`.
- Nichts mit Zugangsdaten oder persönlichen Pfaden committen. Das Repo ist öffentlich.

## Stack

`windows` (WASAPI) bzw. `cpal` für den Prototyp, `rtrb`, `rubato`, `serde` + TOML, Oberfläche Tauri 2 mit Svelte.

## Stand

E0-Prototyp liegt in `crates/engine/examples/e0_passthrough.rs` (Anleitung `docs/E0.md`). Offen: 30-Minuten-Messung auf Niklas' Rechner, Ergebnisse in `docs/E0.md` eintragen. Danach E1.

## Design

Akzent Kupfer, Signal Türkis. Logo: `assets/logo.svg`. Eine Spalte pro Ausgang, ein Fader pro Eingang.
