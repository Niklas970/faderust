<script>
  import Meter from "./Meter.svelte";

  /**
   * One input on one output: fader with scale, mute and the input's meter.
   * Drag the cap, click the track to jump, scroll to nudge (Shift: finer),
   * double-click for 0 dB, arrow keys when focused.
   */
  let { label, db, muted, levelDb, connected = true, onGain, onMute } = $props();

  const MAX = 6;
  const OFF = -120;
  const TICKS = [6, 0, -6, -12, -24, -48];

  // Taper like a hardware fader: most of the travel for the useful range,
  // the bottom 5 % is off.
  function toPos(d) {
    if (d <= -60) return 0;
    if (d < -24) return 0.05 + ((d + 60) / 36) * 0.2;
    return 0.25 + ((Math.min(d, MAX) + 24) / 30) * 0.75;
  }
  function toDb(p) {
    if (p <= 0.05) return OFF;
    if (p < 0.25) return -60 + ((p - 0.05) / 0.2) * 36;
    return -24 + ((Math.min(p, 1) - 0.25) / 0.75) * 30;
  }
  const clamp = (d) => (d <= -60 ? OFF : Math.min(MAX, Math.round(d * 2) / 2));

  let dragging = $state(false);
  let local = $state(0);
  // Follow the engine unless the user is moving the fader right now.
  $effect(() => {
    if (!dragging) local = clamp(db);
  });

  const text = $derived(local <= OFF ? "−∞" : `${local > 0 ? "+" : ""}${local.toFixed(1)}`);
  const pos = $derived(toPos(local));

  function set(d) {
    const next = clamp(d);
    if (next === local) return;
    local = next;
    onGain(local);
  }

  /** Steps by `delta` dB; from off the first step up lands at -59 dB. */
  function nudge(delta) {
    if (local <= OFF) {
      if (delta > 0) set(-59);
      return;
    }
    set(local + delta);
  }

  let track = $state();
  let grab = 0; // offset between pointer and cap while dragging

  function pointerPos(e) {
    const r = track.getBoundingClientRect();
    return Math.max(0, Math.min(1, 1 - (e.clientY - r.top) / r.height));
  }

  function down(e) {
    if (e.button !== 0) return;
    track.setPointerCapture(e.pointerId);
    dragging = true;
    const p = pointerPos(e);
    const capHalf = 12 / track.getBoundingClientRect().height;
    // On the cap: move relative to where it was grabbed. Elsewhere: jump.
    grab = Math.abs(p - pos) <= capHalf ? pos - p : 0;
    if (!grab) set(toDb(p));
  }

  function move(e) {
    if (dragging) set(toDb(pointerPos(e) + grab));
  }

  function up(e) {
    dragging = false;
    track.releasePointerCapture?.(e.pointerId);
  }

  function key(e) {
    const steps = { ArrowUp: 1, ArrowDown: -1, PageUp: 6, PageDown: -6 };
    if (e.key in steps) {
      nudge(steps[e.key] * (e.shiftKey ? 0.5 : 1));
      e.preventDefault();
    } else if (e.key === "Home") {
      set(0);
      e.preventDefault();
    }
  }

  // Not passive, so the page does not scroll while the wheel moves the fader.
  $effect(() => {
    if (!track) return;
    const wheel = (e) => {
      e.preventDefault();
      nudge((e.deltaY < 0 ? 1 : -1) * (e.shiftKey ? 0.5 : 1));
    };
    track.addEventListener("wheel", wheel, { passive: false });
    return () => track.removeEventListener("wheel", wheel);
  });
</script>

<div class="strip" class:offline={!connected}>
  <div class="label" title={label}>{label}</div>
  <div class="body">
    <Meter db={connected && !muted ? levelDb + Math.max(local, OFF) : OFF} thin />
    <div
      class="track"
      class:dragging
      bind:this={track}
      role="slider"
      tabindex="0"
      aria-label={`${label} Pegel`}
      aria-valuemin={-60}
      aria-valuemax={MAX}
      aria-valuenow={local <= OFF ? -60 : local}
      aria-valuetext={local <= OFF ? "aus" : `${local.toFixed(1)} dB`}
      onpointerdown={down}
      onpointermove={move}
      onpointerup={up}
      onpointercancel={up}
      onkeydown={key}
      ondblclick={() => set(0)}
    >
      <div class="scale" aria-hidden="true">
        {#each TICKS as t}
          <span class="tick" class:zero={t === 0} style:bottom={`${toPos(t) * 100}%`}>{t > 0 ? `+${t}` : t}</span>
        {/each}
        <span class="tick" style:bottom="0%">−∞</span>
      </div>
      <div class="rail">
        <div class="signal" class:muted style:height={`${pos * 100}%`}></div>
      </div>
      <div class="cap" style:bottom={`${pos * 100}%`}></div>
    </div>
  </div>
  <div class="db" class:off={local <= OFF}>{text}</div>
  <button class="mute" class:on={muted} aria-pressed={muted} onclick={() => onMute(!muted)}>M</button>
</div>

<style>
  .strip {
    display: grid;
    grid-template-rows: auto 1fr auto auto;
    justify-items: center;
    gap: 8px;
    width: 84px;
    min-height: 0;
  }
  .offline {
    opacity: 0.45;
  }
  .label {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }
  .body {
    display: flex;
    gap: 6px;
    height: 100%;
    min-height: 160px;
    padding: 11px 0;
  }
  .track {
    position: relative;
    width: 64px;
    height: 100%;
    cursor: ns-resize;
    touch-action: none;
    border-radius: 6px;
  }
  .track:focus-visible {
    outline-offset: 4px;
  }
  .scale {
    position: absolute;
    inset: 0 44px 0 0;
  }
  .tick {
    position: absolute;
    right: 0;
    transform: translateY(50%);
    font-size: 9px;
    line-height: 1;
    color: var(--muted);
    opacity: 0.7;
    font-variant-numeric: tabular-nums;
  }
  .tick.zero {
    color: var(--text);
    opacity: 1;
  }
  .rail {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 37px;
    width: 6px;
    border-radius: 3px;
    background: var(--panel-2);
    box-shadow: inset 0 0 0 1px var(--border);
    overflow: hidden;
  }
  .signal {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    background: linear-gradient(to top, color-mix(in srgb, var(--teal) 35%, transparent), var(--teal));
  }
  .signal.muted {
    background: var(--border);
  }
  .cap {
    position: absolute;
    left: 23px;
    width: 34px;
    height: 22px;
    transform: translateY(50%);
    border-radius: 5px;
    background: linear-gradient(to bottom, var(--copper-light), var(--copper) 55%, color-mix(in srgb, var(--copper) 75%, black));
    box-shadow:
      0 2px 6px rgb(0 0 0 / 0.35),
      inset 0 1px 0 rgb(255 255 255 / 0.25);
    pointer-events: none;
    transition: bottom 40ms linear;
  }
  .dragging .cap {
    transition: none;
  }
  .cap::after {
    content: "";
    position: absolute;
    left: 7px;
    right: 7px;
    top: 50%;
    height: 2px;
    margin-top: -1px;
    border-radius: 1px;
    background: rgb(0 0 0 / 0.35);
  }
  .db {
    font-variant-numeric: tabular-nums;
    font-size: 13px;
    font-weight: 600;
  }
  .db.off {
    color: var(--muted);
  }
  .mute {
    width: 44px;
    height: 26px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--panel-2);
    font-size: 12px;
    font-weight: 700;
    color: var(--muted);
  }
  .mute.on {
    background: var(--danger);
    border-color: var(--danger);
    color: #fff;
  }
</style>
