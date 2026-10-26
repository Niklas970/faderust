// Runs the built plugin (bin/plugin.js) between a fake Stream Deck and a
// fake Faderust and checks what flows both ways. Build first: npm run build.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { after, before, test } from "node:test";
import { fileURLToPath } from "node:url";
import { WebSocketServer } from "ws";

const plugin = fileURLToPath(new URL("../com.niklas970.faderust.sdPlugin/bin/plugin.js", import.meta.url));
const UUID = "com.niklas970.faderust";
const TOKEN = "ab".repeat(32);

/** Collects JSON messages and lets a test wait for one that matches. */
class Peer {
  messages = [];
  waiters = [];
  socket = null;
  constructor(server) {
    server.on("connection", (socket, req) => {
      this.socket = socket;
      this.headers = req.headers;
      socket.on("message", (data) => {
        const msg = JSON.parse(data.toString());
        this.messages.push(msg);
        this.waiters = this.waiters.filter((w) => !(w.match(msg) && (w.resolve(msg), true)));
      });
      this.onConnect?.(socket);
    });
  }
  next(match, ms = 5000) {
    const found = this.messages.find(match);
    if (found) {
      this.messages.splice(this.messages.indexOf(found), 1);
      return Promise.resolve(found);
    }
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error(`timeout waiting for ${match}`)), ms);
      this.waiters.push({
        match,
        resolve: (msg) => {
          clearTimeout(timer);
          this.messages.splice(this.messages.indexOf(msg), 1);
          resolve(msg);
        },
      });
    });
  }
  send(msg) {
    this.socket.send(JSON.stringify(msg));
  }
}

const state = {
  type: "state",
  protocol: 3,
  inputs: ["Mikro", "PC"],
  outputs: ["PS5", "Kopfhörer"],
  gains: [
    [{ db: 0, muted: false }, { db: -6, muted: false }],
    [{ db: -12, muted: true }, { db: 0, muted: true }],
  ],
  inputMuted: [false, true],
  masters: [{ db: 0, muted: false }, { db: -3, muted: true }],
  presets: ["Spielen", "Chat"],
  activePreset: "Chat",
};
const devices = {
  type: "devices",
  supported: true,
  playback: [{ id: "{a}", name: "Lautsprecher", isDefault: true, isDefaultCommunications: false, volume: 1, muted: false }],
  recording: [],
};

let sd, fr, sdServer, frServer, child;

before(async () => {
  sdServer = new WebSocketServer({ port: 0, host: "127.0.0.1" });
  frServer = new WebSocketServer({ port: 0, host: "127.0.0.1" });
  await Promise.all([once(sdServer, "listening"), once(frServer, "listening")]);
  sd = new Peer(sdServer);
  fr = new Peer(frServer);
  fr.onConnect = (socket) => {
    socket.send(JSON.stringify(state));
    socket.send(JSON.stringify(devices));
  };
  const tokenFile = join(mkdtempSync(join(tmpdir(), "faderust-")), "remote.token");
  writeFileSync(tokenFile, `${TOKEN}\n`);
  const info = {
    application: { font: "", language: "de", platform: "windows", platformVersion: "10", version: "7.1.0.0" },
    colors: {},
    devicePixelRatio: 1,
    devices: [{ id: "dev", name: "Stream Deck +", size: { columns: 4, rows: 2 }, type: 7 }],
    plugin: { uuid: UUID, version: "0.3.0.0" },
  };
  child = spawn(
    process.execPath,
    [plugin, "-port", String(sdServer.address().port), "-pluginUUID", UUID, "-registerEvent", "registerPlugin", "-info", JSON.stringify(info)],
    {
      cwd: fileURLToPath(new URL("../com.niklas970.faderust.sdPlugin/", import.meta.url)),
      env: { ...process.env, FADERUST_URL: `ws://127.0.0.1:${frServer.address().port}`, FADERUST_TOKEN_FILE: tokenFile },
      stdio: "inherit",
    },
  );
  await sd.next((m) => m.event === "registerPlugin");
  // Give the plugin a moment to receive the state.
  await new Promise((r) => setTimeout(r, 300));
});

after(() => {
  child?.kill();
  sdServer?.close();
  frServer?.close();
});

const appear = (action, context, settings, controller = "Keypad") =>
  sd.send({
    event: "willAppear",
    action: `${UUID}.${action}`,
    context,
    device: "dev",
    payload: { settings, coordinates: { column: 0, row: 0 }, controller, isInMultiAction: false },
  });

const event = (name, action, context, settings, extra = {}, controller = "Keypad") =>
  sd.send({
    event: name,
    action: `${UUID}.${action}`,
    context,
    device: "dev",
    payload: { settings, coordinates: { column: 0, row: 0 }, controller, ...extra },
  });

test("connects to Faderust like a desktop program, without an Origin", () => {
  assert.equal(fr.headers.origin, undefined);
});

test("says hello with its version and the token from the config folder", async () => {
  assert.deepEqual(await fr.next((m) => m.type === "hello"), { type: "hello", plugin: "0.3.0.0", token: TOKEN });
});

test("dial shows the fader and turns it", async () => {
  const settings = { input: 0, output: 1 };
  appear("volume", "dial", settings, "Encoder");
  const fb = await sd.next((m) => m.event === "setFeedback" && m.context === "dial");
  assert.equal(fb.payload.title, "Mikro → Kopfhörer");
  assert.equal(fb.payload.value, "−6.0 dB");

  event("dialRotate", "volume", "dial", settings, { ticks: -2, pressed: false }, "Encoder");
  assert.deepEqual(await fr.next((m) => m.type === "addGain"), { type: "addGain", input: 0, output: 1, delta: -2 });

  event("dialDown", "volume", "dial", settings, {}, "Encoder");
  assert.deepEqual(await fr.next((m) => m.type === "toggleMuted"), { type: "toggleMuted", input: 0, output: 1 });

  // A new state from Faderust redraws the dial.
  const next = structuredClone(state);
  next.gains[0][1] = { db: -8, muted: false };
  fr.send(next);
  const redraw = await sd.next((m) => m.event === "setFeedback" && m.context === "dial");
  assert.equal(redraw.payload.value, "−8.0 dB");
});

