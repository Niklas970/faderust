// Bridge to the Rust side. Outside Tauri (plain `npm run dev` in a browser)
// a small simulation stands in, so the UI can be worked on without devices.

const inTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function tauri() {
  const [core, event] = await Promise.all([
    import("@tauri-apps/api/core"),
    import("@tauri-apps/api/event"),
  ]);
  return { invoke: core.invoke, listen: event.listen };
}

/** App version, e.g. "0.2.0". */
export async function getVersion() {
  if (!inTauri) return __APP_VERSION__;
  const { getVersion } = await import("@tauri-apps/api/app");
  return getVersion();
}

/** Title bar buttons; closing only hides the window (it keeps running in the tray). */
export async function windowAction(action) {
  if (!inTauri) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const w = getCurrentWindow();
  if (action === "minimize") return w.minimize();
  if (action === "maximize") return w.toggleMaximize();
  if (action === "close") return w.close();
}

/** Keeps the window above all others (or not). */
export async function setAlwaysOnTop(on) {
  if (!inTauri) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow().setAlwaysOnTop(on);
}

let pendingUpdate = null;

/** A newer release, as `{ version }`, or null. Checked against GitHub. */
export async function checkUpdate() {
  if (!inTauri)
    return location.search.includes("update") ? { version: "0.4.0" } : null;
  const { check } = await import("@tauri-apps/plugin-updater");
  pendingUpdate = await check();
  return pendingUpdate && { version: pendingUpdate.version };
}

/**
 * Downloads the update found by `checkUpdate`, verifies its signature and
 * starts the setup, which closes Faderust and opens it again afterwards.
 * `onProgress` gets 0…1 while downloading.
 */
export async function installUpdate(onProgress) {
  if (!inTauri) {
    for (let i = 1; i <= 10; i++) {
      await new Promise((r) => setTimeout(r, 120));
      onProgress(i / 10);
    }
    return;
  }
  let total = 0;
  let done = 0;
  await pendingUpdate.downloadAndInstall((e) => {
    if (e.event === "Started") total = e.data.contentLength ?? 0;
    if (e.event === "Progress") {
      done += e.data.chunkLength;
      if (total) onProgress(done / total);
    }
  });
}

export async function getDevices() {
  if (!inTauri) return mock.devices;
  return (await tauri()).invoke("get_devices");
}

/** Programs playing audio and the playback devices they can use. */
export async function getApps() {
  if (!inTauri) return structuredClone(mock.apps);
  return (await tauri()).invoke("get_apps");
}

/** `device` null puts the program back on the Windows default. */
export async function setAppOutput(pids, device) {
  if (!inTauri) return mock.setAppOutput(pids, device);
  return (await tauri()).invoke("set_app_output", { pids, device });
}

/** Program volume in Windows (0…1) and/or mute; `null` leaves a value as it is. */
export async function setAppVolume(pids, volume, muted) {
  if (!inTauri) return mock.setAppVolume(pids, volume, muted);
  return (await tauri()).invoke("set_app_volume", { pids, volume, muted });
}

/** Current peak per process, as `[{ pid, db }]`. */
export async function getAppLevels() {
  if (!inTauri) return mock.appLevels();
  return (await tauri()).invoke("get_app_levels");
}

/** Windows playback and recording devices with defaults and volume. */
export async function getWindowsAudio() {
  if (!inTauri) return structuredClone(mock.windows);
  return (await tauri()).invoke("get_windows_audio");
}

/** Makes a device the Windows default; `communications` only for calls. */
export async function setDefaultDevice(id, communications) {
  if (!inTauri) return mock.setDefault(id, communications);
  return (await tauri()).invoke("set_default_device", { id, communications });
}

/** Device volume in Windows (0…1) and/or mute; `null` leaves a value as it is. */
export async function setDeviceVolume(id, volume, muted) {
  if (!inTauri) return mock.setDeviceVolume(id, volume, muted);
  return (await tauri()).invoke("set_device_volume", { id, volume, muted });
}

/** VB-Audio's virtual cables, by name ("CABLE Input", "CABLE-A Output", …). */
export function isCable(name) {
  const n = name.toLowerCase();
  return n.includes("vb-audio") || n.startsWith("cable");
}

