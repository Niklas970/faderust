//! [`AppRouter`] on Windows. Audio sessions come from WASAPI; the device per
//! program goes through the undocumented `AudioPolicyConfig` factory that the
//! Settings app uses (EarTrumpet and SoundSwitch call it the same way).

use std::ffi::c_void;
use std::ptr::null_mut;

use windows::core::{IInspectable, IUnknown, Interface, GUID, HRESULT, HSTRING, PWSTR};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Foundation::{CloseHandle, RPC_E_CHANGED_MODE, S_OK};
use windows::Win32::Media::Audio::{
    eConsole, eMultimedia, eRender, AudioSessionStateExpired, EDataFlow, ERole,
    IAudioSessionControl2, IAudioSessionManager2, IMMDevice, IMMDeviceEnumerator,
    MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
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

use super::{group_sessions, AppAudio, AppRouter, RenderDevice, RouterError};

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
        for device in active_render(&enumerator()?)? {
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
        let mut pids = Vec::new();
        for device in active_render(&enumerator()?)? {
            // One broken driver should not hide the other devices' programs.
            let _ = session_pids(&device, &mut pids);
        }
        let policy = Policy::new().ok();
        Ok(group_sessions(pids.into_iter().filter_map(|pid| {
            let exe = exe_name(pid)?;
            let device = policy.as_ref().and_then(|p| p.get(pid));
            Some((pid, exe, device))
        })))
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
}

/// COM for the current thread, released again on drop. Callers create their
/// COM objects after this guard so those are released first.
struct Com(bool);

impl Com {
    fn init() -> Result<Self> {
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

fn active_render(enumerator: &IMMDeviceEnumerator) -> Result<Vec<IMMDevice>> {
    let devices =
        unsafe { enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) }.map_err(err)?;
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
    let store = unsafe { device.OpenPropertyStore(STGM_READ) }.map_err(err)?;
    let value = unsafe { store.GetValue(&PKEY_Device_FriendlyName) }.map_err(err)?;
    take_string(unsafe { PropVariantToStringAlloc(&value) }.map_err(err)?)
}

fn session_pids(device: &IMMDevice, pids: &mut Vec<u32>) -> Result<()> {
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
        let Ok(session) = session.cast::<IAudioSessionControl2>() else {
            continue;
        };
        if unsafe { session.IsSystemSoundsSession() } == S_OK {
            continue;
        }
        if let Ok(pid) = unsafe { session.GetProcessId() } {
            if pid != 0 && !pids.contains(&pid) {
                pids.push(pid);
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
        Err(RouterError(
            "Diese Windows-Version erlaubt kein Ausgabegerät pro Programm".into(),
        ))
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

#[cfg(test)]
mod tests {
    use super::*;

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
