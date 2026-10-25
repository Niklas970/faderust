<script>
  import { onMount, untrack } from "svelte";
  import {
    cableCounterpart,
    getAutostart,
    getDevices,
    getStreamDeck,
    installStreamDeck,
    getWindowsAudio,
    isCable,
    openCablePage,
    setAutostart,
    setDefaultDevice,
  } from "./api.js";

  /** Device setup: which inputs and outputs, their names and buffers. */
  let { config, onSave, onClose } = $props();

  // Edit a copy; nothing changes until Save.
  let draft = $state(untrack(() => ({ gains: [], ...$state.snapshot(config) })));
  let devices = $state({ inputs: [], outputs: [] });
  getDevices().then((d) => (devices = d));

  // Stream Deck plugin: looked at while this dialog is open, since the plugin
  // may connect only now.
  let deck = $state(null);
  let deckError = $state("");
  const lookAtDeck = () => getStreamDeck().then((d) => (deck = d), () => {});
  lookAtDeck();
  onMount(() => {
    const timer = setInterval(lookAtDeck, 3000);
    return () => clearInterval(timer);
  });
  async function updateDeck() {
    try {
      await installStreamDeck();
      deckError = "";
    } catch (e) {
      deckError = String(e);
    }
  }
  const deckLabel = $derived.by(() => {
    if (!deck?.bundled) return "";
    if (!deck.connected) return "Stream Deck nicht verbunden. Plugin einrichten oder neu installieren:";
    if (deck.update) return `Plugin ${deck.installed ?? "(alt)"} → ${deck.bundled} verfügbar`;
    return `Plugin ${deck.installed} ist aktuell`;
  });

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
    draft.outputs.push({ label: `Ausgang ${draft.outputs.length + 1}`, device: devices.outputs[0].name, bufferMs: 60, releaseSecs: 30 });
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

  // "Programm-Mikro": an output on a virtual cable, so Discord, Teams & co.
  // get Faderust's microphone mix by recording the cable's other end.
  const matches = (name, pattern) => !!pattern && name.toLowerCase().includes(pattern.toLowerCase());
  // A cable's playback end that is no output yet and whose recording end is
  // no input (that would loop).
  let micCable = $derived(
    devices.outputs.find(
      (d) =>
        isCable(d.name) &&
        /input/i.test(d.name) &&
        !draft.outputs.some((o) => matches(d.name, o.device)) &&
        !draft.inputs.some((i) => matches(cableCounterpart(d.name), i.device)),
    ),
  );
  let hasCable = $derived(devices.outputs.some((d) => isCable(d.name)));
  // Recording device the programs pick, once the output is added.
  let micDevice = $state(null);
  let micDefault = $state(true);
  let micError = $state("");

  function addProgramMic() {
    // Read before the push: afterwards the cable is taken and `micCable` moves on.
    const cable = micCable.name;
    draft.outputs.push({ label: "Programm-Mikro", device: cable, bufferMs: 30, releaseSecs: 30 });
    // The first input (usually the microphone) at 0 dB, everything else muted.
    for (const m of matrices()) m.forEach((row, i) => row.push({ db: 0, muted: i !== 0 }));
    micDevice = cableCounterpart(cable);
  }

  async function save() {
    if (micDevice && micDefault) {
      try {
        const rec = (await getWindowsAudio()).recording?.find((d) => d.name === micDevice);
        if (rec) await setDefaultDevice(rec.id, true);
      } catch (e) {
        micError = `Standardgerät nicht gesetzt: ${e}`;
      }
    }
    onSave($state.snapshot(draft));
  }

  function selected(list, current) {
    return (
      list.find((d) => d.id === current)?.name ??
      list.find((d) => d.name.toLowerCase().includes(current.toLowerCase()))?.name ??
      current
    );
  }

  // Release of Bluetooth outputs while silent (wired ones stay open anyway).
  const RELEASE = [
    { secs: 0, name: "Nie frei" },
    { secs: 5, name: "Frei nach 5 s" },
    { secs: 30, name: "Frei nach 30 s" },
    { secs: 120, name: "Frei nach 2 min" },
  ];
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
      <select
        class="release"
        value={output.releaseSecs ?? 30}
        onchange={(e) => (output.releaseSecs = Number(e.currentTarget.value))}
        aria-label="Bluetooth bei Stille freigeben"
        title="Bluetooth-Kopfhörer bei Stille freigeben, damit sie zum Handy wechseln können (Multipoint). Beim Wiederstart braucht Bluetooth etwa 1 s."
      >
        {#each RELEASE as r}<option value={r.secs}>{r.name}</option>{/each}
      </select>
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
  {#if micDevice}
    <div class="hint">
      <p>
        <strong>Programm-Mikro angelegt.</strong> Wähle in Discord, Teams usw. als Mikrofon
        <strong>{micDevice}</strong>. Die Spalte „Programm-Mikro“ bestimmt, was dort ankommt.
      </p>
      <label class="check">
        <input type="checkbox" bind:checked={micDefault} />
        Als Windows-Mikrofon für Anrufe setzen
      </label>
    </div>
  {:else if micCable}
    <button class="add" onclick={addProgramMic} title="Ausgang auf einem virtuellen Kabel, den Programme als Mikrofon nutzen">
      + Programm-Mikro
    </button>
  {:else}
    <div class="hint">
      <p>
        {hasCable
          ? "Für ein Programm-Mikro (Discord, Teams …) braucht es ein freies virtuelles Kabel. Alle Kabel sind schon belegt."
          : "Für ein Programm-Mikro (Discord, Teams …) braucht es ein virtuelles Kabel."}
        Installiere das kostenlose VB-Cable (für ein zweites Kabel VB-Cable A+B) und starte den PC neu.
      </p>
      <button onclick={() => openCablePage()}>Virtuelles Kabel einrichten</button>
    </div>
  {/if}
  {#if micError}<p class="error">{micError}</p>{/if}

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

  {#if deck?.bundled}
    <h3>Stream Deck</h3>
    <div class="deck">
      <span>{deckLabel}</span>
      <button class:primary={deck.update} onclick={updateDeck}>{deck.connected ? "Plugin aktualisieren" : "Plugin installieren"}</button>
    </div>
    {#if deckError}<p class="error">{deckError}</p>{/if}
  {/if}

  <footer>
    <button onclick={onClose}>Abbrechen</button>
    <button class="primary" onclick={save}>Speichern</button>
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
    width: min(760px, calc(100vw - 32px));
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
  select.release {
    flex: none;
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
  .add + .add {
    margin-left: 6px;
  }
  .hint {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 12px;
    margin-top: 10px;
    padding: 10px 12px;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    line-height: 1.4;
  }
  .hint p {
    flex: 1 1 300px;
    margin: 0;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .deck {
    display: flex;
    align-items: center;
    gap: 12px;
    justify-content: space-between;
    font-size: 13px;
    color: var(--muted);
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
