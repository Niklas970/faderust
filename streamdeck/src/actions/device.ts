import { type Action, action, type KeyDownEvent } from "@elgato/streamdeck";

import { faderust } from "../connection";
import type { Endpoint } from "../faderust";
import { FaderustAction } from "./base";

type Settings = {
  /** Windows device id. */
  id?: string;
  /** Only the default for calls (Discord, Teams …). */
  communications?: boolean;
};

/** Makes a device the Windows default; state 1 while it is. */
@action({ UUID: "com.niklas970.faderust.device" })
export class Device extends FaderustAction<Settings> {
  #find(s: Settings): Endpoint | undefined {
    const d = faderust.devices;
    return [...(d?.playback ?? []), ...(d?.recording ?? [])].find((e) => e.id === s.id);
  }

  protected override async draw(action: Action<Settings>, s: Settings): Promise<void> {
    if (!action.isKey()) return;
    const e = this.#find(s);
    const active = !!e && (s.communications ? e.isDefaultCommunications : e.isDefault);
    await this.show(action, active, () => action.setState(active ? 1 : 0));
  }

  override async onKeyDown(ev: KeyDownEvent<Settings>): Promise<void> {
    const s = this.settingsOf(ev.action);
    if (!s.id || !this.#find(s)) return this.offline(ev.action);
    faderust.send({ type: "setDefaultDevice", id: s.id, communications: !!s.communications });
  }
}
