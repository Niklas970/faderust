<script>
  import { cableCounterpart, getApps, getDevices, isCable, openCablePage, setAppOutput } from "./api.js";

  /**
   * Which program plays on which device, and which programs Faderust records
   * as inputs of their own. A program on a virtual cable also reaches
   * Faderust through the cable's capture side.
   */
  let { inputs, outputs = [], onAddInput, onClose } = $props();

  let data = $state(null);
  let cableInputs = $state([]);
  let error = $state("");

  async function refresh() {
    try {
      data = await getApps();
      error = "";
    } catch (e) {
      error = String(e);
    }
  }
  refresh();
  getDevices().then((d) => (cableInputs = d.inputs.filter((i) => isCable(i.name))));

  let hasCable = $derived(data?.devices.some((d) => d.cable));
  // Cable capture devices no input uses yet.
  // Not offered when Faderust plays on the cable's other end (that would loop).
  let unusedCables = $derived(
    cableInputs.filter(
      (c) =>
        !inputs.some((i) => i.device && c.name.toLowerCase().includes(i.device.toLowerCase())) &&
        !outputs.some((o) => o.device && c.name.replace(/output/i, "Input").toLowerCase().includes(o.device.toLowerCase())) &&
        // Nor when a program input is parked on the cable (heard twice).
        !data?.apps.some(
          (a) => isInput(a) && cableCounterpart(data.devices.find((d) => d.id === a.device)?.name ?? "") === c.name,
        ),
    ),
  );

  // Program inputs record the program itself, see engine apps::capture.
  const programId = (app) => `app:${app.exe.toLowerCase()}`;
  const isInput = (app) => inputs.some((i) => i.device === programId(app));

  // A cable nobody hears: Faderust neither records its other end nor plays
  // on it. A program input parked there is heard only through its fader.
  const matches = (name, pattern) => !!pattern && name.toLowerCase().includes(pattern.toLowerCase());
  const silent = (d) =>
    !!d?.cable &&
    !inputs.some((i) => matches(cableCounterpart(d.name), i.device)) &&
    !outputs.some((o) => matches(d.name, o.device));
  let quietCable = $derived(data?.devices.find(silent));
  // Recorded as an input and still playing somewhere you hear it.
  const doubled = (app) => isInput(app) && !silent(data.devices.find((d) => d.id === app.device));

  // Record the program and park its own output on a quiet cable at once.
  async function addAsInput(app) {
    await onAddInput(programId(app), app.name);
    if (quietCable) await change(app, quietCable.id);
  }

  async function change(app, device) {
    try {
      await setAppOutput($state.snapshot(app.pids), device || null);
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="backdrop" role="presentation" onclick={onClose}></div>
<div class="dialog" role="dialog" aria-modal="true" aria-label="Programme">
  <header>
    <h2>Programme</h2>
    <button onclick={refresh}>Aktualisieren</button>
  </header>

  {#if error}<p class="error">{error}</p>{/if}

  {#if data && !data.supported}
    <p class="note">Ausgabegeräte pro Programm gibt es nur unter Windows.</p>
  {:else if data}
    {#if !hasCable}
      <div class="hint">
        <strong>Kein virtuelles Kabel gefunden.</strong>
        Mit „+ Eingang“ nimmt Faderust ein Programm direkt auf. Damit es dann nicht zusätzlich über die Boxen
        läuft, braucht es ein Gerät, das du nicht hörst: Installiere das kostenlose VB-Cable und starte den PC neu.
        <button onclick={() => openCablePage()}>Virtuelles Kabel einrichten</button>
      </div>
    {/if}
    {#each unusedCables as cable}
      <div class="hint">
        <span><strong>{cable.name}</strong> ist noch kein Eingang. Programme, die auf das Kabel spielen, hörst du erst, wenn es einer ist.</span>
        <button class="primary" onclick={() => onAddInput(cable.name)}>Als Eingang anlegen</button>
      </div>
    {/each}

    {#if data.apps.length}
      <ul>
        {#each data.apps as app (app.exe)}
          <li>
            <span class="name" title={app.exe}>{app.name}</span>
            <select
              value={app.device ?? ""}
              onchange={(e) => change(app, e.currentTarget.value)}
              aria-label={`Ausgabegerät für ${app.name}`}
              class:cable={data.devices.find((d) => d.id === app.device)?.cable}
            >
              <option value="">Windows-Standard</option>
              {#each data.devices as d}
                <option value={d.id}>{d.cable ? `${d.name} · Kabel` : d.name}</option>
              {/each}
              {#if app.device && !data.devices.some((d) => d.id === app.device)}
                <option value={app.device}>Gerät nicht verbunden</option>
              {/if}
            </select>
            {#if isInput(app)}
              <span class="is-input" title="Faderust nimmt das Programm als eigenen Eingang auf">Eingang</span>
            {:else}
              <button
                title={quietCable
                  ? `Programm als eigenen Eingang aufnehmen und auf ${quietCable.name} stellen, damit du es nicht doppelt hörst`
                  : "Programm als eigenen Eingang mit Fader und Effekten aufnehmen"}
                onclick={() => addAsInput(app)}>+ Eingang</button
              >
            {/if}
          </li>
          {#if doubled(app)}
            <li class="double">
              <span>Du hörst {app.name} doppelt: einmal über den Fader, einmal über sein eigenes Gerät.</span>
              {#if quietCable}
                <button class="primary" onclick={() => change(app, quietCable.id)}>Auf {quietCable.name} stellen</button>
              {:else}
                <button onclick={() => openCablePage()}>Virtuelles Kabel einrichten</button>
              {/if}
            </li>
          {/if}
        {/each}
      </ul>
    {:else}
      <p class="note">Gerade spielt kein Programm Ton ab. Starte die Wiedergabe und tippe auf „Aktualisieren“.</p>
    {/if}
    <p class="note small">
      Windows merkt sich die Zuordnung pro Programm. Manche Programme übernehmen sie erst nach einem Neustart.
    </p>
  {/if}

  <footer>
    <button onclick={onClose}>Schließen</button>
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
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }
  h2 {
    margin: 0;
    font-size: 18px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }
  .name {
    width: 150px;
    flex: none;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  select {
    flex: 1;
    min-width: 0;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--panel-2);
  }
  .is-input {
    flex: none;
    padding: 5px 10px;
    font-size: 12px;
    font-weight: 600;
    color: var(--signal);
    border: 1px solid var(--signal);
    border-radius: var(--radius-sm);
  }
  select.cable {
    border-color: var(--signal);
  }
  li.double {
    margin: -4px 0 12px;
    padding: 8px 10px;
    border: 1px solid var(--danger);
    border-radius: var(--radius-sm);
    font-size: 13px;
  }
  li.double span {
    flex: 1;
  }
  .hint {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
    padding: 10px 12px;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    line-height: 1.4;
  }
  .hint:not(:has(button)) {
    display: block;
  }
  .url {
    font-family: ui-monospace, monospace;
    user-select: all;
  }
  .note {
    color: var(--muted);
  }
  .small {
    font-size: 12px;
    margin-top: 16px;
  }
  .error {
    color: var(--danger);
  }
  button {
    padding: 6px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--panel-2);
    white-space: nowrap;
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    margin-top: 16px;
  }
</style>
