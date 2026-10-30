<script>
  import { onMount, untrack } from "svelte";
  import { t } from "./i18n.svelte.js";
  import { deviceLabel } from "./names.js";
  import UsbCableButton from "./UsbCableButton.svelte";
  import Log from "./Log.svelte";
  import Selftest from "./Selftest.svelte";
  import {
    cableCounterpart,
    ownCable,
    getAutostart,
    getDevices,
    getStreamDeck,
    getUsbCable,
    setUsbCable,
    setUsbCableCount,
    setupUsbCable,
    removeUsbCable,
    installStreamDeck,
    getWindowsAudio,
    isCable,
    getConfig,
    setAutostart,
    setDefaultDevice,
  } from "./api.js";

  /** Device setup: which inputs and outputs, their names and buffers. */
  let { config, onApply, onClose } = $props();

  // Edit a copy; changes are saved by themselves a moment after the last one.
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
    if (deck.refused) return t("deck.refused");
    if (!deck.connected) return t("deck.disconnected");
    if (deck.update) return t("deck.available", { from: deck.installed ?? t("deck.old"), to: deck.bundled });
    return t("deck.current", { version: deck.installed });
  });

  // Faderust's own USB cable; applies right away, like autostart.
  let usb = $state(null);
  let usbError = $state("");
  let usbBusy = $state(false);
  const lookAtUsb = () => getUsbCable().then((u) => (usb = u), () => {});
  lookAtUsb();
  onMount(() => {
    const timer = setInterval(lookAtUsb, 2000);
    return () => clearInterval(timer);
  });
  async function usbAction(action) {
    usbBusy = true;
    try {
      await action();
      usbError = "";
    } catch (e) {
      usbError = String(e);
    }
    usbBusy = false;
    lookAtUsb();
  }
  const setupUsb = () =>
    usbAction(async () => {
      await setupUsbCable();
      await setUsbCable(true);
    });

  let selftestOpen = $state(false);
  let logOpen = $state(false);

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
  // Whether the matrices were reshaped since the last save. If not, the
  // saved ones are newer than the copy here (fader moves while this is open).
  let reshaped = false;

  // Program inputs ("app:…") belong to the program dialog, not to this list.
  const isProgram = (input) => input.device.startsWith("app:");
  // Nearest device input in the given direction, or -1.
  function neighbor(i, by) {
    for (let j = i + by; j >= 0 && j < draft.inputs.length; j += by) if (!isProgram(draft.inputs[j])) return j;
    return -1;
  }

  function addInput() {
    reshaped = true;
    draft.inputs.push({ label: t("settings.newInput", { n: draft.inputs.length + 1 }), device: devices.inputs[0].name });
    for (const m of matrices()) m.push(draft.outputs.map(() => ({ db: 0, muted: false })));
  }
  function removeInput(i) {
    reshaped = true;
    draft.inputs.splice(i, 1);
    for (const m of matrices()) m.splice(i, 1);
  }
  function moveInput(i, by) {
    reshaped = true;
    swap(draft.inputs, i, i + by);
    for (const m of matrices()) if (m[i] && m[i + by]) swap(m, i, i + by);
  }
  function addOutput() {
    reshaped = true;
    draft.outputs.push({ label: t("settings.newOutput", { n: draft.outputs.length + 1 }), device: devices.outputs[0].name, bufferMs: 60, releaseSecs: 30 });
    for (const m of matrices()) for (const row of m) row.push({ db: 0, muted: false });
  }
  function removeOutput(o) {
    reshaped = true;
    draft.outputs.splice(o, 1);
    for (const m of matrices()) for (const row of m) row.splice(o, 1);
  }
  function moveOutput(o, by) {
    reshaped = true;
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
        (/input/i.test(d.name) || ownCable(d.name) !== null) &&
        !draft.outputs.some((o) => matches(d.name, o.device)) &&
        !draft.inputs.some((i) => matches(cableCounterpart(d.name, devices.inputs), i.device)),
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
    draft.outputs.push({ label: t("settings.programMic"), device: cable, bufferMs: 30, releaseSecs: 30 });
    reshaped = true;
    // The first input (usually the microphone) at 0 dB, everything else muted.
    for (const m of matrices()) m.forEach((row, i) => row.push({ db: 0, muted: i !== 0 }));
    micDevice = cableCounterpart(cable, devices.inputs);
  }

  let saved = JSON.stringify($state.snapshot(draft));
  let timer;
  let micApplied = false;

  async function autosave() {
    const json = JSON.stringify($state.snapshot(draft));
    const next = $state.snapshot(draft);
    try {
      if (!reshaped) {
        const fresh = await getConfig();
        if (fresh.inputs.length === next.inputs.length && fresh.outputs.length === next.outputs.length) {
          next.gains = fresh.gains;
          next.presets = fresh.presets;
        }
      }
      if (micDevice && micDefault && !micApplied) {
        try {
          const rec = (await getWindowsAudio()).recording?.find((d) => d.name === micDevice);
          if (rec) await setDefaultDevice(rec.id, true);
          micApplied = true;
          micError = "";
        } catch (e) {
          micError = t("settings.defaultFailed", { error: e });
        }
      }
      if (await onApply(next)) {
        saved = json;
        reshaped = false;
      }
    } catch (e) {
      micError = String(e);
    }
  }

  $effect(() => {
    const json = JSON.stringify($state.snapshot(draft));
    void micDefault;
    if (json === saved && (micApplied || !micDevice)) return;
    timer = setTimeout(autosave, 800);
    return () => clearTimeout(timer);
  });

  // Whatever is still waiting goes out before the dialog closes.
  async function close() {
    clearTimeout(timer);
    if (JSON.stringify($state.snapshot(draft)) !== saved || (micDevice && !micApplied)) await autosave();
    onClose();
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
    { secs: 0, name: () => t("release.never") },
    { secs: 5, name: () => t("release.5") },
    { secs: 30, name: () => t("release.30") },
    { secs: 120, name: () => t("release.120") },
  ];
