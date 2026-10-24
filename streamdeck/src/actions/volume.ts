import {
  type Action,
  action,
  type DialDownEvent,
  type DialRotateEvent,
  type KeyDownEvent,
  type TouchTapEvent,
} from "@elgato/streamdeck";

import { faderust } from "../connection";
import { faderPercent, formatDb } from "../faderust";
import { FaderustAction } from "./base";

type Settings = {
  input?: number;
  output?: number;
  /** dB per dial click or key press; keys may use a negative step. */
  step?: number;
};

const DIAL_STEP = 1;
const KEY_STEP = 3;

/** One fader (input on output): dial turns it, press mutes; keys step it. */
@action({ UUID: "com.niklas970.faderust.volume" })
export class Volume extends FaderustAction<Settings> {
  protected override async draw(action: Action<Settings>, s: Settings): Promise<void> {
    const state = faderust.state;
    const input = state?.inputs[s.input ?? 0];
    const output = state?.outputs[s.output ?? 0];
    const gain = state?.gains[s.input ?? 0]?.[s.output ?? 0];
    const name = input && output ? `${input} → ${output}` : "Faderust";
    const value = !state ? "nicht verbunden" : gain ? formatDb(gain) : "Fader fehlt";

    if (action.isDial()) {
      const view = { name, value, percent: gain ? faderPercent(gain.db) : 0, muted: !!gain?.muted };
      await this.show(action, view, () =>
        action.setFeedback({
          title: name,
          value,
          indicator: { value: view.percent, opacity: gain?.muted ? 0.4 : 1 },
        }),
      );
    } else if (action.isKey()) {
      const title = gain ? `${input}\n${value}` : value.replace(" ", "\n");
      await this.show(action, title, () => action.setTitle(title));
    }
  }

  override async onDialRotate(ev: DialRotateEvent<Settings>): Promise<void> {
    const s = this.settingsOf(ev.action);
    const delta = ev.payload.ticks * Math.abs(s.step ?? DIAL_STEP);
    if (!faderust.send({ type: "addGain", input: s.input ?? 0, output: s.output ?? 0, delta })) {
      await this.offline(ev.action);
    }
  }

  override async onDialDown(ev: DialDownEvent<Settings>): Promise<void> {
    await this.#toggleMute(ev.action);
  }

  override async onTouchTap(ev: TouchTapEvent<Settings>): Promise<void> {
    await this.#toggleMute(ev.action);
  }

  override async onKeyDown(ev: KeyDownEvent<Settings>): Promise<void> {
    const s = this.settingsOf(ev.action);
    const delta = s.step ?? KEY_STEP;
    if (!faderust.send({ type: "addGain", input: s.input ?? 0, output: s.output ?? 0, delta })) {
      await this.offline(ev.action);
    }
  }

  async #toggleMute(action: Action<Settings>): Promise<void> {
    const s = this.settingsOf(action);
    if (!faderust.send({ type: "toggleMuted", input: s.input ?? 0, output: s.output ?? 0 })) {
      await this.offline(action);
    }
  }
}
