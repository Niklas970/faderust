# iced prototype

Four desk strips (fader with scale, LED meter, readout, mute) drawn natively
with [iced](https://iced.rs) instead of Tauri/WebView2. Not connected to the
engine; the meters show made-up levels. Standalone on purpose: it is not a
workspace member, so it adds nothing to the app, CI or cargo-deny.

```
cd prototypes/iced-gui
cargo run --release
```

Window like the real one: title bar, desk (output masters left, inputs with A1-A3 and FX), matrix and the dialogs (Programme, Effekte, Presets, Windows-Sound, Design, Einstellungen). Drag the cap, scroll to nudge, double-click for 0 dB.
`FADERUST_VIEW=matrix` and `FADERUST_DIALOG=fx|apps|presets|windows|design|settings` pick what a screenshot shows.
`FADERUST_SHOT=out.png` saves a screenshot and exits (set `ICED_BACKEND=tiny-skia`
for the software renderer, e.g. under Xvfb).