</script>

<div class="backdrop" role="presentation" onclick={close}></div>
<div class="dialog" role="dialog" aria-modal="true" aria-label={t("settings.title")}>
  <h2>{t("settings.title")}</h2>

  <h3>{t("settings.inputs")}</h3>
  {#each draft.inputs as input, i}
    {#if !isProgram(input)}
    <div class="row">
      <input class="name" bind:value={input.label} aria-label={t("settings.name")} />
      <select
        value={selected(devices.inputs, input.device)}
        onchange={(e) => (input.device = e.currentTarget.value)}
        aria-label={t("settings.device")}
      >
        {#each options(devices.inputs, input.device) as name}<option value={name}>{deviceLabel(name)}</option>{/each}
      </select>
      <span class="order">
        <button class="icon" title={t("settings.up")} aria-label={t("settings.up")} disabled={neighbor(i, -1) < 0} onclick={() => moveInput(i, neighbor(i, -1) - i)}
          >↑</button
        >
        <button
          class="icon"
          title={t("settings.down")}
          aria-label={t("settings.down")}
          disabled={neighbor(i, 1) < 0}
          onclick={() => moveInput(i, neighbor(i, 1) - i)}>↓</button
        >
      </span>
      <button class="icon" title={t("settings.remove")} onclick={() => removeInput(i)}>✕</button>
    </div>
    {/if}
  {/each}
  <button
    class="add"
    disabled={!devices.inputs.length}
    onclick={addInput}
  >
    {t("settings.addInput")}
  </button>

  <h3>{t("settings.outputs")}</h3>
  {#each draft.outputs as output, o}
    <div class="row">
      <input class="name" bind:value={output.label} aria-label={t("settings.name")} />
      <select
        value={selected(devices.outputs, output.device)}
        onchange={(e) => (output.device = e.currentTarget.value)}
        aria-label={t("settings.device")}
      >
        {#each options(devices.outputs, output.device) as name}<option value={name}>{deviceLabel(name)}</option>{/each}
      </select>
      <label class="buffer" title={t("settings.buffer")}>
        <input type="number" min="10" max="500" step="10" bind:value={output.bufferMs} />
        ms
      </label>
      <select
        class="release"
        value={output.releaseSecs ?? 30}
        onchange={(e) => (output.releaseSecs = Number(e.currentTarget.value))}
        aria-label={t("settings.releaseLabel")}
        title={t("settings.releaseHint")}
      >
        {#each RELEASE as r}<option value={r.secs}>{r.name()}</option>{/each}
      </select>
      <span class="order">
        <button class="icon" title={t("settings.left")} aria-label={t("settings.left")} disabled={o === 0} onclick={() => moveOutput(o, -1)}
          >←</button
        >
        <button
          class="icon"
          title={t("settings.right")}
          aria-label={t("settings.right")}
          disabled={o === draft.outputs.length - 1}
          onclick={() => moveOutput(o, 1)}>→</button
        >
      </span>
      <button class="icon" title={t("settings.remove")} onclick={() => removeOutput(o)}>✕</button>
    </div>
  {/each}
  <button
    class="add"
    disabled={!devices.outputs.length}
    onclick={addOutput}
  >
    {t("settings.addOutput")}
  </button>
  {#if micDevice}
    <div class="hint">
      <p>
        <strong>{t("settings.micCreated")}</strong>
        {t("settings.micUse", { device: micDevice })}
      </p>
      <label class="check">
        <input type="checkbox" bind:checked={micDefault} />
        {t("settings.micDefault")}
      </label>
    </div>
  {:else if micCable}
    <button class="add" onclick={addProgramMic} title={t("settings.micAddHint")}>
      {t("settings.micAdd")}
    </button>
  {:else}
    <div class="hint">
      <p>
        {hasCable
          ? t("settings.micNoFreeCable")
          : t("settings.micNoCable")}
      </p>
      <UsbCableButton />
    </div>
  {/if}
  {#if micError}<p class="error">{micError}</p>{/if}

  <h3>{t("settings.start")}</h3>
  <label class="check">
    <input
      type="checkbox"
      checked={autostart ?? false}
      disabled={autostart === null}
      onchange={(e) => toggleAutostart(e.currentTarget.checked)}
    />
    {t("settings.autostart")}
  </label>
  {#if autostartError}<p class="error">{autostartError}</p>{/if}

  {#if usb?.supported}
    <h3>{t("usbcable.title")}</h3>
    {#if !usb.installed}
      <div class="deck">
        <span>{t("usbcable.hint")}</span>
        <button disabled={usbBusy} onclick={setupUsb}>
          {usbBusy ? t("usbcable.working") : t("usbcable.setup")}
        </button>
      </div>
    {/if}
    <label class="check">
      <input
        type="checkbox"
        checked={usb.running}
        disabled={usbBusy || !usb.installed}
        onchange={(e) => usbAction(() => setUsbCable(e.currentTarget.checked))}
      />
      {t("usbcable.enable")}
    </label>
    <label class="check">
      {t("usbcable.count")}
      <select
        value={usb.count}
        disabled={usbBusy}
        onchange={(e) => usbAction(() => setUsbCableCount(Number(e.currentTarget.value)))}
      >
        {#each [1, 2, 3, 4] as n}<option value={n}>{n}</option>{/each}
      </select>
    </label>
    {#if usb.installed}
      <button class="remove" disabled={usbBusy} title={t("usbcable.removeHint")} onclick={() => usbAction(removeUsbCable)}>
        {t("usbcable.remove")}
      </button>
    {/if}
    {#if usb.running}
      <p class="note">
        {#each usb.attached as on, i}
          {t(on ? "usbcable.cableOn" : "usbcable.cableWaiting", { name: i ? `Faderust Cable ${i + 1}` : "Faderust Cable" })}<br />
        {/each}
      </p>
    {/if}
    {#if usbError}<p class="error">{usbError}</p>{/if}
  {/if}

  {#if deck?.bundled}
    <h3>Stream Deck</h3>
    <div class="deck">
      <span>{deckLabel}</span>
      <button class:primary={deck.update} onclick={updateDeck}>{deck.connected || deck.refused ? t("deck.update") : t("deck.install")}</button>
    </div>
    {#if deckError}<p class="error">{deckError}</p>{/if}
  {/if}

  <h3>{t("selftest.title")}</h3>
  <div class="deck">
    <span>{t("selftest.hint")}</span>
    <button onclick={() => (selftestOpen = true)}>{t("selftest.start")}</button>
    <button onclick={() => (logOpen = true)} title={t("log.hint")}>{t("nav.log")}</button>
  </div>

  <footer>
    <button class="primary" onclick={close}>{t("common.close")}</button>
  </footer>
</div>

{#if logOpen}
  <Log onSelftest={() => (selftestOpen = true)} onClose={() => (logOpen = false)} />
{/if}

{#if selftestOpen}
  <Selftest onClose={() => (selftestOpen = false)} />
{/if}

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
    color: var(--text);
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
  .remove {
    margin-top: 6px;
  }
  .check select {
    width: auto;
  }
  .note {
    color: var(--muted);
    margin: 4px 0 0;
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
