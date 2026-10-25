//! [`AppRouter`] and [`SystemAudio`] on Windows. Sessions and volumes come
//! from the documented WASAPI interfaces. Two things have no public API and
//! go through the undocumented interfaces the Settings app uses (EarTrumpet
//! and SoundSwitch call them the same way): the device per program
//! (`AudioPolicyConfig` factory) and the default devices (`IPolicyConfig`).

use std::ffi::c_void;
use std::ptr::{null, null_mut};

use windows::core::{IInspectable, IUnknown, Interface, GUID, HRESULT, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Devices::FunctionDiscovery::{
    PKEY_Device_EnumeratorName, PKEY_Device_FriendlyName,
};
use windows::Win32::Foundation::{CloseHandle, PROPERTYKEY, RPC_E_CHANGED_MODE, S_OK};
use windows::Win32::Media::Audio::Endpoints::{IAudioEndpointVolume, IAudioMeterInformation};
use windows::Win32::Media::Audio::{
    eCapture, eCommunications, eConsole, eMultimedia, eRender, AudioSessionStateExpired, EDataFlow,
    ERole, IAudioSessionControl2, IAudioSessionManager2, IMMDevice, IMMDeviceEnumerator,
    ISimpleAudioVolume, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::StructuredStorage::PropVariantToStringAlloc;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL,
    COINIT_MULTITHREADED, STGM_READ,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::System::WinRT::RoGetActivationFactory;

use super::{
    group_sessions, AppAudio, AppRouter, Endpoint, Flow, RenderDevice, Role, RouterError, Session,
    SystemAudio,
};

type Result<T> = std::result::Result<T, RouterError>;

fn err(e: windows::core::Error) -> RouterError {
    RouterError(e.message())
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WasapiRouter;

impl WasapiRouter {
    pub fn new() -> Self {
        Self
    }
}

impl AppRouter for WasapiRouter {
    fn render_devices(&self) -> Result<Vec<RenderDevice>> {
        let _com = Com::init()?;
        let mut devices = Vec::new();
        for device in active(&enumerator()?, eRender)? {
            let id = device_id(&device)?;
            let name = friendly_name(&device).unwrap_or_else(|_| id.clone());
            devices.push(RenderDevice { id, name });
        }
        Ok(devices)
    }

    fn apps(&self) -> Result<Vec<AppAudio>> {
        let _com = Com::init()?;
        // A program pinned to a cable has its session there, not on the
        // default device: look at all of them.
        let mut found: Vec<(u32, f32, bool)> = Vec::new();
        for device in active(&enumerator()?, eRender)? {
            // One broken driver should not hide the other devices' programs.
            let _ = for_each_session(&device, |pid, session| {
                let Ok(volume) = session.cast::<ISimpleAudioVolume>() else {
                    return;
                };
                if !found.iter().any(|(p, ..)| *p == pid) {
                    let level = unsafe { volume.GetMasterVolume() }.unwrap_or(1.0);
                    let muted = unsafe { volume.GetMute() }.is_ok_and(|m| m.as_bool());
                    found.push((pid, level, muted));
                }
            });
        }
        let policy = Policy::new().ok();
        Ok(group_sessions(found.into_iter().filter_map(
            |(pid, volume, muted)| {
                Some(Session {
                    pid,
                    exe: exe_name(pid)?,
                    device: policy.as_ref().and_then(|p| p.get(pid)),
                    volume,
                    muted,
                })
            },
        )))
    }

    fn set_output(&self, pids: &[u32], device: Option<&str>) -> Result<()> {
        let _com = Com::init()?;
        let policy = Policy::new()?;
        let path = device.map_or_else(HSTRING::new, |id| HSTRING::from(device_path(id)));
        for &pid in pids {
            policy.set(pid, &path)?;
        }
        Ok(())
    }

    fn set_app_volume(&self, pids: &[u32], volume: Option<f32>, muted: Option<bool>) -> Result<()> {
        let _com = Com::init()?;
        let mut result = Ok(());
        for device in active(&enumerator()?, eRender)? {
            let _ = for_each_session(&device, |pid, session| {
                if !pids.contains(&pid) {
                    return;
                }
                let Ok(session) = session.cast::<ISimpleAudioVolume>() else {
                    return;
                };
                if let Some(v) = volume {
                    if let Err(e) = unsafe { session.SetMasterVolume(v.clamp(0.0, 1.0), null()) } {
                        result = Err(err(e));
                    }
                }
                if let Some(m) = muted {
                    if let Err(e) = unsafe { session.SetMute(m, null()) } {
                        result = Err(err(e));
                    }
                }
            });
        }
        result
    }

    fn levels(&self) -> Result<Vec<(u32, f32)>> {
        let _com = Com::init()?;
        let mut peaks: Vec<(u32, f32)> = Vec::new();
        for device in active(&enumerator()?, eRender)? {
            let _ = for_each_session(&device, |pid, session| {
                let Ok(meter) = session.cast::<IAudioMeterInformation>() else {
                    return;
                };
                let peak = unsafe { meter.GetPeakValue() }.unwrap_or(0.0);
                match peaks.iter_mut().find(|(p, _)| *p == pid) {
                    Some((_, v)) => *v = v.max(peak),
                    None => peaks.push((pid, peak)),
                }
            });
        }
        Ok(peaks)
    }
}

impl SystemAudio for WasapiRouter {
    fn endpoints(&self) -> Result<Vec<Endpoint>> {
        let _com = Com::init()?;
        let enumerator = enumerator()?;
        let mut endpoints = Vec::new();
        for (flow, data_flow) in [(Flow::Playback, eRender), (Flow::Recording, eCapture)] {
            let default_id = |role| {
                unsafe { enumerator.GetDefaultAudioEndpoint(data_flow, role) }
                    .ok()
                    .and_then(|d| device_id(&d).ok())
            };
            let (default, communications) = (default_id(eConsole), default_id(eCommunications));
            for device in active(&enumerator, data_flow)? {
                let id = device_id(&device)?;
                let name = friendly_name(&device).unwrap_or_else(|_| id.clone());
                let volume: Option<IAudioEndpointVolume> =
                    unsafe { device.Activate(CLSCTX_ALL, None) }.ok();
                let level = volume
                    .as_ref()
                    .and_then(|v| unsafe { v.GetMasterVolumeLevelScalar() }.ok())
                    .unwrap_or(1.0);
                let muted = volume
                    .as_ref()
                    .is_some_and(|v| unsafe { v.GetMute() }.is_ok_and(|m| m.as_bool()));
                endpoints.push(Endpoint {
                    is_default: default.as_deref() == Some(id.as_str()),
                    is_default_communications: communications.as_deref() == Some(id.as_str()),
                    id,
                    name,
                    flow,
                    volume: level,
                    muted,
                });
            }
        }
        Ok(endpoints)
    }

    fn set_default(&self, id: &str, role: Role) -> Result<()> {
        let _com = Com::init()?;
        let config = PolicyConfig::new()?;
        let roles: &[ERole] = match role {
            Role::Default => &[eConsole, eMultimedia],
            Role::Communications => &[eCommunications],
        };
        for &r in roles {
            config.set_default_endpoint(id, r)?;
        }
        Ok(())
    }

    fn set_endpoint_volume(
        &self,
        id: &str,
        volume: Option<f32>,
        muted: Option<bool>,
    ) -> Result<()> {
        let _com = Com::init()?;
        let device = unsafe { enumerator()?.GetDevice(&HSTRING::from(id)) }.map_err(err)?;
        let control: IAudioEndpointVolume =
            unsafe { device.Activate(CLSCTX_ALL, None) }.map_err(err)?;
        if let Some(v) = volume {
            unsafe { control.SetMasterVolumeLevelScalar(v.clamp(0.0, 1.0), null()) }
                .map_err(err)?;
        }
        if let Some(m) = muted {
            unsafe { control.SetMute(m, null()) }.map_err(err)?;
        }
        Ok(())
    }
}

/// COM for the current thread, released again on drop. Callers create their
/// COM objects after this guard so those are released first.
pub(crate) struct Com(bool);

impl Com {
    pub(crate) fn init() -> Result<Self> {
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if hr.is_ok() {
            Ok(Self(true))
        } else if hr == RPC_E_CHANGED_MODE {
            // Already initialised as STA by someone else: usable, not ours.
            Ok(Self(false))
        } else {
            Err(err(hr.into()))
        }
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}

fn enumerator() -> Result<IMMDeviceEnumerator> {
    unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }.map_err(err)
}

fn active(enumerator: &IMMDeviceEnumerator, flow: EDataFlow) -> Result<Vec<IMMDevice>> {
    let devices =
        unsafe { enumerator.EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE) }.map_err(err)?;
    let count = unsafe { devices.GetCount() }.map_err(err)?;
    (0..count)
        .map(|i| unsafe { devices.Item(i) }.map_err(err))
        .collect()
}

/// Takes ownership of a COM-allocated string.
fn take_string(p: PWSTR) -> Result<String> {
    let s = unsafe { p.to_string() };
    unsafe { CoTaskMemFree(Some(p.0 as *const c_void)) };
    s.map_err(|e| RouterError(e.to_string()))
}

fn device_id(device: &IMMDevice) -> Result<String> {
    take_string(unsafe { device.GetId() }.map_err(err)?)
}

fn friendly_name(device: &IMMDevice) -> Result<String> {
    string_property(device, &PKEY_Device_FriendlyName)
}

fn string_property(device: &IMMDevice, key: &PROPERTYKEY) -> Result<String> {
    let store = unsafe { device.OpenPropertyStore(STGM_READ) }.map_err(err)?;
    let value = unsafe { store.GetValue(key) }.map_err(err)?;
    take_string(unsafe { PropVariantToStringAlloc(&value) }.map_err(err)?)
}

/// Path of the driver interface behind an endpoint, e.g.
/// `{1}.\\?\bthenum#...` for Bluetooth headphones.
const PKEY_ENDPOINT_INTERFACE_PATH: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_u128(0xb3f8fa53_0004_438e_9003_51a46e139bfc),
    pid: 2,
};

/// Whether the endpoint with this id sits on Bluetooth (classic or LE).
/// `None` when Windows does not say.
pub fn is_bluetooth(id: &str) -> Option<bool> {
    let _com = Com::init().ok()?;
    let device = unsafe { enumerator().ok()?.GetDevice(&HSTRING::from(id)) }.ok()?;
    let hints = [PKEY_Device_EnumeratorName, PKEY_ENDPOINT_INTERFACE_PATH]
        .iter()
        .filter_map(|key| string_property(&device, key).ok())
        .collect::<Vec<_>>();
    if hints.is_empty() {
        return None;
    }
    Some(hints.iter().any(|h| bluetooth_hint(h)))
}

/// Bluetooth enumerators: BTHENUM (stereo), BTHHFENUM (hands-free),
/// BTHLEDEVICE / BTHLE (LE Audio).
fn bluetooth_hint(text: &str) -> bool {
    text.to_ascii_lowercase().contains("bth")
}

/// Calls `f(pid, session)` for every live program session on `device`,
/// skipping system sounds.
fn for_each_session(
    device: &IMMDevice,
    mut f: impl FnMut(u32, &IAudioSessionControl2),
) -> Result<()> {
    let manager: IAudioSessionManager2 =
        unsafe { device.Activate(CLSCTX_ALL, None) }.map_err(err)?;
    let sessions = unsafe { manager.GetSessionEnumerator() }.map_err(err)?;
    for i in 0..unsafe { sessions.GetCount() }.map_err(err)? {
        let Ok(session) = (unsafe { sessions.GetSession(i) }) else {
            continue;
        };
        if unsafe { session.GetState() }.is_ok_and(|s| s == AudioSessionStateExpired) {
            continue;
        }
        let Ok(control) = session.cast::<IAudioSessionControl2>() else {
            continue;
        };
        if unsafe { control.IsSystemSoundsSession() } == S_OK {
            continue;
        }
        if let Ok(pid) = unsafe { control.GetProcessId() } {
            if pid != 0 {
                f(pid, &control);
            }
        }
    }
    Ok(())
}

/// File name of the process's executable, e.g. `spotify.exe`.
fn exe_name(pid: u32) -> Option<String> {
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    let found = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
    };
    unsafe {
        let _ = CloseHandle(process);
    }
    found.ok()?;
    let path = String::from_utf16_lossy(&buf[..len as usize]);
    path.rsplit('\\').next().map(str::to_string)
}

