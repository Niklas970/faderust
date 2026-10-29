<script>
  import { addTicker } from "./meterClock.js";
  /**
   * LED bar meter in dBFS, -60 dB (empty) to 0 dB (full). Signal segments
   * up to -12 dB, warning up to -3 dB, clip above.
   */
  let { db = -120, vertical = true, thin = false } = $props();

  // Both directions are eased, so levels of the same sound that reach the
  // desk a few tens of milliseconds apart (an input and the output playing
  // it) move as one. Rising: time constant in seconds; falling: dB per
  // second, like a hardware meter. The motion runs on animation frames, so
  // every meter keeps moving in step even when its value stops changing (a
  // stream that ends sends -∞ only once).
  const RISE = 0.06;
  const FALL = 12;
  let shown = -120;
  let target = -120;
  let cover;
  let stop = null;

  function draw() {
    const lit = Math.max(0, Math.min(1, (shown + 60) / 60));
    if (cover) cover.style.transform = vertical ? `scaleY(${1 - lit})` : `scaleX(${1 - lit})`;
  }

  // Returns false once the meter has reached its target.
  function move(dt) {
    shown =
      target > shown
        ? shown + (target - shown) * (1 - Math.exp(-dt / RISE))
        : Math.max(target, shown - FALL * dt);
    const done = Math.abs(target - shown) < 0.1;
    if (done) shown = target;
    draw();
    return !done;
  }

  $effect(() => {
    target = db;
    if (!stop && Math.abs(db - shown) >= 0.1) {
      stop = addTicker((dt) => {
        if (!move(dt)) {
          stop?.();
          stop = null;
        }
      });
    }
  });

  $effect(() => {
    draw();
    return () => stop?.();
  });
</script>

<div class="meter" class:vertical class:thin aria-hidden="true">
  <div class="leds"></div>
  <!-- Covers the part above the level. Only its transform changes, which the
       browser moves without redrawing the LEDs under it. -->
  <div class="cover" bind:this={cover}></div>
</div>

<style>
  .meter {
    position: relative;
    background: var(--well);
    border: 1px solid var(--border);
    border-radius: 2px;
    overflow: hidden;
  }
  .meter.vertical {
    width: 14px;
    height: 100%;
  }
  .meter.vertical.thin {
    width: 7px;
  }
  .meter:not(.vertical) {
    height: 6px;
    width: 100%;
  }
  /* Full-size color ramp; the mask splits it into LEDs. Drawn once. */
  .leds,
  .cover {
    position: absolute;
    inset: 1px;
  }
  .cover {
    background: var(--well);
    will-change: transform;
  }
  .vertical .cover {
    transform-origin: top;
  }
  .meter:not(.vertical) .cover {
    transform-origin: right;
  }
  .vertical .leds {
    background: linear-gradient(
      to top,
      var(--signal) 0 80%,
      var(--amber) 80% 95%,
      var(--danger) 95%
    );
    mask: repeating-linear-gradient(to top, #000 0 4px, transparent 4px 6px);
  }
  .meter:not(.vertical) .leds {
    background: linear-gradient(
      to right,
      var(--signal) 0 80%,
      var(--amber) 80% 95%,
      var(--danger) 95%
    );
    mask: repeating-linear-gradient(to right, #000 0 3px, transparent 3px 4px);
  }
</style>
