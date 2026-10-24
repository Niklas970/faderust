<script>
  import Meter from "./Meter.svelte";

  /**
   * A desk fader with scale, mute and an optional meter. `unit` "db" is an
   * input's gain on an output (-∞…+6 dB, hardware taper); "pct" a Windows
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
    onChange,
    onMute,
  } = $props();

  const OFF = -120;
  const DB = {
    // Most of the travel for the useful range, the bottom 5 % is off.
    toPos(d) {
      if (d <= -60) return 0;
      if (d < -24) return 0.05 + ((d + 60) / 36) * 0.2;
      return 0.25 + ((Math.min(d, 6) + 24) / 30) * 0.75;
    },
    fromPos(p) {
      if (p <= 0.05) return OFF;
      if (p < 0.25) return -60 + ((p - 0.05) / 0.2) * 36;
      return -24 + ((Math.min(p, 1) - 0.25) / 0.75) * 30;
    },
    clamp: (d) => (d <= -60 ? OFF : Math.min(6, Math.round(d * 2) / 2)),
    // From off, the first step up lands at -59 dB.
    nudge: (d, delta) => (d <= OFF ? (delta > 0 ? -59 : OFF) : d + delta),
    format: (d) => (d <= OFF ? "−∞" : `${d > 0 ? "+" : ""}${d.toFixed(1)}`),
    ticks: [
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
    max: 6,
    aria: (d) => (d <= OFF ? "aus" : `${d.toFixed(1)} dB`),
  };
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
    levelDb == null || !connected || muted ? OFF : levelDb + (unit === "db" ? Math.max(local, OFF) : 0),
  );
</script>

<div class="strip" class:offline={!connected}>
  <div class="label" title={label}>{label}</div>
  {#if note}<div class="note" class:on={noteOn}>{note}</div>{/if}
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
  <button class="mute" class:on={muted} aria-pressed={muted} aria-label={`${label} stumm`} onclick={() => onMute(!muted)}
    >Mute</button
  >
</div>

<style>
  .strip {
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
    color: var(--teal);
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
    left: 55px;
    width: 6px;
    background: var(--well);
    border-radius: 3px;
    box-shadow:
      inset 0 1px 3px rgb(0 0 0 / 0.8),
      0 0 0 1px var(--border);
    overflow: hidden;
  }
  .fill {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    background: linear-gradient(to top, color-mix(in srgb, var(--copper) 55%, black), var(--copper));
  }
  .fill.muted {
    background: var(--border);
  }
  /* Hardware-style cap: shaded body, grip ridges, gain in a recessed window. */
  .cap {
    position: absolute;
    left: 29px;
    width: 58px;
    height: 38px;
    transform: translateY(50%);
    display: grid;
    place-items: center;
    background: linear-gradient(
      to bottom,
      #646b74 0%,
      #464c53 18%,
      #33383e 50%,
      #2a2e33 82%,
      #3a3f45 100%
    );
    border: 1px solid #121417;
    border-radius: 4px;
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.22),
      inset 0 -1px 0 rgb(0 0 0 / 0.4),
      0 3px 6px rgb(0 0 0 / 0.55);
    pointer-events: none;
    transition: bottom 40ms linear;
  }
  .cap::before,
  .cap::after {
    content: "";
    position: absolute;
    left: 8px;
    right: 8px;
    height: 2px;
    border-top: 1px solid rgb(0 0 0 / 0.45);
    border-bottom: 1px solid rgb(255 255 255 / 0.12);
  }
  .cap::before {
    top: 4px;
  }
  .cap::after {
    bottom: 4px;
  }
  .value {
    min-width: 42px;
    padding: 2px 3px;
    text-align: center;
    font-family: var(--mono);
    font-size: 12px;
    font-weight: 700;
    line-height: 1;
    color: #e9ecef;
    background: #0d0e10;
    border-radius: 2px;
    box-shadow: inset 0 1px 2px rgb(0 0 0 / 0.9);
  }
  .cap.off .value {
    color: var(--muted);
  }
  .track:hover .cap {
    border-color: #000;
  }
  .track:focus-visible .cap,
  .dragging .cap {
    transition: none;
  }
  .track:focus-visible .value,
  .dragging .value {
    color: var(--copper-light);
  }
  .body :global(.meter.vertical) {
    width: 14px;
  }
  .mute {
    width: 72px;
    height: 28px;
    padding: 0;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: linear-gradient(to bottom, var(--panel-2), var(--panel));
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.4);
  }
  .mute:hover {
    color: var(--text);
  }
  .mute.on {
    background: var(--danger);
    border-color: var(--danger);
    color: #fff;
    box-shadow: inset 0 1px 3px rgb(0 0 0 / 0.4);
  }
</style>
