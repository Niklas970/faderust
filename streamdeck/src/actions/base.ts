import streamDeck, {
  type Action,
  type DidReceiveSettingsEvent,
  type SendToPluginEvent,
  SingletonAction,
  type WillAppearEvent,
  type WillDisappearEvent,
} from "@elgato/streamdeck";

import { faderust } from "../connection";

export type JsonValue = string | number | boolean | null | JsonValue[] | { [key: string]: JsonValue };
export type JsonObject = { [key: string]: JsonValue };

/**
 * Keeps each visible action's settings and redraws it whenever Faderust's
 * state changes, without resending what Stream Deck already shows.
 */
export abstract class FaderustAction<T extends JsonObject> extends SingletonAction<T> {
  readonly #settings = new Map<string, T>();
  readonly #shown = new Map<string, string>();

  constructor() {
    super();
    faderust.on("change", () => void this.#redrawAll());
  }

  /** Draws one action; use `show` to skip unchanged output. */
  protected abstract draw(action: Action<T>, settings: T): Promise<void>;

  protected settingsOf(action: Action<T>): T {
    return this.#settings.get(action.id) ?? ({} as T);
  }

  /** Runs `paint` unless `view` is what the action already shows. */
  protected async show(action: Action<T>, view: JsonValue, paint: () => Promise<void>): Promise<void> {
    const key = JSON.stringify(view);
    if (this.#shown.get(action.id) === key) return;
    this.#shown.set(action.id, key);
    await paint();
  }

  /** Key press while Faderust is not running. */
  protected async offline(action: Action<T>): Promise<void> {
    if (action.isKey() || action.isDial()) await action.showAlert();
  }

  override async onWillAppear(ev: WillAppearEvent<T>): Promise<void> {
    this.#settings.set(ev.action.id, ev.payload.settings);
    this.#shown.delete(ev.action.id);
    await this.draw(ev.action, ev.payload.settings);
  }

  override onWillDisappear(ev: WillDisappearEvent<T>): void {
    this.#settings.delete(ev.action.id);
    this.#shown.delete(ev.action.id);
  }

  override async onDidReceiveSettings(ev: DidReceiveSettingsEvent<T>): Promise<void> {
    this.#settings.set(ev.action.id, ev.payload.settings);
    this.#shown.delete(ev.action.id);
    await this.draw(ev.action, ev.payload.settings);
  }

  // The panel asks for the lists when it loads, which can be before Stream
  // Deck reports it open; answer both.
  override async onPropertyInspectorDidAppear(): Promise<void> {
    await sendOptions(true);
  }

  override async onSendToPlugin(ev: SendToPluginEvent<JsonValue, T>): Promise<void> {
    const payload = ev.payload as { event?: string } | null;
    if (payload?.event === "getOptions") await sendOptions(true);
  }

  async #redrawAll(): Promise<void> {
    for (const action of this.actions) {
      const settings = this.#settings.get(action.id);
      if (settings) await this.draw(action, settings);
    }
    // Keep an open settings panel's lists current.
    if (streamDeck.ui.action) await sendOptions();
  }
}

let lastOptions = "";

/** The lists the settings panels offer (ui/*.html). */
export async function sendOptions(force = false): Promise<void> {
  const s = faderust.state;
  const d = faderust.devices;
  const list = (names: string[] = []) => names.map((label, value) => ({ value, label }));
  const endpoints = (kind: string, list: { id: string; name: string }[] = []) =>
    list.map((e) => ({ value: e.id, label: `${kind}: ${e.name}` }));
  const options = {
    event: "options",
    connected: faderust.connected,
    inputs: list(s?.inputs),
    outputs: list(s?.outputs),
    presets: (s?.presets ?? []).map((p) => ({ value: p, label: p })),
    devices: [...endpoints("Wiedergabe", d?.playback), ...endpoints("Aufnahme", d?.recording)],
  };
  const text = JSON.stringify(options);
  // Gains change all the time while a fader moves; the lists rarely do.
  if (!force && text === lastOptions) return;
  lastOptions = text;
  await streamDeck.ui.sendToPropertyInspector(options);
}
