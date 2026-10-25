// Gain scale shared by the faders: -∞…+12 dB with a hardware-like taper.

export const OFF = -120;

export const DB = {
  // Most of the travel for the useful range, the bottom 5 % is off.
  toPos(d) {
    if (d <= -60) return 0;
    if (d < -24) return 0.05 + ((d + 60) / 36) * 0.2;
    return 0.25 + ((Math.min(d, 12) + 24) / 36) * 0.75;
  },
  fromPos(p) {
    if (p <= 0.05) return OFF;
    if (p < 0.25) return -60 + ((p - 0.05) / 0.2) * 36;
    return -24 + ((Math.min(p, 1) - 0.25) / 0.75) * 36;
  },
  clamp: (d) => (d <= -60 ? OFF : Math.min(12, Math.round(d * 2) / 2)),
  // From off, the first step up lands at -59 dB.
  nudge: (d, delta) => (d <= OFF ? (delta > 0 ? -59 : OFF) : d + delta),
  format: (d) => (d <= OFF ? "−∞" : `${d > 0 ? "+" : ""}${d.toFixed(1)}`),
  ticks: [
    [12, "+12"],
    [6, "+6"],
    [0, "0"],
    [-6, "-6"],
    [-12, "-12"],
    [-24, "-24"],
    [-48, "-48"],
    [OFF, "−∞"],
  ],
  step: 1,
  fine: 0.5,
  page: 6,
  home: 0,
  min: -60,
  max: 12,
  aria: (d) => (d <= OFF ? "−∞" : `${d.toFixed(1)} dB`),
};

/**
 * Strip view: one fader for an input on all outputs it plays on. It shows
 * the loudest of them; moving it shifts all by the same amount, so their
 * differences (set in the matrix) stay.
 */
export function stripValue(row) {
  const on = row.filter((g) => !g.muted);
  return (on.length ? on : row).reduce((m, g) => Math.max(m, g.db), OFF);
}

/** `[output, dB]` pairs that move the strip `row` to `next`. */
export function stripChanges(row, next) {
  const base = stripValue(row);
  const delta = next - base;
  return row.flatMap((g, o) => {
    if (g.muted) return [];
    if (next <= OFF) return [[o, OFF]];
    const db = g.db <= OFF || base <= OFF ? next : DB.clamp(g.db + delta);
    return [[o, db]];
  });
}
