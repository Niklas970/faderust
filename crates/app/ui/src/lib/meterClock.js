// One animation loop for every meter. Each meter used to run its own
// requestAnimationFrame at the display rate; a shared loop capped at ~30 fps
// draws the same motion with far fewer wake-ups of the web view.
const STEP_MS = 33;
const tickers = new Set();
let frame = 0;
let last = 0;

function run(now) {
  if (now - last >= STEP_MS) {
    const dt = Math.min(0.1, (now - last) / 1000);
    last = now;
    for (const tick of [...tickers]) tick(dt);
  }
  frame = tickers.size ? requestAnimationFrame(run) : 0;
}

/** Calls `tick(dtSeconds)` each step until it returns false or is removed. */
export function addTicker(tick) {
  if (!tickers.size) last = performance.now();
  tickers.add(tick);
  if (!frame) frame = requestAnimationFrame(run);
  return () => tickers.delete(tick);
}
