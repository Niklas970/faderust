/**
 * Connection to the Faderust app: a WebSocket on 127.0.0.1 (see
 * crates/app/src/remote.rs). Reconnects on its own while Faderust is closed.
 */
import { EventEmitter } from "node:events";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import WebSocket from "ws";

export const PORT = 47811;
const RETRY_MS = 2000;

/**
 * The secret Faderust writes to its config folder, which only programs of
 * this Windows user can read. `FADERUST_TOKEN_FILE` is for tests.
 */
export function readToken(): string {
  const file =
    process.env.FADERUST_TOKEN_FILE ||
    join(process.env.APPDATA ?? "", "io.github.niklas970.faderust", "remote.token");
  try {
    return readFileSync(file, "utf8").trim();
  } catch {
    return "";
  }
}

export type Gain = { db: number; muted: boolean };

export type State = {
  protocol: number;
  inputs: string[];
  outputs: string[];
  /** `gains[input][output]`. */
  gains: Gain[][];
  presets: string[];
  activePreset: string | null;
};

export type Endpoint = {
  id: string;
  name: string;
  isDefault: boolean;
  isDefaultCommunications: boolean;
  volume: number;
  muted: boolean;
};

export type Devices = {
  supported: boolean;
  playback: Endpoint[];
  recording: Endpoint[];
};

export type Request =
  | { type: "setGain"; input: number; output: number; db: number }
  | { type: "addGain"; input: number; output: number; delta: number }
  | { type: "setMuted"; input: number; output: number; muted: boolean }
  | { type: "toggleMuted"; input: number; output: number }
  | { type: "loadPreset"; name: string }
  | { type: "setDefaultDevice"; id: string; communications: boolean };

type Push =
  | ({ type: "state" } & State)
  | ({ type: "devices" } & Devices)
  | { type: "error"; message: string };

export class Faderust extends EventEmitter<{ change: []; error: [string] }> {
  state: State | null = null;
  devices: Devices | null = null;
  #ws: WebSocket | null = null;
  #timer: NodeJS.Timeout | null = null;
  readonly url: string;
  /** Plugin version, told to Faderust on connecting so the app can offer updates. */
  version: string | null = null;

  constructor(url = `ws://127.0.0.1:${PORT}`) {
    super();
    this.url = url;
  }

  get connected(): boolean {
    return this.#ws?.readyState === WebSocket.OPEN && this.state !== null;
  }

  connect(): void {
    const ws = new WebSocket(this.url);
    this.#ws = ws;
    ws.on("open", () => {
      // Faderust takes nothing from a client that has not said hello first.
      ws.send(JSON.stringify({ type: "hello", plugin: this.version ?? "", token: readToken() }));
    });
    ws.on("message", (data) => this.#receive(data.toString()));
    ws.on("close", () => {
      if (this.#ws !== ws) return;
      this.#ws = null;
      const had = this.state !== null;
      this.state = null;
      this.devices = null;
      if (had) this.emit("change");
      this.#timer = setTimeout(() => this.connect(), RETRY_MS);
    });
    // "close" follows and schedules the retry.
    ws.on("error", () => {});
  }

  close(): void {
    if (this.#timer) clearTimeout(this.#timer);
    const ws = this.#ws;
    this.#ws = null;
    ws?.close();
  }

  /** False when Faderust is not running. */
  send(req: Request): boolean {
    if (!this.connected) return false;
    this.#ws!.send(JSON.stringify(req));
    return true;
  }

  #receive(text: string): void {
    let msg: Push;
    try {
      msg = JSON.parse(text);
    } catch {
      return;
    }
    switch (msg.type) {
      case "state": {
        const { type: _, ...state } = msg;
        this.state = state;
        this.emit("change");
        break;
      }
      case "devices": {
        const { type: _, ...devices } = msg;
        this.devices = devices;
        this.emit("change");
        break;
      }
      case "error":
        this.emit("error", msg.message);
        break;
    }
  }
}

/** Fader text like the app shows it. */
export function formatDb(gain: Gain | undefined): string {
  if (!gain) return "–";
  if (gain.muted) return "Stumm";
  if (gain.db <= -60) return "−∞";
  const db = Math.round(gain.db * 2) / 2;
  return `${db > 0 ? "+" : db < 0 ? "−" : ""}${Math.abs(db).toFixed(1)} dB`;
}

/** Fader travel 0…100 with the app's taper (crates/app/ui/src/lib/Fader.svelte). */
export function faderPercent(db: number): number {
  let p: number;
  if (db <= -60) p = 0;
  else if (db < -24) p = 0.05 + ((db + 60) / 36) * 0.2;
  else p = 0.25 + ((Math.min(db, 12) + 24) / 36) * 0.75;
  return Math.round(p * 100);
}
