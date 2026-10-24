import { type Action, action, type KeyDownEvent } from "@elgato/streamdeck";

import { faderust } from "../connection";
import { FaderustAction } from "./base";

type Settings = {
  input?: number;
  /** -1: the input on every output, e.g. the microphone everywhere. */
  output?: number;
};

const ALL = -1;

/** Mute key with the current state on it (state 1 = muted). */
@action({ UUID: "com.niklas970.faderust.mute" })
export class Mute extends FaderustAction<Settings> {
  /** The outputs the key covers. */
  #outputs(s: Settings): number[] {
    const count = faderust.state?.outputs.length ?? 0;
    const output = s.output ?? 0;
    if (output === ALL) return [...Array(count).keys()];
    return output < count ? [output] : [];
  }

  #muted(s: Settings): boolean | undefined {
    const row = faderust.state?.gains[s.input ?? 0];
    const outputs = this.#outputs(s);
    if (!row || outputs.length === 0) return undefined;
    return outputs.every((o) => row[o]?.muted);
  }

  protected override async draw(action: Action<Settings>, s: Settings): Promise<void> {
    if (!action.isKey()) return;
    const muted = this.#muted(s);
    await this.show(action, muted ?? null, () => action.setState(muted ? 1 : 0));
  }

  override async onKeyDown(ev: KeyDownEvent<Settings>): Promise<void> {
    const s = this.settingsOf(ev.action);
    const muted = this.#muted(s);
    if (muted === undefined || !faderust.connected) return this.offline(ev.action);
    for (const output of this.#outputs(s)) {
      faderust.send({ type: "setMuted", input: s.input ?? 0, output, muted: !muted });
    }
  }
}
