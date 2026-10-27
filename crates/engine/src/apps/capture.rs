//! Recording a single program (WASAPI process loopback, Windows 10 2004 and
//! later). Windows hands over what the program plays, whatever device it
//! plays on, so every program can become its own input with its own effects
//! without a virtual cable per program.
//!
//! The program keeps playing on its own device too. To hear it only through
//! Faderust, point it at a device nobody listens to (e.g. a VB-Cable whose
//! other side is no input).

use std::mem::{size_of, ManuallyDrop};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use windows::core::{implement, Interface, Ref, HRESULT, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows::Win32::Media::Audio::{
    ActivateAudioInterfaceAsync, IActivateAudioInterfaceAsyncOperation,
    IActivateAudioInterfaceCompletionHandler, IActivateAudioInterfaceCompletionHandler_Impl,
    IAudioCaptureClient, IAudioClient, AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED,
    AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM, AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
    AUDCLNT_STREAMFLAGS_LOOPBACK, AUDIOCLIENT_ACTIVATION_PARAMS, AUDIOCLIENT_ACTIVATION_PARAMS_0,
    AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK, AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS,
    PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE, VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK,
    WAVEFORMATEX,
};
use windows::Win32::System::Com::StructuredStorage::{
    PROPVARIANT, PROPVARIANT_0, PROPVARIANT_0_0, PROPVARIANT_0_0_0,
};
use windows::Win32::System::Com::BLOB;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    CreateEventW, OpenProcess, SetEvent, WaitForMultipleObjects, WaitForSingleObject,
    PROCESS_SYNCHRONIZE,
};
use windows::Win32::System::Variant::VT_BLOB;

use super::wasapi::{Com, WasapiRouter};
use super::{capture_roots, AppAudio, AppRouter};
use crate::backend::{ErrorCallback, InputCallback};

/// Programs arrive in this format; Windows converts from whatever they play.
pub const SAMPLE_RATE: u32 = 48_000;
pub const CHANNELS: u16 = 2;

/// How long Windows may take to hand over a recording client.
const ACTIVATE_TIMEOUT_MS: u32 = 5_000;
/// How long `open` waits for the recording thread to start.
const OPEN_TIMEOUT: Duration = Duration::from_secs(8);
/// After Windows failed to start a program's recording, wait this long
/// before asking again. Retrying every second piles up stuck requests in
/// the Windows audio service, which then breaks the program's sound.
const RETRY_AFTER: Duration = Duration::from_secs(30);
/// How often a running recording looks for processes of its program that
/// started after it (a second window, a restarted audio process).
const REFRESH_EVERY: Duration = Duration::from_secs(2);
/// A recording that ran this long without taking Faderust down is trusted.
const TRUSTED_AFTER: Duration = Duration::from_secs(5);

/// `WAVE_FORMAT_IEEE_FLOAT` from mmreg.h.
const WAVE_FORMAT_IEEE_FLOAT: u16 = 3;

/// Frames of silence handed on when Windows reports a silent packet.
const SILENCE_FRAMES: usize = 2048;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureError(pub String);

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn err(e: windows::core::Error) -> CaptureError {
    CaptureError(e.message())
}

static CRASH_MARKER: OnceLock<PathBuf> = OnceLock::new();

/// Program recording is new and goes deep into Windows. While one starts,
/// a marker file names the program; if Faderust dies before the recording
/// has run a few seconds, the file is still there at the next start and
/// [`crashed_last_time`] reports the program so the app can skip it.
pub fn set_crash_marker(path: PathBuf) {
    let _ = CRASH_MARKER.set(path);
}

/// The program whose recording was starting when Faderust last died, if any.
/// Clears the marker.
pub fn crashed_last_time() -> Option<String> {
    let path = CRASH_MARKER.get()?;
    let exe = std::fs::read_to_string(path).ok()?;
    let _ = std::fs::remove_file(path);
    Some(exe.trim().to_string()).filter(|e| !e.is_empty())
}

fn mark_starting(exe: &str) {
    if let Some(path) = CRASH_MARKER.get() {
        let _ = std::fs::write(path, exe);
    }
}

fn clear_marker() {
    if let Some(path) = CRASH_MARKER.get() {
        let _ = std::fs::remove_file(path);
    }
}

/// Programs that play audio right now, except Faderust itself.
pub fn programs() -> Result<Vec<AppAudio>, CaptureError> {
    let own = std::process::id();
    let mut apps = WasapiRouter::new().apps().map_err(|e| CaptureError(e.0))?;
    apps.retain(|a| !a.pids.contains(&own));
    Ok(apps)
}

