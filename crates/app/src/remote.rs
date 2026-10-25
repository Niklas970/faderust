//! Local control interface for the Stream Deck plugin (`streamdeck/`):
//! a WebSocket on 127.0.0.1 that speaks JSON, one thread per client.
//!
//! The server pushes `state` (labels, faders, presets) and `devices`
//! (Windows playback and recording devices) whenever they change; clients
//! send the requests in [`Request`].

use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tungstenite::handshake::server::{ErrorResponse, Request as Handshake, Response};
use tungstenite::http::StatusCode;
use tungstenite::{Message, WebSocket};

use crate::audio::Command;
use crate::config::{Config, Gain};
use crate::AppState;

/// Fixed so the plugin finds Faderust without setup.
pub const PORT: u16 = 47811;
const PROTOCOL: u32 = 1;
const POLL: Duration = Duration::from_millis(50);
const DEVICES_EVERY: Duration = Duration::from_secs(2);

#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Request {
    SetGain {
        input: usize,
        output: usize,
        db: f32,
    },
    /// Moves a fader by `delta` dB.
    AddGain {
        input: usize,
        output: usize,
        delta: f32,
    },
    SetMuted {
        input: usize,
        output: usize,
        muted: bool,
    },
    ToggleMuted {
        input: usize,
        output: usize,
    },
    LoadPreset {
        name: String,
    },
    SetDefaultDevice {
        id: String,
        #[serde(default)]
        communications: bool,
    },
    /// Sent once on connecting by plugins from 0.4.6 on.
    Hello {
        plugin: String,
    },
}

/// Who is connected, for the plugin update in the settings.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Link {
    pub clients: usize,
    /// Version from the latest `hello`; `None` for plugins without one.
    pub version: Option<String>,
}

/// Counts a client for as long as it lives.
struct Connected<'a>(&'a AppState);

impl<'a> Connected<'a> {
    fn new(state: &'a AppState) -> Self {
        let mut link = state.streamdeck.lock().unwrap_or_else(|e| e.into_inner());
        link.clients += 1;
        link.version = None;
        drop(link);
        Self(state)
    }
}

impl Drop for Connected<'_> {
    fn drop(&mut self) {
        let mut link = self.0.streamdeck.lock().unwrap_or_else(|e| e.into_inner());
        link.clients = link.clients.saturating_sub(1);
        if link.clients == 0 {
            link.version = None;
        }
    }
}

/// Whether `bundled` is a newer dotted version than `installed`
/// ("0.3.0.0" equals "0.3.0"). Anything unparsable counts as not newer.
pub fn is_newer(bundled: &str, installed: &str) -> bool {
    fn parts(v: &str) -> Option<Vec<u64>> {
        let mut parts = v
            .trim()
            .split('.')
            .map(|p| p.parse().ok())
            .collect::<Option<Vec<u64>>>()?;
        while parts.last() == Some(&0) {
            parts.pop();
        }
        Some(parts)
    }
    match (parts(bundled), parts(installed)) {
        (Some(b), Some(i)) => b > i,
        _ => false,
    }
}

#[derive(Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct State {
    protocol: u32,
    inputs: Vec<String>,
    outputs: Vec<String>,
    /// `gains[input][output]`.
    gains: Vec<Vec<Gain>>,
    presets: Vec<String>,
    /// The preset the faders match right now.
    active_preset: Option<String>,
}

impl State {
    fn of(config: &Config) -> Self {
        Self {
            protocol: PROTOCOL,
            inputs: config.inputs.iter().map(|i| i.label.clone()).collect(),
            outputs: config.outputs.iter().map(|o| o.label.clone()).collect(),
            gains: config.gains.clone(),
            presets: config.presets.iter().map(|p| p.name.clone()).collect(),
            active_preset: config
                .presets
                .iter()
                .find(|p| p.gains == config.gains)
                .map(|p| p.name.clone()),
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Push<'a> {
    State(&'a State),
    Devices(&'a crate::WindowsAudio),
    Error { message: String },
}

/// Starts listening; without the port (taken by something else) Faderust
/// runs on without remote control.
pub fn spawn(app: AppHandle) {
    let listener = match TcpListener::bind((Ipv4Addr::LOCALHOST, PORT)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("remote control: port {PORT}: {e}");
            return;
        }
    };
    thread::Builder::new()
        .name("faderust-remote".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                let app = app.clone();
                let _ = thread::Builder::new()
                    .name("faderust-remote-client".into())
                    .spawn(move || {
                        if let Err(e) = serve(&app, stream) {
                            eprintln!("remote control: {e}");
                        }
                    });
            }
        })
        .expect("spawn remote control thread");
}

/// Browsers always send an `Origin`; the Stream Deck plugin (Node.js) does
/// not. Refusing it keeps web pages from moving faders.
#[allow(clippy::result_large_err)] // signature set by tungstenite
fn check_origin(req: &Handshake, res: Response) -> Result<Response, ErrorResponse> {
    if req.headers().contains_key("origin") {
        let mut err = ErrorResponse::new(Some("browsers are not allowed".into()));
        *err.status_mut() = StatusCode::FORBIDDEN;
        return Err(err);
    }
    Ok(res)
}

fn serve(app: &AppHandle, stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut ws = tungstenite::accept_hdr(stream, check_origin)?;
    ws.get_ref().set_read_timeout(Some(POLL))?;
    let state: tauri::State<AppState> = app.state();
    let _connected = Connected::new(&state);

    let mut sent_state = None;
    let mut sent_devices = None;
    let mut devices_at: Option<Instant> = None;
    loop {
        match ws.read() {
            Ok(Message::Text(text)) => {
                let result = serde_json::from_str::<Request>(&text)
                    .map_err(|e| format!("Unbekannte Anfrage: {e}"))
                    .and_then(|req| handle(&state, req));
                match result {
                    // A new default device should show at once.
                    Ok(true) => devices_at = None,
                    Ok(false) => {}
                    Err(message) => send(&mut ws, &Push::Error { message })?,
                }
            }
            Ok(Message::Close(_)) => return Ok(()),
            Ok(_) => {}
            Err(tungstenite::Error::Io(e))
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
                return Ok(())
            }
            Err(e) => return Err(e.into()),
        }

        let now = State::of(&state.config.lock().unwrap_or_else(|e| e.into_inner()));
        if sent_state.as_ref() != Some(&now) {
            send(&mut ws, &Push::State(&now))?;
            sent_state = Some(now);
        }

        if devices_at.is_none_or(|t| t.elapsed() >= DEVICES_EVERY) {
            devices_at = Some(Instant::now());
            match crate::windows_audio() {
                Ok(devices) if sent_devices.as_ref() != Some(&devices) => {
                    send(&mut ws, &Push::Devices(&devices))?;
                    sent_devices = Some(devices);
                }
                Ok(_) => {}
                Err(e) => eprintln!("remote control: devices: {e}"),
            }
        }
    }
}