/**
 * The two ends of one virtual cable ("CABLE Input …" plays into
 * "CABLE Output …"), like the engine's `feeds_back`.
 */
export function feedsBack(output, input) {
  const o = output.toLowerCase();
  return (
    o.includes("input") && o.replace("input", "output") === input.toLowerCase()
  );
}

/** The recording end of the cable that `output` plays into. */
export function cableCounterpart(output) {
  return output.replace(/input/i, (m) => (m[0] === "I" ? "Output" : "output"));
}

/** Opens VB-Audio's VB-Cable download page in the browser. */
export async function openCablePage() {
  if (!inTauri)
    return void window.open("https://vb-audio.com/Cable/", "_blank");
  return (await tauri()).invoke("open_cable_page");
}

/** Whether Faderust starts with Windows (into the tray). */
export async function getAutostart() {
  if (!inTauri) return mock.autostart;
  return (await tauri()).invoke("get_autostart");
}

export async function setAutostart(enabled) {
  if (!inTauri) return void (mock.autostart = enabled);
  return (await tauri()).invoke("set_autostart", { enabled });
}

export async function getConfig() {
  if (!inTauri) {
    mock.resize();
    return structuredClone({ ...mock.config, gains: mock.gains });
  }
  return (await tauri()).invoke("get_config");
}

export async function saveConfig(config) {
  if (!inTauri) return mock.setConfig(config);
  return (await tauri()).invoke("save_config", { config });
}

export async function setGain(input, output, db) {
  if (!inTauri) return mock.setGain(input, output, db);
  return (await tauri()).invoke("set_gain", { input, output, db });
}

export async function setMuted(input, output, muted) {
  if (!inTauri) return mock.setMuted(input, output, muted);
  return (await tauri()).invoke("set_muted", { input, output, muted });
}

/** Master fader of a whole output. */
export async function setMasterGain(output, db) {
  if (!inTauri) return mock.setMaster(output, { db });
  return (await tauri()).invoke("set_master_gain", { output, db });
}

export async function setMasterMuted(output, muted) {
  if (!inTauri) return mock.setMaster(output, { muted });
  return (await tauri()).invoke("set_master_muted", { output, muted });
}

/** Mutes a whole input, its routing stays. */
export async function setInputMuted(input, muted) {
  if (!inTauri) return void (mock.config.inputs[input].muted = muted);
  return (await tauri()).invoke("set_input_muted", { input, muted });
}

/** Changes an input's effects live. */
export async function setFx(input, fx) {
  if (!inTauri) return void (mock.config.inputs[input].fx = { ...fx });
  return (await tauri()).invoke("set_fx", { input, fx });
}

export async function savePreset(name) {
  if (!inTauri) return mock.savePreset(name);
  return (await tauri()).invoke("save_preset", { name });
}

export async function loadPreset(name) {
  if (!inTauri) return mock.loadPreset(name);
  return (await tauri()).invoke("load_preset", { name });
}

export async function deletePreset(name) {
  if (!inTauri)
    return void (mock.config.presets = mock.config.presets.filter(
      (p) => p.name !== name,
    ));
  return (await tauri()).invoke("delete_preset", { name });
}

/** Calls `cb({ level, text })` for device changes and errors. */
export async function onNotice(cb) {
  if (!inTauri) return mock.subscribeNotices(cb);
  return (await tauri()).listen("notice", (e) => cb(e.payload));
}

/** Calls `cb(state)` about 20 times a second. Returns an unsubscribe function. */
export async function onState(cb) {
  if (!inTauri) return mock.subscribe(cb);
  const unlisten = await (await tauri()).listen("state", (e) => cb(e.payload));
  return unlisten;
}

/** Effects that change nothing; mirrors `FxSettings::default()`. */
export function defaultFx() {
  return {
    lowCutHz: 0,
    lowDb: 0,
    midDb: 0,
    highDb: 0,
    gate: false,
    gateDb: -45,
  };
}

/** True if any effect is switched on. */
export function fxActive(fx) {
  return (
    !!fx &&
    (fx.lowCutHz > 0 ||
      fx.lowDb !== 0 ||
      fx.midDb !== 0 ||
      fx.highDb !== 0 ||
      fx.gate)
  );
}

