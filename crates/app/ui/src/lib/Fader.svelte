<script>
  import Meter from "./Meter.svelte";
  import { DB, OFF } from "./scale.js";

  /**
   * A desk fader with scale, mute and an optional meter. `unit` "db" is an
   * input's gain on an output (-∞…+12 dB, hardware taper); "pct" a Windows
   * volume (0…100 %). Drag the cap, click the track to jump, scroll to nudge
   * (Shift: finer), double-click for the default, arrow keys when focused.
   */
  let {
    label,
    value,
    unit = "db",
    muted,
    levelDb = null,
    connected = true,
    note = "",
    noteOn = false,
    // Silenced by Faderust (a feedback loop): shown, but plays nothing.
    blocked = false,
    noteTitle = "",
    showMute = true,
    onChange,
    onMute,
  } = $props();

  const PCT = {
    toPos: (v) => v / 100,
    fromPos: (p) => p * 100,
    clamp: (v) => Math.max(0, Math.min(100, Math.round(v))),
    nudge: (v, delta) => v + delta,
    format: (v) => `${v}`,
    ticks: [100, 75, 50, 25, 0].map((t) => [t, `${t}`]),
    step: 2,
    fine: 1,
    page: 10,
    home: 100,
    min: 0,
    max: 100,
    aria: (v) => `${v} %`,
  };
  const scale = $derived(unit === "pct" ? PCT : DB);

  let dragging = $state(false);
  let local = $state(0);
  // Follow the outside value unless the user is moving the fader right now.
  $effect(() => {
    if (!dragging) local = scale.clamp(value);
  });

  const pos = $derived(scale.toPos(local));
  const isOff = $derived(unit === "pct" ? local === 0 : local <= OFF);

  function set(v) {
    const next = scale.clamp(v);
    if (next === local) return;
    local = next;
    onChange(local);
  }

  function nudge(steps, fine) {
    set(scale.nudge(local, steps * (fine ? scale.fine : scale.step)));
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
    const capHalf = 15 / track.getBoundingClientRect().height;
    // On the cap: move relative to where it was grabbed. Elsewhere: jump.
    grab = Math.abs(p - pos) <= capHalf ? pos - p : 0;
    if (!grab) set(scale.fromPos(p));
  }

  function move(e) {
    if (dragging) set(scale.fromPos(pointerPos(e) + grab));
  }

  function up(e) {
    dragging = false;
    track.releasePointerCapture?.(e.pointerId);
  }

  function key(e) {
    const steps = { ArrowUp: 1, ArrowDown: -1 };
    const pages = { PageUp: 1, PageDown: -1 };
    if (e.key in steps) {
      nudge(steps[e.key], e.shiftKey);
    } else if (e.key in pages) {
      set(local + pages[e.key] * scale.page);
    } else if (e.key === "Home") {
      set(scale.home);
    } else {
      return;
    }
    e.preventDefault();
  }

  // Not passive, so the page does not scroll while the wheel moves the fader.
  $effect(() => {
    if (!track) return;
    const wheel = (e) => {
      e.preventDefault();
      nudge(e.deltaY < 0 ? 1 : -1, e.shiftKey);
    };
    track.addEventListener("wheel", wheel, { passive: false });
    return () => track.removeEventListener("wheel", wheel);
  });

  const meterDb = $derived(
    levelDb == null || !connected || muted || blocked ? OFF : levelDb + (unit === "db" ? Math.max(local, OFF) : 0),
  );
</script>