/// Running processes as `(pid, parent, exe)`.
fn processes() -> Result<Vec<(u32, u32, String)>, CaptureError> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }.map_err(err)?;
    let mut list = Vec::new();
    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut more = unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok();
    while more {
        let len = entry
            .szExeFile
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(entry.szExeFile.len());
        list.push((
            entry.th32ProcessID,
            entry.th32ParentProcessID,
            String::from_utf16_lossy(&entry.szExeFile[..len]),
        ));
        more = unsafe { Process32NextW(snapshot, &mut entry) }.is_ok();
    }
    unsafe {
        let _ = CloseHandle(snapshot);
    }
    Ok(list)
}

/// Programs whose recording failed to start, with the time it failed.
static FAILED: Mutex<Vec<(String, Instant)>> = Mutex::new(Vec::new());

fn failed(exe: &str) {
    if let Ok(mut f) = FAILED.lock() {
        let exe = exe.to_lowercase();
        f.retain(|(e, _)| *e != exe);
        f.push((exe, Instant::now()));
    }
}

/// Time left before `exe` may be tried again, if it failed recently.
fn backoff(exe: &str) -> Option<Duration> {
    let f = FAILED.lock().ok()?;
    let exe = exe.to_lowercase();
    let (_, at) = f.iter().find(|(e, _)| *e == exe)?;
    RETRY_AFTER
        .checked_sub(at.elapsed())
        .filter(|d| !d.is_zero())
}

/// Records the program `exe` (e.g. `spotify.exe`) until dropped. `data` gets
/// interleaved stereo `f32` at [`SAMPLE_RATE`] on a thread of its own;
/// `error` runs once if recording stops by itself (the program quit).
pub fn open(
    exe: &str,
    mut data: InputCallback,
    mut error: ErrorCallback,
) -> Result<ProgramCapture, CaptureError> {
    if let Some(left) = backoff(exe) {
        return Err(CaptureError(
            crate::lang::tr(
                "Aufnahme von {exe} ist eben fehlgeschlagen, neuer Versuch in {s} s",
                "Recording {exe} just failed, trying again in {s} s",
            )
            .replace("{exe}", exe)
            .replace("{s}", &left.as_secs().max(1).to_string()),
        ));
    }
    let app = programs()?
        .into_iter()
        .find(|a| a.exe.eq_ignore_ascii_case(exe))
        .ok_or_else(|| {
            CaptureError(
                crate::lang::tr(
                    "{exe} spielt gerade keinen Ton ab",
                    "{exe} is not playing sound right now",
                )
                .replace("{exe}", exe),
            )
        })?;
    let exe_owned = app.exe.clone();
    let roots = capture_roots(&app.pids, &processes()?);
    if roots.is_empty() {
        return Err(CaptureError(
            crate::lang::tr("{exe} läuft nicht mehr", "{exe} is no longer running")
                .replace("{exe}", exe),
        ));
    }

    mark_starting(exe);
    let stop = Arc::new(AtomicBool::new(false));
    let (ready_tx, ready_rx) = mpsc::channel();
    let thread_stop = Arc::clone(&stop);
    let spawned = std::thread::Builder::new()
        .name(format!("capture {exe}"))
        .spawn(move || {
            // A panic here must not take the engine with it.
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                match Recorder::new(&exe_owned, &roots) {
                    Ok(mut recorder) => {
                        let _ = ready_tx.send(Ok(()));
                        recorder.run(&thread_stop, &mut *data)
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        true
                    }
                }
            }));
            // Faderust is still alive, so this recording did not crash it.
            clear_marker();
            let stopped_by_us = matches!(outcome, Ok(true)) || thread_stop.load(Ordering::Relaxed);
            if !stopped_by_us {
                error();
            }
        });
    let thread = match spawned {
        Ok(t) => t,
        Err(e) => {
            clear_marker();
            return Err(CaptureError(e.to_string()));
        }
    };
    match ready_rx.recv_timeout(OPEN_TIMEOUT) {
        Ok(Ok(())) => Ok(ProgramCapture {
            stop,
            thread: Some(thread),
        }),
        Ok(Err(e)) => {
            let _ = thread.join();
            clear_marker();
            failed(exe);
            Err(e)
        }
        Err(_) => {
            // Stuck in Windows or gone: let it finish on its own.
            stop.store(true, Ordering::Relaxed);
            clear_marker();
            failed(exe);
            Err(CaptureError("Programm-Aufnahme startet nicht".into()))
        }
    }
}

