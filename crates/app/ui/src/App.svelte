<script>
  import { onMount } from "svelte";
  import OutputColumn from "./lib/OutputColumn.svelte";
  import Settings from "./lib/Settings.svelte";
  import Apps from "./lib/Apps.svelte";
  import FxDialog from "./lib/FxDialog.svelte";
  import Presets from "./lib/Presets.svelte";
  import Notices from "./lib/Notices.svelte";
  import { fxActive, getConfig, onState, saveConfig, setGain, setMuted } from "./lib/api.js";

  let state = $state(null);
  let config = $state(null);
  let settingsOpen = $state(false);
  let appsOpen = $state(false);
  let presetsOpen = $state(false);
  // Index of the input whose effects are open.
  let fxOpen = $state(null);
  let error = $state("");

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
  <header class="top">
    <img src="/logo.svg" alt="" width="28" height="28" />
    <h1>Faderust</h1>
    <div class="inputs">
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
    <button onclick={() => (presetsOpen = true)}>Presets</button>
    <button onclick={openApps}>Programme</button>
    <button class="settings" onclick={openSettings}>Geräte</button>
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

{#if settingsOpen && config}
  <Settings {config} onSave={save} onClose={() => (settingsOpen = false)} />
{/if}

<style>
  .app {
    display: grid;
    grid-template-rows: auto auto 1fr;
    height: 100vh;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }
  h1 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }
  .inputs {
    display: flex;
    gap: 6px;
    flex: 1;
    overflow: hidden;
    margin-left: 12px;
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
    display: flex;
    gap: 16px;
    padding: 16px;
    overflow: auto;
    min-height: 0;
    align-items: stretch;
  }
  .empty {
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
