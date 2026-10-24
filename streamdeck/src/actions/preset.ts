import { type Action, action, type KeyDownEvent } from "@elgato/streamdeck";

import { faderust } from "../connection";
import { FaderustAction } from "./base";

type Settings = { name?: string };

/** Loads a preset; state 1 while the faders match it. */
@action({ UUID: "com.niklas970.faderust.preset" })
export class Preset extends FaderustAction<Settings> {
  protected override async draw(action: Action<Settings>, s: Settings): Promise<void> {
    if (!action.isKey()) return;
    const active = !!s.name && faderust.state?.activePreset === s.name;
    await this.show(action, active, () => action.setState(active ? 1 : 0));
  }

  override async onKeyDown(ev: KeyDownEvent<Settings>): Promise<void> {
    const { name } = this.settingsOf(ev.action);
    if (!name || !faderust.state?.presets.includes(name)) return this.offline(ev.action);
    if (!faderust.send({ type: "loadPreset", name })) return this.offline(ev.action);
  }
}