/// A running program recording; dropping it stops the recording.
pub struct ProgramCapture {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for ProgramCapture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// One loopback client per root process, all on the recording thread.
struct Recorder {
    exe: String,
    // Field order matters: COM objects go before COM itself.
    sources: Vec<Source>,
    _com: Com,
}

struct Source {
    pid: u32,
    /// Signalled when the process ends; `None` if it could not be opened.
    process: Option<HANDLE>,
    client: IAudioClient,
    capture: IAudioCaptureClient,
    event: HANDLE,
}

impl Recorder {
    fn new(exe: &str, roots: &[u32]) -> Result<Self, CaptureError> {
        let com = Com::init().map_err(|e| CaptureError(e.0))?;
        let mut recorder = Self {
            exe: exe.to_string(),
            sources: Vec::new(),
            _com: com,
        };
        for &pid in roots {
            recorder.sources.push(Source::new(pid)?);
        }
        for s in &recorder.sources {
            unsafe { s.client.Start() }.map_err(err)?;
        }
        Ok(recorder)
    }

    /// Starts recording processes of the program that appeared since. A
    /// program that gets a new process after the recording began (a second
    /// window, a restarted audio service) would otherwise stay silent.
    fn add_new_roots(&mut self) {
        let Ok(apps) = programs() else { return };
        let Some(app) = apps.iter().find(|a| a.exe.eq_ignore_ascii_case(&self.exe)) else {
            return;
        };
        let Ok(list) = processes() else { return };
        for pid in new_roots(&capture_roots(&app.pids, &list), &self.sources) {
            // A failure is not retried: asking again and again piles up
            // requests in the Windows audio service.
            if let Ok(source) = Source::new(pid) {
                if unsafe { source.client.Start() }.is_ok() {
                    self.sources.push(source);
                }
            }
        }
    }

    /// Hands on audio until `stop` is set (true) or every process ended (false).
    fn run(&mut self, stop: &AtomicBool, data: &mut dyn FnMut(&[f32])) -> bool {
        let silence = vec![0.0f32; SILENCE_FRAMES * CHANNELS as usize];
        let started = Instant::now();
        let mut refreshed = Instant::now();
        let mut trusted = false;
        while !stop.load(Ordering::Relaxed) {
            if !trusted && started.elapsed() >= TRUSTED_AFTER {
                clear_marker();
                trusted = true;
            }
            self.sources.retain(|s| !s.ended());
            if self.sources.is_empty() {
                return false;
            }
            if refreshed.elapsed() >= REFRESH_EVERY {
                self.add_new_roots();
                refreshed = Instant::now();
            }
            let events: Vec<HANDLE> = self.sources.iter().map(|s| s.event).collect();
            let woke = unsafe { WaitForMultipleObjects(&events, false, 100) };
            if woke == WAIT_TIMEOUT {
                continue;
            }
            for s in &self.sources {
                if s.drain(data, &silence).is_err() {
                    return false;
                }
            }
        }
        true
    }
}

/// The processes in `roots` that no source records yet.
fn new_roots(roots: &[u32], sources: &[Source]) -> Vec<u32> {
    roots
        .iter()
        .copied()
        .filter(|pid| !sources.iter().any(|s| s.pid == *pid))
        .collect()
}

impl Source {
    fn new(pid: u32) -> Result<Self, CaptureError> {
        let process = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) }.ok();
        let client = match activate(pid) {
            Ok(c) => c,
            Err(e) => {
                close(process);
                return Err(e);
            }
        };
        let format = WAVEFORMATEX {
            wFormatTag: WAVE_FORMAT_IEEE_FLOAT,
            nChannels: CHANNELS,
            nSamplesPerSec: SAMPLE_RATE,
            nAvgBytesPerSec: SAMPLE_RATE * u32::from(CHANNELS) * 4,
            nBlockAlign: CHANNELS * 4,
            wBitsPerSample: 32,
            cbSize: 0,
        };
        let setup = (|| {
            unsafe {
                client.Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    AUDCLNT_STREAMFLAGS_LOOPBACK
                        | AUDCLNT_STREAMFLAGS_EVENTCALLBACK
                        | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
                    200_000, // 20 ms in 100 ns units
                    0,
                    &format,
                    None,
                )
            }
            .map_err(err)?;
            let event = unsafe { CreateEventW(None, false, false, PCWSTR::null()) }.map_err(err)?;
            let capture: IAudioCaptureClient = match unsafe { client.GetService() } {
                Ok(c) => c,
                Err(e) => {
                    unsafe {
                        let _ = CloseHandle(event);
                    }
                    return Err(err(e));
                }
            };
            if let Err(e) = unsafe { client.SetEventHandle(event) } {
                unsafe {
                    let _ = CloseHandle(event);
                }
                return Err(err(e));
            }
            Ok((capture, event))
        })();
        match setup {
            Ok((capture, event)) => Ok(Self {
                pid,
                process,
                client,
                capture,
                event,
            }),
            Err(e) => {
                close(process);
                Err(e)
            }
        }
    }

    /// Whether the recorded process has ended.
    fn ended(&self) -> bool {
        self.process
            .is_some_and(|p| unsafe { WaitForSingleObject(p, 0) } == WAIT_OBJECT_0)
    }

    fn drain(&self, data: &mut dyn FnMut(&[f32]), silence: &[f32]) -> windows::core::Result<()> {
        let channels = CHANNELS as usize;
        while unsafe { self.capture.GetNextPacketSize() }? > 0 {
            let mut buffer = std::ptr::null_mut();
            let mut frames = 0u32;
            let mut flags = 0u32;
            unsafe {
                self.capture
                    .GetBuffer(&mut buffer, &mut frames, &mut flags, None, None)
            }?;
            let len = frames as usize * channels;
            if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 || buffer.is_null() {
                let mut left = len;
                while left > 0 {
                    let n = left.min(silence.len());
                    data(&silence[..n]);
                    left -= n;
                }
            } else {
                // The client delivers the float format it was initialised with.
                let samples = unsafe { std::slice::from_raw_parts(buffer as *const f32, len) };
                data(samples);
            }
            unsafe { self.capture.ReleaseBuffer(frames) }?;
        }
        Ok(())
    }
}

