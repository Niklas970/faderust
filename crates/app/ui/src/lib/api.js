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

/** Calls `cb(state)` about 20 times a second. Returns an unsubscribe function. */
export async function onState(cb) {
  if (!inTauri) return mock.subscribe(cb);
  const unlisten = await (await tauri()).listen("state", (e) => cb(e.payload));
  return unlisten;
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
      { label: "Mikro", device: "Mikrofon" },
      { label: "PC", device: "CABLE Output" },
      { label: "PS5", device: "Line In" },
    ],
    outputs: [
      { label: "PS5", device: "USB Sound Card", bufferMs: 30 },
      { label: "Kopfhörer", device: "Galaxy Buds", bufferMs: 80 },
      { label: "Box", device: "JBL", bufferMs: 80 },
    ],
  },
  gains: [],
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