// The policy factory names devices by their interface path, not the plain
// endpoint id.
const DEVICE_PREFIX: &str = r"\\?\SWD#MMDEVAPI#";
const RENDER_SUFFIX: &str = "#{e6327cad-dcec-4949-ae8a-991e976a79d2}";

fn device_path(endpoint_id: &str) -> String {
    format!("{DEVICE_PREFIX}{endpoint_id}{RENDER_SUFFIX}")
}

fn endpoint_id(path: &str) -> Option<String> {
    let id = path.strip_prefix(DEVICE_PREFIX).unwrap_or(path);
    let id = match id.len().checked_sub(RENDER_SUFFIX.len()) {
        Some(cut) if id.is_char_boundary(cut) && id[cut..].eq_ignore_ascii_case(RENDER_SUFFIX) => {
            &id[..cut]
        }
        _ => id,
    };
    (!id.is_empty()).then(|| id.to_string())
}

const POLICY_CLASS: &str = "Windows.Media.Internal.AudioPolicyConfig";
// Windows 10 21H2 and later, and the older builds before that.
const POLICY_IIDS: [GUID; 2] = [
    GUID::from_u128(0xab3d4648_e242_459f_b02f_541c70306324),
    GUID::from_u128(0x2a59116d_6c4f_45e0_a74f_707e3fef9258),
];