<div class="strip" class:offline={!connected || blocked}>
  <div class="label" title={label}>{label}</div>
  {#if note}<div class="note" class:on={noteOn} class:warn={blocked} title={noteTitle}>{note}</div>{/if}
  <div class="body">
    <div
      class="track"
      class:dragging
      bind:this={track}
      role="slider"
      tabindex="0"
      aria-label={`${label} Pegel`}
      aria-valuemin={scale.min}
      aria-valuemax={scale.max}
      aria-valuenow={isOff ? scale.min : local}
      aria-valuetext={scale.aria(local)}
      onpointerdown={down}
      onpointermove={move}
      onpointerup={up}
      onpointercancel={up}
      onkeydown={key}
      ondblclick={() => set(scale.home)}
    >
      <div class="scale" aria-hidden="true">
        {#each scale.ticks as [t, text]}
          <span class="tick" class:zero={t === scale.home} style:bottom={`${scale.toPos(t) * 100}%`}>{text}</span>
        {/each}
      </div>
      <div class="slot">
        <div class="fill" class:muted style:height={`${pos * 100}%`}></div>
      </div>
      <div class="cap" class:off={isOff} style:bottom={`${pos * 100}%`}>
        <span class="value">{scale.format(local)}</span>
      </div>
    </div>
    {#if levelDb != null}<Meter db={meterDb} />{/if}
  </div>
  <div class="readout" class:off={isOff || muted}>{scale.format(local)} {unit === "pct" ? "%" : "dB"}</div>
  {#if showMute}
    <button class="mute" class:on={muted} aria-pressed={muted} aria-label={`${label} stumm`} onclick={() => onMute(!muted)}
      >Mute</button
    >
  {/if}
</div>

<style>
  .strip {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    width: 124px;
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
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text);
    background: var(--panel-2);
    border-radius: var(--radius-sm);
  }
  .note {
    margin-top: -4px;
    font-size: 10px;
    color: var(--muted);
  }
  .note.on {
    color: var(--signal);
  }
  /* Over the scale, so the fader stays level with its neighbours. */
  .note.warn {
    position: absolute;
    top: 30px;
    margin: 0;
    color: var(--danger);
    font-weight: 700;
  }
  .body {
    flex: 1;
    display: flex;
    gap: 6px;
    min-height: 160px;
    padding: 18px 0;
  }
  .track {
    position: relative;
    width: 92px;
    height: 100%;
    cursor: ns-resize;
    touch-action: none;
  }
  .track:focus-visible {
    outline-offset: 18px;
  }
  .scale {
    position: absolute;
    inset: 0 66px 0 0;
  }
  .tick {
    position: absolute;
    right: 0;
    transform: translateY(50%);
    font-family: var(--mono);
    font-size: 10px;
    line-height: 1;
    color: var(--muted);
  }
  /* Scale marks left and right of the slot. */
  .tick::before,
  .tick::after {
    content: "";
    position: absolute;
    top: 50%;
    width: 7px;
    height: 1px;
    background: var(--border);
  }
  .tick::before {
    right: -27px;
  }
  .tick::after {
    right: -45px;
  }
  .tick.zero {
    color: var(--text);
    font-weight: 700;
  }
  .tick.zero::before,
  .tick.zero::after {
    background: var(--muted);
  }
  .slot {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 53px;
    width: 10px;
    background: #050505;
    border-radius: 5px;
    box-shadow:
      inset 0 2px 4px #000,
      0 0 0 1px #2c2f33;
  }
  /* Glowing accent bar up to the cap; grey when muted. */
  .fill {
    position: absolute;
    left: 3px;
    right: 3px;
    bottom: 3px;
    max-height: calc(100% - 6px);
    border-radius: 2px;
    background: var(--accent);
    box-shadow: 0 0 8px color-mix(in srgb, var(--accent) 60%, transparent);
  }
  .fill.muted {
    background: #555a61;
    box-shadow: none;
  }
  /* Graphite cap with accent edges and a glowing index line; gain in a window. */
  .cap {
    position: absolute;
    left: 29px;
    width: 58px;
    height: 40px;
    transform: translateY(50%);
    display: grid;
    place-items: center;
    background:
      repeating-linear-gradient(90deg, rgb(255 255 255 / 0.035) 0 1px, transparent 1px 2px),
      linear-gradient(#55595f, #2f3338 45%, #23262a 55%, #33373c);
    border: 1px solid #0c0d0f;
    border-left: 3px solid var(--accent);
    border-right: 3px solid var(--accent);
    border-radius: 4px;
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.25),
      0 4px 8px rgb(0 0 0 / 0.6);
    pointer-events: none;
    transition: bottom 40ms linear;
  }
  .cap::after {
    content: "";
    position: absolute;
    left: -3px;
    right: -3px;
    top: 50%;
    height: 2px;
    margin-top: -1px;
    background: var(--accent-light);
    box-shadow: 0 0 6px var(--accent);
  }
  .cap.off {
    border-left-color: #555a61;
    border-right-color: #555a61;
  }
  .cap.off::after {
    background: #858b93;
    box-shadow: none;
  }
  .value {
    position: relative;
    z-index: 1;
    min-width: 44px;
    padding: 3px 4px;
    text-align: center;
    font-family: var(--mono);
    font-size: 12px;
    font-weight: 700;
    line-height: 1;
    color: #f1f3f5;
    background: #0b0c0e;
    border-radius: 2px;
    box-shadow:
      inset 0 1px 2px #000,
      0 1px 0 rgb(255 255 255 / 0.15);
  }
  .cap.off .value {
    color: var(--muted);
  }
  .track:hover .cap {
    filter: brightness(1.12);
  }
  .track:focus-visible .cap,
  .dragging .cap {
    transition: none;
  }
  .track:focus-visible .value,
  .dragging .value {
    color: var(--accent-light);
  }
  .body :global(.meter.vertical) {
    width: 14px;
  }
  .readout {
    margin-top: -6px;
    font-family: var(--mono);
    font-size: 13px;
    font-weight: 700;
    color: var(--accent-light);
  }
  .readout.off {
    color: var(--muted);
  }
  /* Lit keycap: an LED strip on top glows red when muted. */
  .mute {
    position: relative;
    width: 72px;
    height: 32px;
    padding: 6px 0 0;
    border-radius: 4px;
    border: 1px solid #101113;
    background: linear-gradient(#3b3f45, #2a2d32 60%, #24272b);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #aeb4bb;
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.14),
      0 2px 3px rgb(0 0 0 / 0.5);
  }
  .mute::before {
    content: "";
    position: absolute;
    top: 4px;
    left: 50%;
    width: 16px;
    height: 3px;
    margin-left: -8px;
    border-radius: 2px;
    background: #15171a;
    box-shadow: inset 0 1px 1px #000;
  }
  .mute:hover {
    color: var(--text);
  }
  .mute:active {
    transform: translateY(1px);
    box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.1);
  }
  .mute.on {
    color: #fff;
    background: linear-gradient(#e8665f, #c8423b 60%, #b03630);
    border-color: #5a1a17;
  }
  .mute.on::before {
    background: #ffd2cf;
    box-shadow: 0 0 6px #ff6b62;
  }
</style>
