<script>
  import UsbCableButton from "./UsbCableButton.svelte";
  import { t } from "./i18n.svelte.js";
  import { cableCounterpart, ownCable, getApps, getDevices, getWindowsAudio, isCable, setAppOutput } from "./api.js";

  /**
   * Which program plays on which device, and which programs Faderust records
   * as inputs of their own. A program on a virtual cable also reaches
   * Faderust through the cable's capture side.
   */
  let { inputs, outputs = [], gains = [], onAddInput, onClose } = $props();

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
  let defaultId = $state(null);
  getWindowsAudio()
    .then((w) => (defaultId = w.playback.find((e) => e.isDefault)?.id ?? null))
    .catch(() => {});

  let hasCable = $derived(data?.devices.some((d) => d.cable));
  // The playback end of the cable whose recording end is `c`.
  const playbackOf = (c) => {
    const own = ownCable(c.name);
    if (own === null) return c.name.replace(/output/i, "Input");
    return data?.devices.find((d) => ownCable(d.name) === own)?.name ?? "";
  };
  // Cable capture devices no input uses yet.
  // Not offered when Faderust plays on the cable's other end (that would loop).
  let unusedCables = $derived(
    cableInputs.filter(
      (c) =>
        !inputs.some((i) => i.device && c.name.toLowerCase().includes(i.device.toLowerCase())) &&
        !outputs.some((o) => o.device && playbackOf(c).toLowerCase().includes(o.device.toLowerCase())) &&
        // Nor when a program input is parked on the cable (heard twice).
        !data?.apps.some(
          (a) => isInput(a) && cableCounterpart(data.devices.find((d) => d.id === a.device)?.name ?? "", cableInputs) === c.name,
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
    !inputs.some((i) => matches(cableCounterpart(d.name, cableInputs), i.device)) &&
    !outputs.some((o) => matches(d.name, o.device));
  let quietCable = $derived(data?.devices.find(silent));
  // The input reaches an output: not muted and at least one fader up.
  const routed = (app) => {
    const row = inputs.findIndex((i) => i.device === programId(app));
    return row >= 0 && !inputs[row].muted && (gains[row] ?? []).some((g) => !g.muted && g.db > -59);
  };
  // Heard on its own device: a cable somebody listens to, a device Faderust
  // plays on too, or the Windows default. Another device (speakers while you
  // wear headphones) is not counted, Faderust cannot know whether you hear it.
  const heard = (app) => {
    const d = data.devices.find((x) => x.id === (app.device ?? defaultId));
    if (!d) return false;
    return d.cable ? !silent(d) : app.device == null || outputs.some((o) => matches(d.name, o.device));
  };
  // Recorded as an input, routed to an output and still playing somewhere
  // you hear it.
  const doubled = (app) => isInput(app) && routed(app) && heard(app);

  // Park the program on a quiet cable first and record it once it has
  // moved there: recording it mid-move can leave it without sound.
  async function addAsInput(app) {
    if (quietCable) {
      await change(app, quietCable.id);
      await new Promise((r) => setTimeout(r, 800));
    }
    await onAddInput(programId(app), app.name);
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
<div class="dialog" role="dialog" aria-modal="true" aria-label={t("programs.title")}>
  <header>
    <h2>{t("programs.title")}</h2>
    <button onclick={refresh}>{t("common.refresh")}</button>
  </header>

  {#if error}<p class="error">{error}</p>{/if}

  {#if data && !data.supported}
    <p class="note">{t("apps.unsupported")}</p>
  {:else if data}
    {#if !hasCable}
      <div class="hint">
        <strong>{t("apps.noCable")}</strong>
        {t("apps.noCableText")}
        <UsbCableButton />
      </div>
    {/if}
    {#each unusedCables as cable}
      <div class="hint">
        <span><strong>{cable.name}</strong> {t("apps.cableUnused")}</span>
        <button class="primary" onclick={() => onAddInput(cable.name)}>{t("apps.makeInput")}</button>
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
              aria-label={t("apps.outputFor", { name: app.name })}
              class:cable={data.devices.find((d) => d.id === app.device)?.cable}
            >
              <option value="">{t("apps.winDefault")}</option>
              {#each data.devices as d}
                <option value={d.id}>{d.cable ? t("apps.cableSuffix", { name: d.name }) : d.name}</option>
              {/each}
              {#if app.device && !data.devices.some((d) => d.id === app.device)}
                <option value={app.device}>{t("apps.disconnected")}</option>
              {/if}
            </select>
            {#if isInput(app)}
              <span class="is-input" title={t("apps.isInputHint")}>{t("apps.isInput")}</span>
            {:else}
              <button
                title={quietCable
                  ? t("apps.addParked", { name: quietCable.name })
                  : t("apps.add")}
                onclick={() => addAsInput(app)}>{t("apps.addButton")}</button
              >
            {/if}
          </li>
          {#if doubled(app)}
            <li class="double">
              <span>{t("apps.doubled", { name: app.name })}</span>
              {#if quietCable}
                <button class="primary" onclick={() => change(app, quietCable.id)}>{t("apps.moveTo", { name: quietCable.name })}</button>
              {:else}
                <UsbCableButton />
              {/if}
            </li>
          {/if}
        {/each}
      </ul>
    {:else}
      <p class="note">{t("apps.none")}</p>
    {/if}
    <p class="note small">
      {t("apps.note")}
    </p>
  {/if}

  <footer>
    <button onclick={onClose}>{t("common.close")}</button>
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
