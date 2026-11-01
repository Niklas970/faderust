<p align="center"><b>English</b> · <a href="README.de.md">Deutsch</a> · <a href="README.fr.md">Français</a> · <a href="README.es.md">Español</a> · <a href="README.it.md">Italiano</a> · <a href="README.nl.md">Nederlands</a> · <a href="README.pl.md">Polski</a></p>

<p align="center"><img src="assets/logo.svg" width="112" alt="Faderust logo"></p>

<h1 align="center">Faderust</h1>

<p align="center">
  Audio mixer for Windows, a lean, open replacement for Voicemeeter.<br>
  Every output gets its own mix, devices may come and go while it runs, and Faderust brings its own virtual cables.
</p>

<p align="center">
  <a href="https://github.com/Niklas970/faderust/releases/latest"><img src="https://img.shields.io/github/v/release/Niklas970/faderust?label=Download" alt="Latest version"></a>
  <a href="https://github.com/Niklas970/faderust/actions/workflows/ci.yml"><img src="https://github.com/Niklas970/faderust/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/Niklas970/faderust" alt="MIT license"></a>
</p>

![Faderust desk with devices, programs and three outputs](assets/screenshot.png)

<p align="center">
  <a href="assets/faderust-reel.mp4"><img src="assets/faderust-reel.gif" width="270" alt="Faderust in 40 seconds"></a><br>
  <sub>Preview, <a href="assets/faderust-reel.mp4">full-quality video</a> (download)</sub>
</p>

