// Bridge to the Rust side. Outside Tauri (plain `npm run dev` in a browser)
// a small simulation stands in, so the UI can be worked on without devices.

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function tauri() {
  const [core, event] = await Promise.all([
    import("@tauri-apps/api/core"),
    import("@tauri-apps/api/event"),
  ]);
  return { invoke: core.invoke, listen: event.listen };
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

/** VB-Audio's virtual cables, by name ("CABLE Input", "CABLE-A Output", …). */
export function isCable(name) {
  const n = name.toLowerCase();
  return n.includes("vb-audio") || n.startsWith("cable");
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
  if (!inTauri) return void (mock.config.presets = mock.config.presets.filter((p) => p.name !== name));
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
  return { lowCutHz: 0, lowDb: 0, midDb: 0, highDb: 0, gate: false, gateDb: -45 };
}

/** True if any effect is switched on. */
export function fxActive(fx) {
  return !!fx && (fx.lowCutHz > 0 || fx.lowDb !== 0 || fx.midDb !== 0 || fx.highDb !== 0 || fx.gate);
}

const mock = {
  devices: {
    inputs: [
      { id: "mic", name: "Mikrofon (USB Audio Device)" },
      { id: "cable", name: "CABLE Output (VB-Audio Virtual Cable)" },
      { id: "ps5", name: "Line In (USB Sound Card)" },
    ],
    outputs: [
      { id: "ps5out", name: "Lautsprecher (USB Sound Card)" },
      { id: "buds", name: "Kopfhörer (Galaxy Buds2 Pro)" },
      { id: "jbl", name: "Lautsprecher (JBL Flip 6)" },
    ],
  },
  config: {
    inputs: [
      { label: "Mikro", device: "Mikrofon", fx: { lowCutHz: 80, lowDb: 0, midDb: 2, highDb: 0, gate: true, gateDb: -45 } },
      { label: "PC", device: "CABLE Output", fx: defaultFx() },
      { label: "PS5", device: "Line In", fx: defaultFx() },
    ],
    outputs: [
      { label: "PS5", device: "USB Sound Card", bufferMs: 30 },
      { label: "Kopfhörer", device: "Galaxy Buds", bufferMs: 80 },
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
      { id: "cable", name: "CABLE Input (VB-Audio Virtual Cable)", cable: true },
      { id: "buds", name: "Kopfhörer (Galaxy Buds2 Pro)", cable: false },
    ],
    apps: [
      { name: "Discord", exe: "Discord.exe", pids: [4120, 4188], device: null },
      { name: "firefox", exe: "firefox.exe", pids: [9932], device: "cable" },
      { name: "Spotify", exe: "Spotify.exe", pids: [7311], device: "cable" },
    ],
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
      () => cb({ level: "warn", text: "„Box“ getrennt. Faderust verbindet neu, sobald das Gerät wieder da ist." }),
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
      outputs.map((_, o) => this.gains[i]?.[o] ?? { db: i === o ? 0 : -12, muted: false }),
    );
  },
  setGain(i, o, db) {
    this.gains[i][o].db = db;
  },
  setMuted(i, o, muted) {
    this.gains[i][o].muted = muted;
  },
  subscribe(cb) {
    this.resize();
    let t = 0;
    const id = setInterval(() => {
      t += 0.05;
      const wave = (k) => -30 + 18 * Math.sin(t * (1.3 + k) + k) + 6 * Math.random();
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
          // The second Bluetooth device drops out now and then.
          connected: !(o === 2 && Math.sin(t / 4) > 0.6),
          deviceName: this.devices.outputs[o % 3].name,
          levelDb: Math.max(...inLevels.map((l, i) => (this.gains[i][o].muted ? -120 : l + this.gains[i][o].db))),
        })),
        gains: structuredClone(this.gains),
        lanes: inputs.map(() => outputs.map((_, o) => ({ fillMs: o ? 80 : 30, correctionPpm: -40, underruns: 0 }))),
      });
    }, 50);
    return () => clearInterval(id);
  },
};
