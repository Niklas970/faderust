<script>
  import Meter from "./Meter.svelte";

  /** One input on one output: level fader, mute and the input's meter. */
  let { label, db, muted, levelDb, connected = true, onGain, onMute } = $props();

  const MIN = -61; // bottom position = off
  const MAX = 6;

  let dragging = $state(false);
  let local = $state(0);
  // Follow the engine unless the user is moving the fader right now.
  $effect(() => {
    if (!dragging) local = Math.max(MIN, Math.min(MAX, db));
  });

  const text = $derived(local <= MIN ? "−∞" : `${local > 0 ? "+" : ""}${local.toFixed(1)}`);

  function input(e) {
    local = Number(e.currentTarget.value);
    onGain(local <= MIN ? -120 : local);
  }
</script>

<div class="strip" class:offline={!connected}>
  <div class="label" title={label}>{label}</div>
  <div class="body">
    <Meter db={connected && !muted ? levelDb + Math.max(local, -120) : -120} thin />
    <input
      type="range"
      min={MIN}
      max={MAX}
      step="0.5"
      value={local}
      aria-label={`${label} Pegel`}
      oninput={input}
      onpointerdown={() => (dragging = true)}
      onpointerup={() => (dragging = false)}
      onblur={() => (dragging = false)}
      ondblclick={() => {
        local = 0;
        onGain(0);
      }}
    />
  </div>
  <div class="db">{text}</div>
  <button class="mute" class:on={muted} aria-pressed={muted} onclick={() => onMute(!muted)}>M</button>
</div>

<style>
  .strip {
    display: grid;
    grid-template-rows: auto 1fr auto auto;
    justify-items: center;
    gap: 6px;
    width: 64px;
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
    color: var(--muted);
  }
  .body {
    display: flex;
    gap: 6px;
    height: 100%;
    min-height: 140px;
  }
  input[type="range"] {
    writing-mode: vertical-lr;
    direction: rtl;
    width: 22px;
    height: 100%;
    margin: 0;
    accent-color: var(--copper);
    cursor: ns-resize;
  }
  .db {
    font-variant-numeric: tabular-nums;
    font-size: 12px;
  }
  .mute {
    width: 32px;
    height: 24px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--panel-2);
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }
  .mute.on {
    background: var(--danger);
    border-color: var(--danger);
    color: #fff;
  }
</style>
