import {
  type Action,
  action,
  type DialDownEvent,
  type DialRotateEvent,
  type KeyDownEvent,
  type TouchTapEvent,
} from "@elgato/streamdeck";

import { faderust } from "../connection";
import { faderPercent, formatDb, stripGain } from "../faderust";
import { FaderustAction } from "./base";

type Settings = {
  input?: number;
  /** dB per dial click or key press; keys may use a negative step. */
  step?: number;
};

const DIAL_STEP = 1;
const KEY_STEP = 3;

/** A desk strip like the app's Pult: one fader for every output the input plays on. Press mutes the input. */
@action({ UUID: "com.niklas970.faderust.strip" })
export class Strip extends FaderustAction<Settings> {
  protected override async draw(action: Action<Settings>, s: Settings): Promise<void> {
    const state = faderust.state;
    const i = s.input ?? 0;
    const name = state?.inputs[i] ?? "Faderust";
    const gain = stripGain(state?.gains[i]);
    const muted = !!state?.inputMuted?.[i];
    const value = !state ? "nicht verbunden" : muted ? "Stumm" : gain ? formatDb(gain) : "Fader fehlt";

    if (action.isDial()) {
      const view = { name, value, percent: gain ? faderPercent(gain.db) : 0, muted };
      await this.show(action, view, () =>
        action.setFeedback({
          title: name,
          value,
          indicator: { value: view.percent, opacity: muted ? 0.4 : 1 },
        }),
      );
    } else if (action.isKey()) {
      const title = state?.inputs[i] ? `${name}\n${value}` : value.replace(" ", "\n");
      await this.show(action, title, () => action.setTitle(title));
    }
  }

  override async onDialRotate(ev: DialRotateEvent<Settings>): Promise<void> {
    const s = this.settingsOf(ev.action);
    const delta = ev.payload.ticks * Math.abs(s.step ?? DIAL_STEP);
    if (!faderust.send({ type: "addStripGain", input: s.input ?? 0, delta })) await this.offline(ev.action);
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
    if (!faderust.send({ type: "addStripGain", input: s.input ?? 0, delta })) await this.offline(ev.action);
  }

  async #toggleMute(action: Action<Settings>): Promise<void> {
    const input = this.settingsOf(action).input ?? 0;
    const muted = faderust.state?.inputMuted?.[input];
    if (muted === undefined || !faderust.send({ type: "setInputMuted", input, muted: !muted })) {
      await this.offline(action);
    }
  }
}