/// Vtable of `IAudioPolicyConfigFactory`. Both interface versions share it.
#[repr(C)]
struct PolicyVtbl {
    // IUnknown (3) and IInspectable (3), then 19 methods not used here.
    _skipped: [usize; 25],
    set_persisted_default_audio_endpoint: unsafe extern "system" fn(
        this: *mut c_void,
        pid: u32,
        flow: EDataFlow,
        role: ERole,
        device: *mut c_void,
    ) -> HRESULT,
    get_persisted_default_audio_endpoint: unsafe extern "system" fn(
        this: *mut c_void,
        pid: u32,
        flow: EDataFlow,
        role: ERole,
        device: *mut *mut c_void,
    ) -> HRESULT,
}

struct Policy(IUnknown);

impl Policy {
    fn new() -> Result<Self> {
        let factory: IInspectable =
            unsafe { RoGetActivationFactory(&HSTRING::from(POLICY_CLASS)) }.map_err(err)?;
        for iid in POLICY_IIDS {
            let mut raw = null_mut();
            if unsafe { factory.query(&iid, &mut raw) }.is_ok() && !raw.is_null() {
                return Ok(Self(unsafe { IUnknown::from_raw(raw) }));
            }
        }
        Err(RouterError(crate::lang::tr(
            "Diese Windows-Version erlaubt kein Ausgabegerät pro Programm",
            "This Windows version does not allow an output device per program",
        )))
    }

