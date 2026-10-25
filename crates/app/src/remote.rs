//! Local control interface for the Stream Deck plugin (`streamdeck/`):
//! a WebSocket on 127.0.0.1 that speaks JSON, one thread per client.
//!
//! The server pushes `state` (labels, faders, presets) and `devices`
//! (Windows playback and recording devices) whenever they change; clients
//! send the requests in [`Request`].

use std::fs;
use std::io;
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::Path;
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
const PROTOCOL: u32 = 2;
/// Where the plugin's token file is read from, inside the config folder.
pub const TOKEN_FILE: &str = "remote.token";
/// A client must say hello within this long, or it is turned away.
const HELLO_WITHIN: Duration = Duration::from_secs(5);
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
    /// First message of every client: plugin version and the token from
    /// `remote.token`, which only programs of this Windows user can read.
    Hello {
        plugin: String,
        #[serde(default)]
        token: String,
    },
}

/// Who is connected, for the plugin update in the settings.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Link {
    /// Clients that said hello with the right token.
    pub clients: usize,
    /// Version from the latest `hello`.
    pub version: Option<String>,
    /// A client was turned away since the last one got in: a plugin from
    /// before the token, or a wrong one.
    pub refused: bool,
}

/// Counts a signed-in client for as long as it lives.
struct Connected<'a>(&'a AppState);

impl<'a> Connected<'a> {
    fn new(state: &'a AppState, version: String) -> Self {
        let mut link = state.streamdeck.lock().unwrap_or_else(|e| e.into_inner());
        link.clients += 1;
        link.version = Some(version);
        link.refused = false;
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

/// The token file's content: 32 random bytes as hex, created on first start.
/// The config folder belongs to the Windows user, so other users on the same
/// PC (who share `127.0.0.1`) cannot read it.
pub fn load_or_create_token(path: &Path) -> io::Result<String> {
    let valid = |t: &str| t.len() == 64 && t.bytes().all(|b| b.is_ascii_hexdigit());
    match fs::read_to_string(path) {
        Ok(text) if valid(text.trim()) => return Ok(text.trim().to_string()),
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| io::Error::other(e.to_string()))?;
    let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, &token)?;
    Ok(token)
}

/// Compares without stopping at the first difference. An empty token never
/// matches (no token file, no remote control).
pub fn token_matches(expected: &str, given: &str) -> bool {
    !expected.is_empty()
        && expected.len() == given.len()
        && expected
            .bytes()
            .zip(given.bytes())
            .fold(0, |diff, (a, b)| diff | (a ^ b))
            == 0
}

/// The plugin version of a valid hello, `None` for anything else.
fn authenticate(expected: &str, request: &Request) -> Option<String> {
    match request {
        Request::Hello { plugin, token } if token_matches(expected, token) => Some(plugin.clone()),
        _ => None,
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
    let started = Instant::now();
    // Set once the client has said hello with the right token; nothing is
    // sent to or taken from it before.
    let mut connected: Option<Connected> = None;

    let mut sent_state = None;
    let mut sent_devices = None;
    let mut devices_at: Option<Instant> = None;
    loop {
        match ws.read() {
            Ok(Message::Text(text)) => {
                let request = serde_json::from_str::<Request>(&text);
                if connected.is_none() {
                    match request.ok().and_then(|r| authenticate(&state.token, &r)) {
                        Some(version) => connected = Some(Connected::new(&state, version)),
                        None => return refuse(&mut ws, &state),
                    }
                    continue;
                }
                let result = request
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

        if connected.is_none() {
            if started.elapsed() >= HELLO_WITHIN {
                return refuse(&mut ws, &state);
            }
            continue;
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

/// Tells a client without a valid hello why it is dropped, and notes it for
/// the settings ("plugin needs an update").
fn refuse(
    ws: &mut WebSocket<TcpStream>,
    state: &AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    state
        .streamdeck
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .refused = true;
    let _ = send(
        ws,
        &Push::Error {
            message: faderust_engine::lang::tr(
                "Nicht angemeldet. Das Plugin in Faderust unter „Geräte“ aktualisieren.",
                "Not signed in. Update the plugin in Faderust under “Devices”.",
            ),
        },
    );
    let _ = ws.close(None);
    Ok(())
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
        Request::Hello { plugin, .. } => {
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
        let hello =
            serde_json::from_str::<Request>(r#"{"type":"hello","plugin":"0.3.1.0","token":"ab"}"#);
        assert_eq!(
            hello.unwrap(),
            Request::Hello {
                plugin: "0.3.1.0".into(),
                token: "ab".into()
            }
        );
        // A hello without a token parses, and is turned away later.
        let old = serde_json::from_str::<Request>(r#"{"type":"hello","plugin":"0.3.0.0"}"#);
        assert!(matches!(old.unwrap(), Request::Hello { token, .. } if token.is_empty()));
    }

    #[test]
    fn token_must_match_exactly() {
        assert!(token_matches("abc123", "abc123"));
        assert!(!token_matches("abc123", "abc124"));
        assert!(!token_matches("abc123", "abc12"));
        assert!(!token_matches("abc123", ""));
        // No token file, no remote control.
        assert!(!token_matches("", ""));
    }

    #[test]
    fn only_a_hello_with_the_token_signs_in() {
        let hello = |token: &str| Request::Hello {
            plugin: "0.3.1.0".into(),
            token: token.into(),
        };
        assert_eq!(
            authenticate("secret", &hello("secret")).as_deref(),
            Some("0.3.1.0")
        );
        assert_eq!(authenticate("secret", &hello("wrong")), None);
        assert_eq!(authenticate("secret", &hello("")), None);
        let gain = Request::SetGain {
            input: 0,
            output: 0,
            db: 0.0,
        };
        assert_eq!(authenticate("secret", &gain), None);
    }

    #[test]
    fn token_file_is_created_once_and_kept() {
        let dir = std::env::temp_dir().join(format!("faderust-token-{}", std::process::id()));
        let path = dir.join(TOKEN_FILE);
        let _ = fs::remove_dir_all(&dir);

        let first = load_or_create_token(&path).unwrap();
        assert_eq!(first.len(), 64);
        assert!(first.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_eq!(load_or_create_token(&path).unwrap(), first);

        // Garbage is replaced by a fresh token.
        fs::write(&path, "not a token").unwrap();
        let second = load_or_create_token(&path).unwrap();
        assert_ne!(second, first);
        assert_eq!(fs::read_to_string(&path).unwrap(), second);
        let _ = fs::remove_dir_all(&dir);
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
