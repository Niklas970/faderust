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
  output?: number;
  /** dB per dial click or key press; keys may use a negative step. */
  step?: number;
};

const DIAL_STEP = 1;
const KEY_STEP = 3;

/** The master fader of an output: dial turns it, press mutes; keys step it. */
@action({ UUID: "com.niklas970.faderust.master" })
export class Master extends FaderustAction<Settings> {
  protected override async draw(action: Action<Settings>, s: Settings): Promise<void> {
    const state = faderust.state;
    const o = s.output ?? 0;
    const name = state?.outputs[o] ?? "Faderust";
    const gain = state?.masters?.[o];
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
      const title = gain ? `${name}\n${value}` : value.replace(" ", "\n");
      await this.show(action, title, () => action.setTitle(title));
    }
  }

  override async onDialRotate(ev: DialRotateEvent<Settings>): Promise<void> {
    const s = this.settingsOf(ev.action);
    const delta = ev.payload.ticks * Math.abs(s.step ?? DIAL_STEP);
    if (!faderust.send({ type: "addMasterGain", output: s.output ?? 0, delta })) await this.offline(ev.action);
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
    if (!faderust.send({ type: "addMasterGain", output: s.output ?? 0, delta })) await this.offline(ev.action);
  }

  async #toggleMute(action: Action<Settings>): Promise<void> {
    const output = this.settingsOf(action).output ?? 0;
    const muted = faderust.state?.masters?.[output]?.muted;
    if (muted === undefined || !faderust.send({ type: "setMasterMuted", output, muted: !muted })) {
      await this.offline(action);
    }
  }
}
