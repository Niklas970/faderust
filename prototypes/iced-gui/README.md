# iced prototype

Four desk strips (fader with scale, LED meter, readout, mute) drawn natively
with [iced](https://iced.rs) instead of Tauri/WebView2. Not connected to the
engine; the meters show made-up levels. Standalone on purpose: it is not a
workspace member, so it adds nothing to the app, CI or cargo-deny.

```
cd prototypes/iced-gui
cargo run --release
```

Drag the cap, scroll to nudge, double-click for 0 dB. Header with Pult/Matrix switch, a routing matrix (click a cell to toggle) and a settings dialog (`FADERUST_VIEW=matrix`, `FADERUST_DIALOG=1` for screenshots).
`FADERUST_SHOT=out.png` saves a screenshot and exits (set `ICED_BACKEND=tiny-skia`
for the software renderer, e.g. under Xvfb).
