import assert from "node:assert/strict";
import { test } from "node:test";

import { faderPercent, formatDb } from "../src/faderust.ts";

test("formats fader values like the app", () => {
  assert.equal(formatDb({ db: 0, muted: false }), "0.0 dB");
  assert.equal(formatDb({ db: -6.24, muted: false }), "−6.0 dB");
  assert.equal(formatDb({ db: 3, muted: false }), "+3.0 dB");
  assert.equal(formatDb({ db: -120, muted: false }), "−∞");
  assert.equal(formatDb({ db: 0, muted: true }), "Stumm");
  assert.equal(formatDb(undefined), "–");
});

test("fader travel follows the app's taper", () => {
  assert.equal(faderPercent(-120), 0);
  assert.equal(faderPercent(-60), 0);
  assert.equal(faderPercent(-42), 15);
  assert.equal(faderPercent(-24), 25);
  assert.equal(faderPercent(12), 100);
  assert.equal(faderPercent(20), 100);
});
