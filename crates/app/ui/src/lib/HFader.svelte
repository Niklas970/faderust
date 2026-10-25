<script>
  import { DB } from "./scale.js";

  /**
   * Horizontal gain fader for the matrix and the program list. Drag, click
   * to jump, scroll to nudge (Shift: finer), double-click for 0 dB, arrow
   * keys when focused.
   */
  let { value, label, dim = false, onChange } = $props();

  let dragging = $state(false);
  let local = $state(0);
  $effect(() => {
    if (!dragging) local = DB.clamp(value);
  });
  const pos = $derived(DB.toPos(local));

  function set(v) {
    const next = DB.clamp(v);
    if (next === local) return;
    local = next;
    onChange(local);
  }
  const nudge = (steps, fine) => set(DB.nudge(local, steps * (fine ? DB.fine : DB.step)));

  let track = $state();
  let grab = 0;
  function pointerPos(e) {
    const r = track.getBoundingClientRect();
    return Math.max(0, Math.min(1, (e.clientX - r.left) / r.width));
  }
  function down(e) {
    if (e.button !== 0) return;
    track.setPointerCapture(e.pointerId);
    dragging = true;
    const p = pointerPos(e);
    const capHalf = 12 / track.getBoundingClientRect().width;
    grab = Math.abs(p - pos) <= capHalf ? pos - p : 0;
    if (!grab) set(DB.fromPos(p));
  }
  function move(e) {
    if (dragging) set(DB.fromPos(pointerPos(e) + grab));
  }
  function up(e) {
    dragging = false;
    track.releasePointerCapture?.(e.pointerId);
  }
  function key(e) {
    const steps = { ArrowRight: 1, ArrowUp: 1, ArrowLeft: -1, ArrowDown: -1 };
    if (e.key in steps) nudge(steps[e.key], e.shiftKey);
    else if (e.key === "Home") set(DB.home);
    else return;
    e.preventDefault();
  }
  $effect(() => {
    if (!track) return;
    const wheel = (e) => {
      e.preventDefault();
      nudge(e.deltaY < 0 ? 1 : -1, e.shiftKey);
    };
    track.addEventListener("wheel", wheel, { passive: false });
    return () => track.removeEventListener("wheel", wheel);
  });
</script>

<div
  class="track"
  class:dim
  class:dragging
  bind:this={track}
  role="slider"
  tabindex="0"
  aria-label={label}
  aria-valuemin={DB.min}
  aria-valuemax={DB.max}
  aria-valuenow={local <= -120 ? DB.min : local}
  aria-valuetext={DB.aria(local)}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
  onkeydown={key}
  ondblclick={() => set(DB.home)}
>
  <div class="slot"><div class="fill" style:width={`calc(${pos * 100}% - 6px)`}></div></div>
  <div class="zero" style:left={`${DB.toPos(0) * 100}%`}></div>
  <div class="cap" style:left={`${pos * 100}%`}></div>
</div>

<style>
  .track {
    position: relative;
    flex: 1;
    min-width: 80px;
    height: 28px;
    margin: 0 12px;
    cursor: ew-resize;
    touch-action: none;
  }
  .dim {
    opacity: 0.45;
  }
  .slot {
    position: absolute;
    left: 0;
    right: 0;
    top: 9px;
    height: 10px;
    background: #050505;
    border-radius: 5px;
    box-shadow:
      inset 0 2px 4px #000,
      0 0 0 1px #2c2f33;
  }
  .fill {
    position: absolute;
    top: 3px;
    bottom: 3px;
    left: 3px;
    min-width: 0;
    border-radius: 2px;
    background: var(--accent);
    box-shadow: 0 0 6px color-mix(in srgb, var(--accent) 60%, transparent);
  }
  .zero {
    position: absolute;
    top: 2px;
    bottom: 2px;
    width: 1px;
    background: var(--muted);
  }
  .cap {
    position: absolute;
    top: 0;
    width: 22px;
    height: 28px;
    transform: translateX(-50%);
    background: linear-gradient(90deg, #55595f, #2f3338 45%, #23262a 55%, #33373c);
    border: 1px solid #0c0d0f;
    border-top: 3px solid var(--accent);
    border-bottom: 3px solid var(--accent);
    border-radius: 3px;
    box-shadow: 0 3px 6px rgb(0 0 0 / 0.5);
    pointer-events: none;
    transition: left 40ms linear;
  }
  .cap::after {
    content: "";
    position: absolute;
    top: -3px;
    bottom: -3px;
    left: 50%;
    width: 2px;
    margin-left: -1px;
    background: var(--accent-light);
  }
  .dragging .cap,
  .track:focus-visible .cap {
    transition: none;
  }
  .track:focus-visible {
    outline-offset: 4px;
  }
  .track:hover .cap {
    filter: brightness(1.12);
  }
</style>
