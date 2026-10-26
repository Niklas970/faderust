<script>
  import { untrack } from "svelte";
  /**
   * LED bar meter in dBFS, -60 dB (empty) to 0 dB (full). Signal segments
   * up to -12 dB, warning up to -3 dB, clip above.
   */
  let { db = -120, vertical = true, thin = false } = $props();

  // Rises at once, falls slowly like a hardware meter (dB per second). The
  // fall runs on animation frames, so every meter keeps falling in step even
  // when its value stops changing (a stream that ends sends -∞ only once).
  const FALL = 20;
  let shown = $state(-120);
  let target = -120;
  let frame = 0;
  let last = 0;

  function fall(now) {
    const dt = Math.max(0, Math.min(0.1, (now - last) / 1000));
    last = now;
    shown = Math.max(target, shown - FALL * dt);
    frame = shown > target ? requestAnimationFrame(fall) : 0;
  }

  $effect(() => {
    target = db;
    if (db >= untrack(() => shown)) {
      shown = db;
    } else if (!frame) {
      last = performance.now();
      frame = requestAnimationFrame(fall);
    }
  });

  $effect(() => () => cancelAnimationFrame(frame));

  const pct = $derived(Math.max(0, Math.min(100, ((shown + 60) / 60) * 100)));
</script>

<div class="meter" class:vertical class:thin aria-hidden="true">
  <div class="leds" style:--pct={`${pct}%`}></div>
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
  /* Full-size color ramp, cut down to the level; the mask splits it into LEDs. */
  .leds {
    position: absolute;
    inset: 1px;
    transition: clip-path 60ms linear;
  }
  .vertical .leds {
    background: linear-gradient(
      to top,
      var(--signal) 0 80%,
      var(--amber) 80% 95%,
      var(--danger) 95%
    );
    clip-path: inset(calc(100% - var(--pct)) 0 0 0);
    mask: repeating-linear-gradient(to top, #000 0 4px, transparent 4px 6px);
  }
  .meter:not(.vertical) .leds {
    background: linear-gradient(
      to right,
      var(--signal) 0 80%,
      var(--amber) 80% 95%,
      var(--danger) 95%
    );
    clip-path: inset(0 calc(100% - var(--pct)) 0 0);
    mask: repeating-linear-gradient(to right, #000 0 3px, transparent 3px 4px);
  }
</style>
