<script>
  import { getApps, getDevices, isCable, setAppOutput, setAppVolume } from "./api.js";
  import Volume from "./Volume.svelte";

  /**
   * Which program plays on which device. A program on a virtual cable
   * reaches Faderust through the cable's capture side, as its own input.
   */
  let { inputs, onAddInput, onClose } = $props();

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
  let unusedCables = $derived(
    cableInputs.filter(
      (c) => !inputs.some((i) => i.device && c.name.toLowerCase().includes(i.device.toLowerCase())),
    ),
  );

  async function volume(app, v, m) {
    try {
      await setAppVolume($state.snapshot(app.pids), v, m);
    } catch (e) {
      error = String(e);
    }
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
        Damit ein Programm einen eigenen Fader bekommt, braucht es ein virtuelles Kabel. Installiere das
        kostenlose VB-Cable von <span class="url">vb-audio.com/Cable</span> und starte den PC neu.
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
            <div class="vol">
              <Volume
                volume={app.volume}
                muted={app.muted}
                label={app.name}
                onChange={(v, m) => volume(app, v, m)}
              />
            </div>
          </li>
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
    width: min(760px, calc(100vw - 32px));
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
  .vol {
    width: 180px;
    flex: none;
  }
  .name {
    width: 130px;
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
  select.cable {
    border-color: var(--teal);
  }
  .hint {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
    padding: 10px 12px;
    border: 1px solid var(--copper);
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
    background: var(--copper);
    border-color: var(--copper);
    color: #fff;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    margin-top: 16px;
  }
</style>
