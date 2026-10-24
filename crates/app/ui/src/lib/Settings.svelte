<script>
  import { untrack } from "svelte";
  import { getAutostart, getDevices, setAutostart } from "./api.js";

  /** Device setup: which inputs and outputs, their names and buffers. */
  let { config, onSave, onClose } = $props();

  // Edit a copy; nothing changes until Save.
  let draft = $state(untrack(() => ({ gains: [], ...$state.snapshot(config) })));
  let devices = $state({ inputs: [], outputs: [] });
  getDevices().then((d) => (devices = d));

  // Applies right away, independent of Save.
  let autostart = $state(null);
  let autostartError = $state("");
  getAutostart().then((a) => (autostart = a), (e) => (autostartError = String(e)));
  async function toggleAutostart(enabled) {
    try {
      await setAutostart(enabled);
      autostart = enabled;
      autostartError = "";
    } catch (e) {
      autostartError = String(e);
    }
  }

  // The configured pattern may match a device that is unplugged right now:
  // keep it selectable.
  function options(list, current) {
    const names = list.map((d) => d.name);
    return list.some((d) => d.id === current) || names.some((n) => n.toLowerCase().includes(current.toLowerCase())) || !current
      ? names
      : [current, ...names];
  }

  // Keep the gain matrices (gains[input][output], also in every preset) in
  // step with the lists.
  const matrices = () => [draft.gains, ...(draft.presets ?? []).map((p) => p.gains)];
  const swap = (list, a, b) => ([list[a], list[b]] = [list[b], list[a]]);

  function addInput() {
    draft.inputs.push({ label: `Eingang ${draft.inputs.length + 1}`, device: devices.inputs[0].name });
    for (const m of matrices()) m.push(draft.outputs.map(() => ({ db: 0, muted: false })));
  }
  function removeInput(i) {
    draft.inputs.splice(i, 1);
    for (const m of matrices()) m.splice(i, 1);
  }
  function moveInput(i, by) {
    swap(draft.inputs, i, i + by);
    for (const m of matrices()) if (m[i] && m[i + by]) swap(m, i, i + by);
  }
  function addOutput() {
    draft.outputs.push({ label: `Ausgang ${draft.outputs.length + 1}`, device: devices.outputs[0].name, bufferMs: 60 });
    for (const m of matrices()) for (const row of m) row.push({ db: 0, muted: false });
  }
  function removeOutput(o) {
    draft.outputs.splice(o, 1);
    for (const m of matrices()) for (const row of m) row.splice(o, 1);
  }
  function moveOutput(o, by) {
    swap(draft.outputs, o, o + by);
    for (const m of matrices()) for (const row of m) if (o + by < row.length) swap(row, o, o + by);
  }

  function selected(list, current) {
    return (
      list.find((d) => d.id === current)?.name ??
      list.find((d) => d.name.toLowerCase().includes(current.toLowerCase()))?.name ??
      current
    );
  }
</script>

<div class="backdrop" role="presentation" onclick={onClose}></div>
<div class="dialog" role="dialog" aria-modal="true" aria-label="Geräte">
  <h2>Geräte</h2>

  <h3>Eingänge</h3>
  {#each draft.inputs as input, i}
    <div class="row">
      <input class="name" bind:value={input.label} aria-label="Name" />
      <select
        value={selected(devices.inputs, input.device)}
        onchange={(e) => (input.device = e.currentTarget.value)}
        aria-label="Gerät"
      >
        {#each options(devices.inputs, input.device) as name}<option>{name}</option>{/each}
      </select>
      <span class="order">
        <button class="icon" title="Nach oben" aria-label="Nach oben" disabled={i === 0} onclick={() => moveInput(i, -1)}
          >↑</button
        >
        <button
          class="icon"
          title="Nach unten"
          aria-label="Nach unten"
          disabled={i === draft.inputs.length - 1}
          onclick={() => moveInput(i, 1)}>↓</button
        >
      </span>
      <button class="icon" title="Entfernen" onclick={() => removeInput(i)}>✕</button>
    </div>
  {/each}
  <button
    class="add"
    disabled={!devices.inputs.length}
    onclick={addInput}
  >
    + Eingang
  </button>

  <h3>Ausgänge</h3>
  {#each draft.outputs as output, o}
    <div class="row">
      <input class="name" bind:value={output.label} aria-label="Name" />
      <select
        value={selected(devices.outputs, output.device)}
        onchange={(e) => (output.device = e.currentTarget.value)}
        aria-label="Gerät"
      >
        {#each options(devices.outputs, output.device) as name}<option>{name}</option>{/each}
      </select>
      <label class="buffer" title="Puffer: Bluetooth braucht mehr als USB">
        <input type="number" min="10" max="500" step="10" bind:value={output.bufferMs} />
        ms
      </label>
      <span class="order">
        <button class="icon" title="Nach links" aria-label="Nach links" disabled={o === 0} onclick={() => moveOutput(o, -1)}
          >←</button
        >
        <button
          class="icon"
          title="Nach rechts"
          aria-label="Nach rechts"
          disabled={o === draft.outputs.length - 1}
          onclick={() => moveOutput(o, 1)}>→</button
        >
      </span>
      <button class="icon" title="Entfernen" onclick={() => removeOutput(o)}>✕</button>
    </div>
  {/each}
  <button
    class="add"
    disabled={!devices.outputs.length}
    onclick={addOutput}
  >
    + Ausgang
  </button>

  <h3>Start</h3>
  <label class="check">
    <input
      type="checkbox"
      checked={autostart ?? false}
      disabled={autostart === null}
      onchange={(e) => toggleAutostart(e.currentTarget.checked)}
    />
    Mit Windows starten (im Tray, ohne Fenster)
  </label>
  {#if autostartError}<p class="error">{autostartError}</p>{/if}

  <footer>
    <button onclick={onClose}>Abbrechen</button>
    <button class="primary" onclick={() => onSave($state.snapshot(draft))}>Speichern</button>
  </footer>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.5);
  }
  .dialog {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: min(640px, calc(100vw - 32px));
    max-height: calc(100vh - 32px);
    overflow: auto;
    padding: 20px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  h2 {
    margin: 0 0 8px;
    font-size: 18px;
  }
  h3 {
    margin: 18px 0 8px;
    font-size: 13px;
    font-weight: 600;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .row {
    display: flex;
    gap: 8px;
    margin-bottom: 8px;
  }
  input,
  select {
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--panel-2);
    min-width: 0;
  }
  .name {
    width: 130px;
    flex: none;
  }
  select {
    flex: 1;
  }
  .buffer {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--muted);
  }
  .buffer input {
    width: 70px;
  }
  button {
    padding: 6px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--panel-2);
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .order {
    display: flex;
    gap: 2px;
  }
  .order .icon {
    padding: 6px 8px;
  }
  .icon {
    padding: 6px 10px;
  }
  .add {
    border-style: dashed;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .error {
    color: var(--danger);
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 20px;
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
</style>
