<script>
  import { onMount } from "svelte";
  import OutputColumn from "./lib/OutputColumn.svelte";
  import Settings from "./lib/Settings.svelte";
  import Apps from "./lib/Apps.svelte";
  import WindowsAudio from "./lib/WindowsAudio.svelte";
  import FxDialog from "./lib/FxDialog.svelte";
  import Presets from "./lib/Presets.svelte";
  import Notices from "./lib/Notices.svelte";
  import {
    fxActive,
    getConfig,
    getVersion,
    onState,
    saveConfig,
    setGain,
    setMuted,
    windowAction,
  } from "./lib/api.js";

  let state = $state(null);
  let config = $state(null);
  let settingsOpen = $state(false);
  let appsOpen = $state(false);
  let windowsOpen = $state(false);
  let presetsOpen = $state(false);
  // Index of the input whose effects are open.
  let fxOpen = $state(null);
  let error = $state("");
  let version = $state("");
  getVersion().then((v) => (version = v));

  onMount(() => {
    let unlisten;
    onState((s) => (state = s)).then((u) => (unlisten = u));
    return () => unlisten?.();
  });

  // Fetch fresh: fader moves since the last fetch live in the gains.
  async function openSettings() {
    config = await getConfig();
    settingsOpen = true;
  }

  async function openApps() {
    config = await getConfig();
    appsOpen = true;
  }

  // A virtual cable's capture side as a new input, 0 dB on every output.
  async function addCableInput(device) {
    const next = await getConfig();
    next.inputs.push({ label: "PC", device });
    next.gains.push(next.outputs.map(() => ({ db: 0, muted: false })));
    try {
      await saveConfig(next);
      config = next;
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  async function save(next) {
    try {
      await saveConfig(next);
      config = next;
      settingsOpen = false;
      error = "";
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="app">
  <!-- Own title bar (the window has no decorations): drag on empty space,
       double-click to maximize. -->
  <header class="titlebar" data-tauri-drag-region>
    <div class="brand" data-tauri-drag-region>
      <img src="/logo.svg" alt="" width="22" height="22" data-tauri-drag-region />
      <span class="name" data-tauri-drag-region>Faderust</span>
      {#if version}<span class="version" data-tauri-drag-region>v{version}</span>{/if}
    </div>

    <div class="inputs" data-tauri-drag-region>
      {#each state?.inputs ?? [] as input, i}
        <button
          class="chip"
          class:off={!input.connected}
          title={`${input.deviceName ?? input.device} · Effekte`}
          onclick={() => (fxOpen = i)}
        >
          <span class="dot"></span>{input.label}
          {#if fxActive(input.fx)}<span class="fx">FX</span>{/if}
        </button>
      {/each}
    </div>

    <nav class="nav">
      <button onclick={() => (presetsOpen = true)} class:active={presetsOpen}>
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 2h8v12l-4-3-4 3z" /></svg>Presets
      </button>
      <button onclick={openApps} class:active={appsOpen}>
        <svg viewBox="0 0 16 16" aria-hidden="true"
          ><rect x="2" y="2" width="5" height="5" rx="1" /><rect x="9" y="2" width="5" height="5" rx="1" /><rect
            x="2"
            y="9"
            width="5"
            height="5"
            rx="1"
          /><rect x="9" y="9" width="5" height="5" rx="1" /></svg
        >Programme
      </button>
      <button onclick={() => (windowsOpen = true)} class:active={windowsOpen}>
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M2 6h3l4-3v10l-4-3H2z" /><path d="M11.5 5.5a3.5 3.5 0 0 1 0 5M13.5 3.5a6.3 6.3 0 0 1 0 9" /></svg
        >Windows
      </button>
      <button onclick={openSettings} class:active={settingsOpen}>
        <svg viewBox="0 0 16 16" aria-hidden="true"
          ><path d="M4 2v12M8 2v12M12 2v12" /><rect x="2.5" y="8" width="3" height="2.5" rx=".6" /><rect
            x="6.5"
            y="4"
            width="3"
            height="2.5"
            rx=".6"
          /><rect x="10.5" y="10" width="3" height="2.5" rx=".6" /></svg
        >Geräte
      </button>
    </nav>

    <div class="window">
      <button title="Minimieren" aria-label="Minimieren" onclick={() => windowAction("minimize")}>
        <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M1 5h8" /></svg>
      </button>
      <button title="Maximieren" aria-label="Maximieren" onclick={() => windowAction("maximize")}>
        <svg viewBox="0 0 10 10" aria-hidden="true"><rect x="1" y="1" width="8" height="8" rx="1" /></svg>
      </button>
      <button class="close" title="In den Tray" aria-label="In den Tray" onclick={() => windowAction("close")}>
        <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M1.5 1.5l7 7M8.5 1.5l-7 7" /></svg>
      </button>
    </div>
  </header>

  {#if error}<div class="error">{error}</div>{/if}

  {#if state && state.outputs.length && state.inputs.length}
    <main>
      {#each state.outputs as output, o}
        <OutputColumn
          {output}
          index={o}
          inputs={state.inputs}
          gains={state.gains}
          lanes={state.lanes}
          onGain={setGain}
          onMute={setMuted}
        />
      {/each}
    </main>
  {:else if state}
    <div class="empty">
      <p>Noch keine Geräte eingerichtet.</p>
      <button class="primary" onclick={openSettings}>Geräte einrichten</button>
    </div>
  {/if}
</div>

{#if fxOpen !== null && state?.inputs[fxOpen]}
  <FxDialog index={fxOpen} input={state.inputs[fxOpen]} onClose={() => (fxOpen = null)} />
{/if}

{#if presetsOpen}
  <Presets onError={(e) => (error = e)} onClose={() => (presetsOpen = false)} />
{/if}

<Notices />

{#if appsOpen && config}
  <Apps inputs={config.inputs} onAddInput={addCableInput} onClose={() => (appsOpen = false)} />
{/if}

{#if windowsOpen}
  <WindowsAudio onClose={() => (windowsOpen = false)} />
{/if}

{#if settingsOpen && config}
  <Settings {config} onSave={save} onClose={() => (settingsOpen = false)} />
{/if}

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }
  .titlebar {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 48px;
    padding-left: 14px;
    background: var(--panel);
    border-bottom: 1px solid var(--border);
    user-select: none;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .brand img {
    border-radius: 5px;
  }
  .name {
    font-size: 15px;
    font-weight: 650;
    letter-spacing: 0.01em;
  }
  .version {
    padding: 1px 7px;
    border: 1px solid var(--border);
    border-radius: 999px;
    font-size: 11px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .nav {
    display: flex;
    gap: 2px;
    padding: 3px;
    background: var(--panel-2);
    border: 1px solid var(--border);
    border-radius: 10px;
  }
  .nav button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 11px;
    border: 0;
    border-radius: 7px;
    background: none;
    color: var(--muted);
    font-weight: 600;
  }
  .nav button:hover,
  .nav button.active {
    background: var(--panel);
    color: var(--text);
  }
  .nav svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.5;
    stroke-linejoin: round;
  }
  .nav button.active svg,
  .nav button:hover svg {
    stroke: var(--copper);
  }
  .window {
    display: flex;
    align-self: stretch;
  }
  .window button {
    display: grid;
    place-items: center;
    width: 46px;
    padding: 0;
    border: 0;
    border-radius: 0;
    background: none;
    color: var(--muted);
  }
  .window button:hover {
    background: var(--panel-2);
    color: var(--text);
  }
  .window button.close:hover {
    background: #c42b1c;
    color: #fff;
  }
  .window svg {
    width: 10px;
    height: 10px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.1;
  }
  .inputs {
    display: flex;
    align-self: stretch;
    align-items: center;
    gap: 6px;
    flex: 1;
    overflow: hidden;
    margin-left: 8px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 10px;
    border: 1px solid var(--border);
    border-radius: 999px;
    font-size: 12px;
    white-space: nowrap;
  }
  .chip {
    background: none;
    cursor: pointer;
  }
  .chip:hover {
    border-color: var(--copper);
  }
  .chip .fx {
    font-size: 10px;
    font-weight: 700;
    color: var(--copper);
  }
  .chip .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--teal);
  }
  .chip.off {
    color: var(--muted);
  }
  .chip.off .dot {
    background: var(--border);
  }
  button {
    padding: 6px 14px;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--panel-2);
  }
  .primary {
    background: var(--copper);
    border-color: var(--copper);
    color: #fff;
  }
  main {
    flex: 1;
    display: flex;
    gap: 16px;
    padding: 16px;
    overflow: auto;
    min-height: 0;
    align-items: stretch;
  }
  .empty {
    flex: 1;
    display: grid;
    place-content: center;
    justify-items: center;
    color: var(--muted);
  }
  .error {
    margin: 12px 16px 0;
    padding: 8px 12px;
    border: 1px solid var(--danger);
    border-radius: 8px;
    color: var(--danger);
  }
</style>