> **Alpha.** Faderust runs, but has hardly been tested on real hardware yet. Please report problems as an [issue](https://github.com/Niklas970/faderust/issues).

## What Faderust does

- **A desk like Voicemeeter.** Every device and every program gets a strip with a fader, MUTE and the buttons A1, A2, A3 for the outputs. Each output has its own master fader.
- **Matrix for fine work.** Alternatively as a table: every input on every output set individually, e.g. the microphone quieter on the headphones than in the stream.
- **Capture programs directly.** Discord, Spotify or the browser get their own strip with effects, without a virtual cable per program. Faderust hides silent programs and brings them back individually on request ("N silent, show").
- **Its own virtual cables, no extra driver.** The app sets up the "Faderust Kabel" (up to four) itself, for example one for the microphone and one for programs. Details under [Faderust Kabel](#faderust-kabel).
- **Tidy.** Outputs can be hidden with the × and come back by themselves as soon as sound plays on them again.
- **Devices come and go.** Switch Bluetooth headphones off and on, unplug a USB device: Faderust reconnects without interrupting the other devices.
- **Multipoint friendly.** Faderust releases Bluetooth outputs during silence (after 30 s, adjustable per output) so the headphones can switch to the phone. When sound returns, they reopen by themselves.
- **Effects per input:** noise suppression ([RNNoise](https://jmvalin.ca/demo/rnnoise/), against keyboard and fan noise), low-cut, three-band EQ and gate. Noise suppression runs on 48 kHz devices and delays the sound by 10 ms.
- **German and English.** The language follows Windows and can be changed under "Design".
- **Settings save themselves.** The devices dialog applies changes shortly after the last input.
- **Self-test.** Under "Devices" Faderust checks devices, buffers, feedback, remote control and the Stream Deck plugin and shows what is fine and what is not. The report can be copied for an issue.
- **Windows sound in one place:** default devices for playback, recording and calls, plus the volume per device.
- **Presets, autostart in the tray, in-app updates** (with signature check) and a **Stream Deck plugin**.
- **Compact and on top.** Two buttons in the title bar: a narrow view without labels and "Always on top", e.g. next to a game. Both are remembered.
- **Your own colors:** accent and signal color freely selectable, dark (OLED black) or light.

## Views

<table>
  <tr>
    <td width="68%"><img src="assets/matrix.png" alt="Matrix: every input on every output set individually"></td>
    <td><img src="assets/design.png" alt="Design dialog with view, colors and appearance"></td>
  </tr>
  <tr>
    <td><b>Matrix:</b> every input on every output individually, e.g. the microphone quieter on the headphones than in the stream.</td>
    <td><b>Design:</b> desk or matrix, side of the outputs, colors, dark or light.</td>
  </tr>
</table>

## Installation

1. Download `Faderust_*_x64-setup.exe` from [Releases](https://github.com/Niklas970/faderust/releases/latest) and run it. The setup needs no admin rights and speaks German or English.
2. The setup is not signed, so Windows SmartScreen warns: "More info" → "Run anyway".
3. Open Faderust from the Start menu. The app reports new versions at the top of the title bar; a click on the version number checks right away.

Requirement: Windows 10 (version 2004 or later) or Windows 11.

## Getting started

1. Open **Devices** and create inputs (microphone, line-in, Faderust Kabel) and outputs (headphones, speakers, stream). Bluetooth needs about 80 ms of buffer, USB usually 30 to 60 ms.
2. In the **desk**, use A1–A3 to choose where an input plays. The fader controls all enabled outputs together.
3. Using faders: drag, mouse wheel (finer with Shift), double-click for 0 dB, or click the dB number below and type a value.
4. **Effects** under a strip opens low-cut, EQ and gate.
5. Under **Design**, switch between desk and matrix and choose the side of the outputs and the colors.

### Programs

**+ Program** in the desk lists all programs that are playing sound right now. **+ Input** captures one directly. So that you do not hear it twice, Faderust first moves the program to a free Faderust Kabel that you do not listen to (if set up) and then captures it. A program that is silent at the moment is hidden by Faderust; **N silent, show** brings it back.

### Microphone for Discord, Teams & co.

Under **Devices**, **+ Program mic** creates an output on a free Faderust Kabel that carries only the microphone with its effects. In the program, choose the recording side of the same cable as the microphone. Faderust mutes feedback loops (an output plays onto the cable that an input comes from) and shows "Loop".

### Faderust Kabel

The Faderust Kabel is a virtual USB sound device (stereo, 48 kHz, 24 bit) that Faderust provides itself. Faderust shows it as "Faderust Kabel (Playback)" and "Faderust Kabel (Recording)"; what a program plays to the playback side comes out again on the recording side.

1. Under **Devices** → **Faderust Kabel** click **Set up** (once, with an administrator prompt). Faderust downloads [usbip-win2](https://github.com/vadimgrn/usbip-win2), verifies the checksum and installs it in the background. It does not appear separately in the Windows app list.
2. Switch **Faderust Kabel** on and choose the number (1 to 4), for example one for the microphone and one for programs. Windows shows them in its settings as "Faderust Cable", "Faderust Cable 2" and so on.
3. **Remove** undoes the setup including usbip-win2.

Note: the cable is still an experiment and has been tested on little hardware.

## Limits

- Programs remember faders, outputs and effects as long as their strip stays in the desk: if Discord restarts, it comes back with its old settings. If you remove the strip under "Devices", they are gone.
- No custom kernel driver. The virtual cables ("Faderust Kabel", up to four) are USB sound devices that Faderust provides itself via USB/IP. Windows uses its built-in USB audio driver for them; the USB/IP part is [usbip-win2](https://github.com/vadimgrn/usbip-win2) (BSD-2, signed by Microsoft), which the setup installs once.
- Windows delivers sound per program, not per browser tab or web app. Everything from one browser runs through the same audio process.
- After a release, Bluetooth needs about a second until sound returns. If you do not want that, set the output to "Never release".
- The "program → device" assignment uses the same undocumented Windows interface as the volume mixer in the settings.

Settings: `%APPDATA%\io.github.niklas970.faderust\config.toml`, next to it the log `faderust.log`, which helps with errors.

## Development

Rust for the audio core and the interface ([iced](https://iced.rs), no WebView2).

| Path | Contents |
|---|---|
| `crates/engine` | Audio core: devices (WASAPI via cpal), ring buffers, drift compensation, mix matrix, effects, program capture |
| `crates/usbcable` | Faderust Kabel: virtual USB audio device over USB/IP |
| `crates/core` | Logic behind the interface: configuration, devices, programs, cables, remote control, updater |
| `crates/gui` | The `faderust` program: iced window, tray, autostart |
| `streamdeck` | Stream Deck plugin |

Requirements: Rust (stable); Node.js for the Stream Deck plugin.

```sh
cargo test
cargo run --release -p faderust
```

- Build the setup: `makensis` with `installer/faderust.nsi` (parameters are at the top of the file). Releases are built by the "Release" workflow.
- Engine without interface: `cargo run --release -p faderust-engine --example mixer -- --list`

Before a commit: `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`. New dependencies need a permissive license (`cargo deny check`).

## Stream Deck

The plugin in [`streamdeck/`](streamdeck/README.md) controls Faderust from the Elgato Stream Deck: faders via dial or key, mute, presets and the Windows default device. Double-click `com.niklas970.faderust.streamDeckPlugin` from the [Releases](https://github.com/Niklas970/faderust/releases) (Stream Deck 7.1 or newer). From then on the setup ships the plugin: under "Devices" Faderust shows whether a newer plugin version is available and opens it with one click in the Stream Deck software, which then only asks for confirmation.

## License

[MIT](LICENSE)
