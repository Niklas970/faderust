<script>
  /**
   * Horizontal Windows volume (0…1) with a mute button. Changes go out one
   * at a time: while one is on its way, only the newest value waits.
   */
  let { volume = 1, muted = false, label, onChange } = $props();

  let value = $state(volume);
  let mute = $state(muted);
  $effect(() => {
    value = volume;
  });
  $effect(() => {
    mute = muted;
  });

  let busy = false;
  let queued = null;
  async function send(v, m) {
    if (busy) {
      queued = [v, m];
      return;
    }
    busy = true;
    try {
      await onChange(v, m);
    } finally {
      busy = false;
      if (queued) {
        const [qv, qm] = queued;
        queued = null;
        send(qv, qm);
      }
    }
  }

  function input(e) {
    value = Number(e.currentTarget.value) / 100;
    send(value, null);
  }

  // Not passive, so the dialog does not scroll while the wheel moves the slider.
  let slider;
  $effect(() => {
    const wheel = (e) => {
      e.preventDefault();
      value = Math.max(0, Math.min(1, Math.round(value * 100 - Math.sign(e.deltaY) * 2) / 100));
      send(value, null);
    };
    slider.addEventListener("wheel", wheel, { passive: false });
    return () => slider.removeEventListener("wheel", wheel);
  });

  function toggle() {
    mute = !mute;
    send(null, mute);
  }
</script>

<div class="volume" class:muted={mute}>
  <button class="mute" onclick={toggle} aria-pressed={mute} aria-label={`${label} stummschalten`} title="Stumm">
    <svg viewBox="0 0 16 16" aria-hidden="true">
      <path d="M2 6h3l4-3v10l-4-3H2z" />
      {#if mute}<path d="M11 6l4 4M15 6l-4 4" />{:else}<path d="M11.5 5.5a3.5 3.5 0 0 1 0 5" />{/if}
    </svg>
  </button>
  <input
    type="range"
    min="0"
    max="100"
    step="1"
    value={Math.round(value * 100)}
    bind:this={slider}
    oninput={input}
    aria-label={`Lautstärke ${label}`}
    style={`--pct:${value * 100}%`}
  />
  <span class="pct">{Math.round(value * 100)}</span>
</div>

<style>
  .volume {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .mute {
    flex: none;
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-2);
  }
  .mute svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: var(--text);
    stroke-width: 1.4;
    stroke-linejoin: round;
    stroke-linecap: round;
  }
  .muted .mute {
    border-color: var(--danger);
  }
  .muted .mute svg {
    stroke: var(--danger);
  }
  input {
    flex: 1;
    min-width: 60px;
    height: 4px;
    margin: 0;
    appearance: none;
    border-radius: 2px;
    background: linear-gradient(to right, var(--teal) var(--pct), var(--border) var(--pct));
    cursor: pointer;
  }
  .muted input {
    background: linear-gradient(to right, var(--muted) var(--pct), var(--border) var(--pct));
  }
  input::-webkit-slider-thumb {
    appearance: none;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--copper);
    border: 2px solid var(--bg);
  }
  input::-moz-range-thumb {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--copper);
    border: 2px solid var(--bg);
  }
  .pct {
    flex: none;
    width: 28px;
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--muted);
    font-size: 12px;
  }
</style>