impl Drop for Source {
    fn drop(&mut self) {
        unsafe {
            let _ = self.client.Stop();
            let _ = CloseHandle(self.event);
        }
        close(self.process);
    }
}

fn close(handle: Option<HANDLE>) {
    if let Some(h) = handle {
        unsafe {
            let _ = CloseHandle(h);
        }
    }
}

/// Signals an event when the asynchronous activation is done. `implement`
/// makes it agile, as the API requires; it may complete on any thread.
#[implement(IActivateAudioInterfaceCompletionHandler)]
struct Activated(HANDLE);

impl IActivateAudioInterfaceCompletionHandler_Impl for Activated_Impl {
    fn ActivateCompleted(
        &self,
        _operation: Ref<IActivateAudioInterfaceAsyncOperation>,
    ) -> windows::core::Result<()> {
        unsafe { SetEvent(self.0) }
    }
}

fn activate(pid: u32) -> Result<IAudioClient, CaptureError> {
    let mut params = AUDIOCLIENT_ACTIVATION_PARAMS {
        ActivationType: AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK,
        Anonymous: AUDIOCLIENT_ACTIVATION_PARAMS_0 {
            ProcessLoopbackParams: AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS {
                TargetProcessId: pid,
                ProcessLoopbackMode: PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE,
            },
        },
    };
    // `PROPVARIANT` frees its contents on drop (`PropVariantClear`), which
    // would hand `params`, a stack value, to `CoTaskMemFree` and corrupt the
    // heap. The blob only borrows `params`: never drop it.
    let blob = ManuallyDrop::new(PROPVARIANT {
        Anonymous: PROPVARIANT_0 {
            Anonymous: ManuallyDrop::new(PROPVARIANT_0_0 {
                vt: VT_BLOB,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: PROPVARIANT_0_0_0 {
                    blob: BLOB {
                        cbSize: size_of::<AUDIOCLIENT_ACTIVATION_PARAMS>() as u32,
                        pBlobData: &mut params as *mut _ as *mut u8,
                    },
                },
            }),
        },
    });

    let done = unsafe { CreateEventW(None, true, false, PCWSTR::null()) }.map_err(err)?;
    let handler: IActivateAudioInterfaceCompletionHandler = Activated(done).into();
    let result = (|| {
        let operation = unsafe {
            ActivateAudioInterfaceAsync(
                VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK,
                &IAudioClient::IID,
                Some(&*blob),
                &handler,
            )
        }
        .map_err(err)?;
        if unsafe { WaitForSingleObject(done, ACTIVATE_TIMEOUT_MS) } != WAIT_OBJECT_0 {
            // The handler may still fire later: leave the event open for it.
            return Err(CaptureError(crate::lang::tr(
                "Windows hat die Aufnahme nicht gestartet",
                "Windows did not start the recording",
            )));
        }
        let mut hr = HRESULT(0);
        let mut unknown = None;
        unsafe { operation.GetActivateResult(&mut hr, &mut unknown) }.map_err(err)?;
        hr.ok().map_err(|e| {
            CaptureError(format!(
                "{}: {}",
                crate::lang::tr(
                    "Programm-Aufnahme nicht möglich (braucht Windows 10 2004 oder neuer)",
                    "Program recording is not possible (needs Windows 10 2004 or newer)"
                ),
                e.message()
            ))
        })?;
        unknown
            .ok_or_else(|| CaptureError("keine Aufnahme erhalten".into()))?
            .cast::<IAudioClient>()
            .map_err(err)
    })();
    if unsafe { WaitForSingleObject(done, 0) } == WAIT_OBJECT_0 {
        unsafe {
            let _ = CloseHandle(done);
        }
    }
    // `blob` points at `params`; both live on this frame until here.
    result
}