    fn vtbl(&self) -> &PolicyVtbl {
        unsafe { &**(self.0.as_raw() as *const *const PolicyVtbl) }
    }

    /// Pins `pid` to `path`; an empty string means the system default.
    /// Windows keeps separate choices for media and communication, set both.
    fn set(&self, pid: u32, path: &HSTRING) -> Result<()> {
        for role in [eMultimedia, eConsole] {
            unsafe {
                (self.vtbl().set_persisted_default_audio_endpoint)(
                    self.0.as_raw(),
                    pid,
                    eRender,
                    role,
                    std::mem::transmute_copy(path),
                )
            }
            .ok()
            .map_err(err)?;
        }
        Ok(())
    }

    fn get(&self, pid: u32) -> Option<String> {
        let mut raw = null_mut();
        let hr = unsafe {
            (self.vtbl().get_persisted_default_audio_endpoint)(
                self.0.as_raw(),
                pid,
                eRender,
                eMultimedia,
                &mut raw,
            )
        };
        if hr.is_err() || raw.is_null() {
            return None;
        }
        // Takes ownership: freed when dropped.
        let path: HSTRING = unsafe { std::mem::transmute(raw) };
        endpoint_id(&path.to_string())
    }
}

// IPolicyConfig: sets the default devices. Undocumented, stable since
// Windows 7.
const CLSID_POLICY_CONFIG: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);
const IID_POLICY_CONFIG: GUID = GUID::from_u128(0xf8679f50_850a_41cf_9c72_430f290290c8);

