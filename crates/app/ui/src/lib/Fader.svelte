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
      <div class="slot">
        <div class="fill" class:muted style:height={`${pos * 100}%`}></div>
      </div>
      <div class="cap" class:off={local <= OFF} style:bottom={`${pos * 100}%`}>{text}</div>
    </div>
    <Meter db={connected && !muted ? levelDb + Math.max(local, OFF) : OFF} thin />
  </div>
  <button class="mute" class:on={muted} aria-pressed={muted} aria-label={`${label} stumm`} onclick={() => onMute(!muted)}
    >Mute</button
  >
</div>

<style>
  .strip {
    display: grid;
    grid-template-rows: auto 1fr auto;
    justify-items: center;
    gap: 8px;
    width: 78px;
    min-height: 0;
  }
  .offline {
    opacity: 0.45;
  }
  .label {
    width: 100%;
    padding: 3px 4px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: center;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text);
    background: var(--panel-2);
    border-radius: var(--radius-sm);
  }
  .body {
    display: flex;
    gap: 4px;
    height: 100%;
    min-height: 160px;
    padding: 11px 0;
  }
  .track {
    position: relative;
    width: 58px;
    height: 100%;
    cursor: ns-resize;
    touch-action: none;
  }
  .track:focus-visible {
    outline-offset: 12px;
  }
  .scale {
    position: absolute;
    inset: 0 42px 0 0;
  }
  .tick {
    position: absolute;
    right: 0;
    transform: translateY(50%);
    font-family: var(--mono);
    font-size: 9px;
    line-height: 1;
    color: var(--muted);
  }
  .tick::after {
    content: "";
    position: absolute;
    top: 50%;
    right: -20px;
    width: 4px;
    height: 1px;
    background: var(--border);
  }
  .tick.zero {
    color: var(--text);
  }
  .tick.zero::after {
    background: var(--muted);
  }
  .slot {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 35px;
    width: 4px;
    background: var(--well);
    box-shadow: 0 0 0 1px var(--border);
  }
  .fill {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    background: var(--copper);
  }
  .fill.muted {
    background: var(--border);
  }
  /* Flat cap with the gain printed on it. */
  .cap {
    position: absolute;
    left: 18px;
    width: 38px;
    height: 20px;
    transform: translateY(50%);
    display: grid;
    place-items: center;
    font-family: var(--mono);
    font-size: 10px;
    line-height: 1;
    color: var(--text);
    background: var(--panel-2);
    border: 1px solid #5a6068;
    border-top-color: #737a83;
    border-radius: 2px;
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.5);
    pointer-events: none;
    transition: bottom 40ms linear;
  }
  .cap.off {
    color: var(--muted);
  }
  .track:focus-visible .cap,
  .dragging .cap {
    border-color: var(--copper);
    color: var(--copper-light);
    transition: none;
  }
  .mute {
    width: 52px;
    height: 22px;
    padding: 0;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--panel-2);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .mute:hover {
    color: var(--text);
  }
  .mute.on {
    background: var(--danger);
    border-color: var(--danger);
    color: #fff;
  }
</style>