const mock = {
  devices: {
    inputs: [
      { id: "mic", name: "Mikrofon (USB Audio Device)" },
      { id: "cable", name: "CABLE Output (VB-Audio Virtual Cable)" },
      { id: "ps5", name: "Line In (USB Sound Card)" },
      { id: "cablea", name: "CABLE-A Output (VB-Audio Cable A)" },
    ],
    outputs: [
      { id: "ps5out", name: "Lautsprecher (USB Sound Card)" },
      { id: "buds", name: "Kopfhörer (Galaxy Buds2 Pro)" },
      { id: "jbl", name: "Lautsprecher (JBL Flip 6)" },
      { id: "cablein", name: "CABLE Input (VB-Audio Virtual Cable)" },
      { id: "cableain", name: "CABLE-A Input (VB-Audio Cable A)" },
    ],
  },
  config: {
    inputs: [
      {
        label: "Mikro",
        device: "Mikrofon",
        fx: {
          lowCutHz: 80,
          lowDb: 0,
          midDb: 2,
          highDb: 0,
          gate: true,
          gateDb: -45,
        },
      },
      { label: "PC", device: "CABLE Output", fx: defaultFx() },
      { label: "PS5", device: "Line In", fx: defaultFx() },
    ],
    outputs: [
      {
        label: "PS5",
        device: "USB Sound Card",
        bufferMs: 30,
      },
      {
        label: "Kopfhörer",
        device: "Galaxy Buds",
        bufferMs: 80,
      },
      { label: "Box", device: "JBL", bufferMs: 80 },
    ],
    presets: [{ name: "Zocken", gains: [] }],
  },
  gains: [],
  autostart: false,
  apps: {
    supported: true,
    devices: [
      { id: "ps5out", name: "Lautsprecher (USB Sound Card)", cable: false },
      {
        id: "cable",
        name: "CABLE Input (VB-Audio Virtual Cable)",
        cable: true,
      },
      { id: "buds", name: "Kopfhörer (Galaxy Buds2 Pro)", cable: false },
      {
        id: "cablea",
        name: "CABLE-A Input (VB-Audio Cable A)",
        cable: true,
      },
    ],
    apps: [
      {
        name: "Discord",
        exe: "Discord.exe",
        pids: [4120, 4188],
        device: null,
        volume: 0.8,
        muted: false,
      },
      {
        name: "firefox",
        exe: "firefox.exe",
        pids: [9932],
        device: "cable",
        volume: 1,
        muted: false,
      },
      {
        name: "Spotify",
        exe: "Spotify.exe",
        pids: [7311],
        device: "cable",
        volume: 0.45,
        muted: true,
      },
    ],
  },
  windows: {
    supported: true,
    playback: [
      {
        id: "ps5out",
        name: "Lautsprecher (USB Sound Card)",
        isDefault: false,
        isDefaultCommunications: false,
        volume: 1,
        muted: false,
      },
      {
        id: "cable",
        name: "CABLE Input (VB-Audio Virtual Cable)",
        isDefault: true,
        isDefaultCommunications: false,
        volume: 1,
        muted: false,
      },
      {
        id: "buds",
        name: "Kopfhörer (Galaxy Buds2 Pro)",
        isDefault: false,
        isDefaultCommunications: true,
        volume: 0.6,
        muted: false,
      },
    ],
    recording: [
      {
        id: "mic",
        name: "Mikrofon (USB Audio Device)",
        isDefault: true,
        isDefaultCommunications: true,
        volume: 0.75,
        muted: false,
      },
      {
        id: "cableout",
        name: "CABLE Output (VB-Audio Virtual Cable)",
        isDefault: false,
        isDefaultCommunications: false,
        volume: 1,
        muted: false,
      },
      {
        id: "cableaout",
        name: "CABLE-A Output (VB-Audio Cable A)",
        isDefault: false,
        isDefaultCommunications: false,
        volume: 1,
        muted: false,
      },
    ],
  },
  appLevels() {
    const t = Date.now() / 1000;
    return this.apps.apps.flatMap((a, k) =>
      a.pids.map((pid) => ({
        pid,
        db: a.muted
          ? -120
          : -24 +
            14 * Math.sin(t * (1.1 + k) + k) +
            20 * Math.log10(a.volume || 1e-6),
      })),
    );
  },
  setAppVolume(pids, volume, muted) {
    const app = this.apps.apps.find((a) => a.pids[0] === pids[0]);
    if (app && volume != null) app.volume = volume;
    if (app && muted != null) app.muted = muted;
  },
  setDefault(id, communications) {
    const w = this.windows;
    const list = w.playback.some((d) => d.id === id) ? w.playback : w.recording;
    const key = communications ? "isDefaultCommunications" : "isDefault";
    for (const d of list) d[key] = d.id === id;
  },
  setDeviceVolume(id, volume, muted) {
    const d = [...this.windows.playback, ...this.windows.recording].find(
      (d) => d.id === id,
    );
    if (d && volume != null) d.volume = volume;
    if (d && muted != null) d.muted = muted;
  },
  setAppOutput(pids, device) {
    const app = this.apps.apps.find((a) => a.pids[0] === pids[0]);
    if (app) app.device = device;
  },
  savePreset(name) {
    this.resize();
    const preset = { name, gains: structuredClone(this.gains) };
    const i = this.config.presets.findIndex((p) => p.name === name);
    if (i >= 0) this.config.presets[i] = preset;
    else this.config.presets.push(preset);
  },
  loadPreset(name) {
    const p = this.config.presets.find((p) => p.name === name);
    if (!p) throw new Error(`Preset „${name}“ gibt es nicht`);
    this.gains = structuredClone(p.gains);
    this.resize();
  },
  subscribeNotices(cb) {
    const id = setTimeout(
      () =>
        cb({
          level: "warn",
          text: "„Box“ getrennt. Faderust verbindet neu, sobald das Gerät wieder da ist.",
        }),
      1500,
    );
    return () => clearTimeout(id);
  },
  setConfig(config) {
    this.config = structuredClone(config);
    if (config.gains) this.gains = structuredClone(config.gains);
    this.resize();
  },
  resize() {
    const { inputs, outputs } = this.config;
    this.gains = inputs.map((_, i) =>
      outputs.map(
        (_, o) => this.gains[i]?.[o] ?? { db: i === o ? 0 : -12, muted: false },
      ),
    );
  },
  setGain(i, o, db) {
    this.gains[i][o].db = db;
  },
  setMuted(i, o, muted) {
    this.gains[i][o].muted = muted;
  },
  setMaster(o, patch) {
    const out = this.config.outputs[o];
    out.master = { db: 0, muted: false, ...out.master, ...patch };
  },
  subscribe(cb) {
    this.resize();
    let t = 0;
    const id = setInterval(() => {
      t += 0.05;
      const wave = (k) =>
        -30 + 18 * Math.sin(t * (1.3 + k) + k) + 6 * Math.random();
      const { inputs, outputs } = this.config;
      const inLevels = inputs.map((_, i) => wave(i));
      cb({
        inputs: inputs.map((c, i) => ({
          ...c,
          connected: true,
          deviceName: this.devices.inputs[i % 3].name,
          levelDb: inLevels[i],
        })),
        outputs: outputs.map((c, o) => ({
          ...c,
          master: c.master ?? { db: 0, muted: false },
          // The second Bluetooth device is released now and then.
          connected: !(o === 2 && Math.sin(t / 4) > 0.6),
          released: o === 2 && Math.sin(t / 4) > 0.6,
          deviceName: (
            this.devices.outputs.find((d) => d.name.includes(c.device)) ??
            this.devices.outputs[o % 3]
          ).name,
          levelDb: Math.max(
            ...inLevels.map((l, i) =>
              this.gains[i][o].muted ? -120 : l + this.gains[i][o].db,
            ),
          ),
        })),
        gains: structuredClone(this.gains),
        lanes: inputs.map((input) =>
          outputs.map((output, o) => ({
            fillMs: o ? 80 : 30,
            correctionPpm: -40,
            underruns: 0,
            // The mock has patterns, not device names.
            looped:
              cableCounterpart(output.device)
                .toLowerCase()
                .includes(input.device.toLowerCase()) &&
              /input/i.test(output.device),
          })),
        ),
      });
    }, 50);
    return () => clearInterval(id);
  },
};