#[repr(C)]
struct PolicyConfigVtbl {
    // IUnknown (3), then 10 methods not used here (formats, periods, share
    // mode, property values).
    _skipped: [usize; 13],
    set_default_endpoint:
        unsafe extern "system" fn(this: *mut c_void, device: PCWSTR, role: ERole) -> HRESULT,
}

struct PolicyConfig(IUnknown);

impl PolicyConfig {
    fn new() -> Result<Self> {
        let unknown: IUnknown =
            unsafe { CoCreateInstance(&CLSID_POLICY_CONFIG, None, CLSCTX_ALL) }.map_err(err)?;
        let mut raw = null_mut();
        unsafe { unknown.query(&IID_POLICY_CONFIG, &mut raw) }
            .ok()
            .map_err(err)?;
        if raw.is_null() {
            return Err(RouterError(crate::lang::tr(
                "Standardgeräte lassen sich hier nicht setzen",
                "Default devices cannot be set here",
            )));
        }
        Ok(Self(unsafe { IUnknown::from_raw(raw) }))
    }

    fn set_default_endpoint(&self, id: &str, role: ERole) -> Result<()> {
        let id = HSTRING::from(id);
        unsafe {
            let vtbl = &**(self.0.as_raw() as *const *const PolicyConfigVtbl);
            (vtbl.set_default_endpoint)(self.0.as_raw(), PCWSTR(id.as_ptr()), role)
        }
        .ok()
        .map_err(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_bluetooth_endpoints() {
        assert!(bluetooth_hint("BTHENUM"));
        assert!(bluetooth_hint(r"{1}.\\?\bthhfenum#{0000111e-0000}"));
        assert!(!bluetooth_hint("USB"));
        assert!(!bluetooth_hint(r"{1}.\\?\hdaudio#func_01&ven_10ec"));
    }

    const ID: &str = "{0.0.0.00000000}.{5f1b3c2a-0000-4000-8000-123456789abc}";

    #[test]
    fn device_paths_round_trip() {
        assert_eq!(endpoint_id(&device_path(ID)).as_deref(), Some(ID));
        assert_eq!(
            endpoint_id(&device_path(ID).to_uppercase()).as_deref(),
            Some(ID.to_uppercase().as_str())
        );
        assert_eq!(endpoint_id(ID).as_deref(), Some(ID));
        assert_eq!(endpoint_id(""), None);
    }
}