test("volume key steps by its setting", async () => {
  const settings = { input: 1, output: 0, step: -2 };
  appear("volume", "vkey", settings);
  const title = await sd.next((m) => m.event === "setTitle" && m.context === "vkey");
  assert.equal(title.payload.title, "PC\nStumm");
  event("keyDown", "volume", "vkey", settings);
  assert.deepEqual(await fr.next((m) => m.type === "addGain"), { type: "addGain", input: 1, output: 0, delta: -2 });
});

test("mute key on all outputs shows and flips the state", async () => {
  const settings = { input: 1, output: -1 };
  appear("mute", "mute", settings);
  const st = await sd.next((m) => m.event === "setState" && m.context === "mute");
  assert.equal(st.payload.state, 1);
  event("keyDown", "mute", "mute", settings);
  const a = await fr.next((m) => m.type === "setMuted" && m.output === 0);
  const b = await fr.next((m) => m.type === "setMuted" && m.output === 1);
  assert.equal(a.muted, false);
  assert.equal(b.muted, false);
});

test("strip dial moves the whole strip and mutes the input", async () => {
  const settings = { input: 0 };
  appear("strip", "strip", settings, "Encoder");
  const fb = await sd.next((m) => m.event === "setFeedback" && m.context === "strip");
  assert.equal(fb.payload.title, "Mikro");
  assert.equal(fb.payload.value, "0.0 dB");
  event("dialRotate", "strip", "strip", settings, { ticks: 3, pressed: false }, "Encoder");
  assert.deepEqual(await fr.next((m) => m.type === "addStripGain"), { type: "addStripGain", input: 0, delta: 3 });
  event("dialDown", "strip", "strip", settings, {}, "Encoder");
  assert.deepEqual(await fr.next((m) => m.type === "setInputMuted"), { type: "setInputMuted", input: 0, muted: true });
});

test("master dial shows the output and mutes it", async () => {
  const settings = { output: 1 };
  appear("master", "master", settings, "Encoder");
  const fb = await sd.next((m) => m.event === "setFeedback" && m.context === "master");
  assert.equal(fb.payload.title, "Kopfhörer");
  assert.equal(fb.payload.value, "Stumm");
  event("dialRotate", "master", "master", settings, { ticks: -1, pressed: false }, "Encoder");
  assert.deepEqual(await fr.next((m) => m.type === "addMasterGain"), { type: "addMasterGain", output: 1, delta: -1 });
  event("dialDown", "master", "master", settings, {}, "Encoder");
  assert.deepEqual(await fr.next((m) => m.type === "setMasterMuted"), { type: "setMasterMuted", output: 1, muted: false });
});

test("mute key can be the strip's MUTE button", async () => {
  const settings = { input: 1, output: -2 };
  appear("mute", "smute", settings);
  assert.equal((await sd.next((m) => m.event === "setState" && m.context === "smute")).payload.state, 1);
  event("keyDown", "mute", "smute", settings);
  assert.deepEqual(await fr.next((m) => m.type === "setInputMuted"), { type: "setInputMuted", input: 1, muted: false });
});

test("preset key lights up for the active preset and loads", async () => {
  appear("preset", "p1", { name: "Chat" });
  assert.equal((await sd.next((m) => m.event === "setState" && m.context === "p1")).payload.state, 1);
  appear("preset", "p2", { name: "Spielen" });
  assert.equal((await sd.next((m) => m.event === "setState" && m.context === "p2")).payload.state, 0);
  event("keyDown", "preset", "p2", { name: "Spielen" });
  assert.deepEqual(await fr.next((m) => m.type === "loadPreset"), { type: "loadPreset", name: "Spielen" });
});

test("device key sets the Windows default", async () => {
  const settings = { id: "{a}", communications: true };
  appear("device", "d", settings);
  assert.equal((await sd.next((m) => m.event === "setState" && m.context === "d")).payload.state, 0);
  event("keyDown", "device", "d", settings);
  assert.deepEqual(await fr.next((m) => m.type === "setDefaultDevice"), {
    type: "setDefaultDevice",
    id: "{a}",
    communications: true,
  });
});

test("settings panel gets the lists", async () => {
  sd.send({ event: "propertyInspectorDidAppear", action: `${UUID}.mute`, context: "mute", device: "dev" });
  await sd.next((m) => m.event === "sendToPropertyInspector");
  sd.send({ event: "sendToPlugin", action: `${UUID}.mute`, context: "mute", payload: { event: "getOptions" } });
  const msg = await sd.next((m) => m.event === "sendToPropertyInspector");
  assert.deepEqual(msg.payload.inputs, [
    { value: 0, label: "Mikro" },
    { value: 1, label: "PC" },
  ]);
  assert.deepEqual(msg.payload.devices, [{ value: "{a}", label: "Wiedergabe: Lautsprecher" }]);
  assert.equal(msg.payload.connected, true);
});

test("keys alert while Faderust is closed", async () => {
  fr.socket.close();
  await new Promise((r) => setTimeout(r, 200));
  event("keyDown", "preset", "p2", { name: "Spielen" });
  assert.ok(await sd.next((m) => m.event === "showAlert" && m.context === "p2"));
});