/// Carries out one request; `true` when the Windows devices changed.
fn handle(state: &AppState, req: Request) -> Result<bool, String> {
    let cmd = match req {
        Request::SetGain { input, output, db } => Command::SetGain(input, output, db),
        Request::AddGain {
            input,
            output,
            delta,
        } => Command::AddGain(input, output, delta),
        Request::SetMuted {
            input,
            output,
            muted,
        } => Command::SetMuted(input, output, muted),
        Request::ToggleMuted { input, output } => Command::ToggleMuted(input, output),
        Request::LoadPreset { name } => return crate::apply_preset(state, &name).map(|_| false),
        Request::SetDefaultDevice { id, communications } => {
            return crate::set_default(&id, communications).map(|_| true)
        }
        Request::Hello { plugin } => {
            state
                .streamdeck
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .version = Some(plugin);
            return Ok(false);
        }
    };
    crate::send(state, cmd).map(|_| false)
}

fn send(ws: &mut WebSocket<TcpStream>, push: &Push) -> Result<(), Box<dyn std::error::Error>> {
    ws.send(Message::text(serde_json::to_string(push)?))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_requests() {
        let parse = |s| serde_json::from_str::<Request>(s).unwrap();
        assert_eq!(
            parse(r#"{"type":"addGain","input":1,"output":0,"delta":-2}"#),
            Request::AddGain {
                input: 1,
                output: 0,
                delta: -2.0
            }
        );
        assert_eq!(
            parse(r#"{"type":"setDefaultDevice","id":"{0.0.0}"}"#),
            Request::SetDefaultDevice {
                id: "{0.0.0}".into(),
                communications: false
            }
        );
        assert!(serde_json::from_str::<Request>(r#"{"type":"format"}"#).is_err());
    }

    #[test]
    fn compares_plugin_versions() {
        assert!(is_newer("0.4.6", "0.4.5"));
        assert!(is_newer("0.10.0", "0.9.9"));
        assert!(is_newer("0.3.1.0", "0.3.0.0"));
        assert!(!is_newer("0.3.0", "0.3.0.0"));
        assert!(!is_newer("0.3.0", "0.3.1"));
        assert!(!is_newer("", "0.3.0"));
        assert!(!is_newer("0.3.0", "dev"));
    }

    #[test]
    fn parses_hello() {
        let hello = serde_json::from_str::<Request>(r#"{"type":"hello","plugin":"0.3.0.0"}"#);
        assert_eq!(
            hello.unwrap(),
            Request::Hello {
                plugin: "0.3.0.0".into()
            }
        );
    }

    #[test]
    fn refuses_browsers() {
        use tungstenite::client::IntoClientRequest;

        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            (0..2)
                .map(|_| {
                    let (stream, _) = listener.accept().unwrap();
                    tungstenite::accept_hdr(stream, check_origin).is_ok()
                })
                .collect::<Vec<_>>()
        });
        let connect = |origin: Option<&str>| {
            let mut req = format!("ws://{addr}").into_client_request().unwrap();
            if let Some(o) = origin {
                req.headers_mut().insert("origin", o.parse().unwrap());
            }
            tungstenite::client(req, TcpStream::connect(addr).unwrap()).is_ok()
        };
        assert!(connect(None));
        assert!(!connect(Some("https://example.com")));
        assert_eq!(server.join().unwrap(), [true, false]);
    }

    #[test]
    fn state_lists_labels_and_presets() {
        let config = Config {
            inputs: vec![crate::config::InputEntry {
                label: "Mikro".into(),
                device: "USB".into(),
                fx: Default::default(),
                muted: false,
            }],
            outputs: vec![crate::config::OutputEntry {
                label: "PS5".into(),
                device: "PS5".into(),
                buffer_ms: 60,
                master: Default::default(),
                release_secs: 30,
            }],
            gains: vec![],
            presets: vec![],
        }
        .normalized();
        let json = serde_json::to_value(Push::State(&State::of(&config))).unwrap();
        assert_eq!(json["type"], "state");
        assert_eq!(json["inputs"][0], "Mikro");
        assert_eq!(json["gains"][0][0]["db"], 0.0);
        assert_eq!(json["activePreset"], serde_json::Value::Null);

        let mut config = config;
        config.save_preset("Spielen");
        let json = serde_json::to_value(Push::State(&State::of(&config))).unwrap();
        assert_eq!(json["presets"][0], "Spielen");
        assert_eq!(json["activePreset"], "Spielen");
    }
}
