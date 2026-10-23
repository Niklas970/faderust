# Faderust – Projektplan

Audio-Mixer für Windows in Rust als Ersatz für Voicemeeter Potato.
Jeder Ausgang bekommt seine eigene Mischung, Geräte (v. a. Bluetooth) kommen und gehen ohne Neustart der Audio-Engine, moderne Oberfläche.

Status: Planung abgeschlossen, Repo noch leer. Stand 2026-10-24.

## Ausgangslage (Niklas' Setup)

- Eingänge: USB-Mikrofon, PC-Audio, PS5 (über USB-auf-Audio-Adapter)
- Ausgänge: PS5 (USB) und mehrere Bluetooth-Geräte, je Ausgang eine eigene Mischung
- Probleme mit Voicemeeter Potato: Bluetooth-Geräte funktionieren nicht automatisch, Audio Engine muss jedes Mal neu gestartet werden, veraltete Optik, Last
- Elgato Wave Link ausprobiert: Richtung stimmt, aber zu wenig Routing-Freiheit
- Plattform jetzt: Windows. Linux (PipeWire) eventuell später als zweites Backend

## Entscheidungen

| Thema | Entscheidung |
|---|---|
| Name | **Faderust** (Rustmix war vergeben). Vor dem Anlegen auf GitHub und crates.io prüfen |
| Repo | öffentlich, `Niklas970/faderust` (öffentliche Repos haben kostenlose Actions-Minuten). Vorher sicherstellen, dass keine Zugangsdaten im Verlauf stehen |
| Lizenz | MIT (Datei `LICENSE`). Keine Rechtsberatung. Abhängigkeiten mit `cargo-deny` prüfen (`deny.toml`) |
| Logo | Drei Fader auf verschiedenen Höhen, Kupfer (Rust) mit türkisem Signalpunkt. Datei `assets/logo.svg` |
| Treiber | Kein eigener Kernel-Treiber. Virtuelle Geräte über VB-Cable (nicht mitliefern, nur verlinken, eigene Lizenz) |

## Architektur

```
Eingänge                 Rust-Engine                       Ausgänge
USB-Mikro   ──capture──▶ ┌──────────────────────────┐ ──▶ PS5 (USB)
PC-Audio    ──capture──▶ │ 1 Ringpuffer je Gerät    │ ──▶ Bluetooth A
 (VB-Cable)              │ 2 Drift-Ausgleich        │ ──▶ Bluetooth B
PS5 (USB)   ──capture──▶ │ 3 Mix-Matrix  N × M      │
                         │ 4 Mikro-Effekte (EQ,Gate)│
                         │ 5 Pegelmessung           │
                         ├──────────────────────────┤
                         │ Steuerung: Hot-Plug,     │
                         │ Nachrichten zur GUI      │
                         └──────────────────────────┘
```

- Audio-Thread ohne Speicherzuweisung und ohne Sperren im Echtzeitpfad
- Geräte-Threads schreiben und lesen über lock-freie Ringpuffer (`rtrb`)
- Taktunterschiede zwischen Geräten gleicht ein adaptiver Resampler (`rubato`) anhand des Puffer-Füllstands aus
- Mix-Matrix: pro Eingang und Ausgang ein Pegel plus Stummschalter
- Geräteüberwachung über die Windows-Benachrichtigungen (`IMMNotificationClient`), damit Bluetooth-Geräte automatisch auftauchen und wieder verbinden

## Oberfläche

Eine Spalte pro Ausgang, darin ein Fader pro Eingang, daneben Pegelanzeige. Verbindungsstatus oben in jeder Spalte. Dark Mode, Tray-Icon. Farben: Kupfer als Akzent, Türkis für Signal.

## Technik-Stack

| Baustein | Wahl | Grund |
|---|---|---|
| Audio-Zugriff | `windows` Crate (WASAPI), für den Prototyp `cpal` | Kontrolle über Event-Modus und Geräte-Benachrichtigungen |
| Puffer | `rtrb` | Lock-frei |
| Resampling | `rubato` | Drift zwischen USB und Bluetooth |
| Oberfläche | Tauri 2 mit Svelte | Beste Optik bei wenig Aufwand. Alternative `egui` (schneller, nüchterner) |
| Konfiguration | `serde` mit TOML | Presets und Geräte-Zuordnung als lesbare Datei |

## Fahrplan (Feierabend-Projekt, 6–10 Wochen)

| Etappe | Inhalt | Aufwand |
|---|---|---|
| E0 Prototyp | Mikrofon aufnehmen, auf Bluetooth-Ausgang wiedergeben. Latenz und Drift über 30 Minuten messen | 0,5–1 Woche |
| E1 Engine-Kern | Mehrere Eingänge, Mix-Matrix, Ringpuffer, Drift-Ausgleich, Pegelwerte | 1–2 Wochen |
| E2 Hot-Plug und Bluetooth | Geräte-Benachrichtigungen, automatisches Wiederverbinden, wechselnde Latenz | 1–1,5 Wochen |
| E3 Oberfläche | Tauri-Fenster mit Fadern, Meter, Dark Mode, Tray | 1–2 Wochen |
| E4 PC-Audio je Programm | VB-Cable einbinden, Zuordnung Programm → Kabel erleichtern | 1–2 Wochen |
| E5 Feinschliff | Mikro-EQ und Gate, Presets, Autostart, Fehlermeldungen | 1–1,5 Wochen |

Erst E0 bis E3 bauen, dann entscheiden, ob E4 und E5 nötig sind.

## Risiken

| Risiko | Stufe | Gegenmaßnahme |
|---|---|---|
| Takt-Drift zwischen Geräten | hoch | Adaptiver Resampler, Füllstandsregelung, früh im Prototyp testen |
| Bluetooth-Latenz und Aussetzer | hoch | Größere Puffer für Bluetooth, Latenz pro Ausgang einstellbar |
| Programme mit Exklusivmodus | mittel | Hinweis in der Oberfläche, Programm auf Shared-Modus stellen |
| Abhängigkeit von VB-Cable | mittel | Klare Schnittstelle für virtuelle Geräte, späterer Austausch möglich |
| Umfang wächst | niedrig | Etappen einhalten |

## Geplante Repo-Struktur

```
faderust/
├─ crates/
│  ├─ engine/        # Audio-Kern: WASAPI, Puffer, Mix-Matrix
│  └─ app/           # Tauri-Oberfläche
├─ assets/logo.svg
├─ docs/PLAN.md
├─ .github/workflows/ci.yml   # cargo fmt, clippy, test (windows-latest)
├─ CLAUDE.md
├─ LICENSE
└─ README.md
```

## Nächste Schritte

1. Namen auf GitHub und crates.io prüfen, Repo anlegen, Lizenzdateien und `.gitignore` committen
2. Cargo-Workspace mit `crates/engine` und `crates/app` aufsetzen
3. E0: Mikrofon (USB) auf Bluetooth-Kopfhörer leiten, Latenz und Drift über 30 Minuten messen
4. Mix-Matrix mit zwei Eingängen und zwei Ausgängen, dann erst die Oberfläche
