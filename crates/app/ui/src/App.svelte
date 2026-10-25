<script>
  import { onMount } from "svelte";
  import StripView from "./lib/StripView.svelte";
  import MatrixView from "./lib/MatrixView.svelte";
  import Settings from "./lib/Settings.svelte";
  import Apps from "./lib/Apps.svelte";
  import WindowsAudio from "./lib/WindowsAudio.svelte";
  import FxDialog from "./lib/FxDialog.svelte";
  import Presets from "./lib/Presets.svelte";
  import Notices from "./lib/Notices.svelte";
  import Theme from "./lib/Theme.svelte";
  import {
    checkUpdate,
    installUpdate,
    getConfig,
    getVersion,
    onState,
    saveConfig,
    setGain,
    setMasterGain,
    setMasterMuted,
    setInputMuted,
    setMuted,
    windowAction,
  } from "./lib/api.js";

  let state = $state(null);
  let config = $state(null);
  let settingsOpen = $state(false);
  let appsOpen = $state(false);
  let windowsOpen = $state(false);
  let presetsOpen = $state(false);
  let themeOpen = $state(false);
  // Index of the input whose effects are open.
  let fxOpen = $state(null);
  let error = $state("");

  // "strips" (Voicemeeter-style desk) or "matrix", remembered per machine.
  const VIEW_KEY = "faderust.view";
  let view = $state(readView());
  function readView() {
    try {
      return localStorage.getItem(VIEW_KEY) === "matrix" ? "matrix" : "strips";
    } catch {
      return "strips";
    }
  }
  function setView(v) {
    view = v;
    try {
      localStorage.setItem(VIEW_KEY, v);
    } catch {
      // Remembered for this session only.
    }
  }
  // Where the desk shows the outputs: "left" (default) or "right".
  const SIDE_KEY = "faderust.outputs";
  let side = $state(readSide());
  function readSide() {
    try {
      return localStorage.getItem(SIDE_KEY) === "right" ? "right" : "left";
    } catch {
      return "left";
    }
  }
  function setSide(v) {
    side = v;
    try {
      localStorage.setItem(SIDE_KEY, v);
    } catch {
      // Remembered for this session only.
    }
  }
  // Program strips silent this long (seconds) are hidden; 0 shows them all.
  const HIDE_KEY = "faderust.hideSilent";
  let hideAfter = $state(readHide());
  function readHide() {
    try {
      const v = Number(localStorage.getItem(HIDE_KEY) ?? 120);
      return Number.isFinite(v) && v >= 0 ? v : 120;
    } catch {
      return 120;
    }
  }
  function setHide(v) {
    hideAfter = v;
    try {
      localStorage.setItem(HIDE_KEY, String(v));
    } catch {
      // Remembered for this session only.
    }
  }
  let version = $state("");
  getVersion().then((v) => (version = v));

  // Look for a new release shortly after start, then every half hour and
  // when the window comes back (Faderust often runs for days in the tray).
  // Offline or rate-limited is fine; the next look tries again.
  let update = $state(null);
  let updating = $state(null); // download progress 0…1 while installing
  const RECHECK_MS = 30 * 60 * 1000;
  const REFOCUS_MS = 5 * 60 * 1000;
  let lastCheck = 0;
  async function lookForUpdate() {
    if (updating !== null) return;
    lastCheck = Date.now();
    try {
      update = (await checkUpdate()) ?? update;
    } catch {
      // Try again next time.
    }
  }
  onMount(() => {
    const first = setTimeout(lookForUpdate, 3000);
    const every = setInterval(lookForUpdate, RECHECK_MS);
    const back = () => {
      if (document.visibilityState === "visible" && Date.now() - lastCheck > REFOCUS_MS) lookForUpdate();
    };
    document.addEventListener("visibilitychange", back);
    window.addEventListener("focus", back);
    return () => {
      clearTimeout(first);
      clearInterval(every);
      document.removeEventListener("visibilitychange", back);
      window.removeEventListener("focus", back);
    };
  });

  // A click on the version looks right away and says what it found.
  let checkNote = $state("");
  let noteTimer;
  async function checkNow() {
    if (updating !== null || checkNote === "Suche …") return;
    checkNote = "Suche …";
    lastCheck = Date.now();
    try {
      update = (await checkUpdate()) ?? null;
      checkNote = update ? "" : "Aktuell";
    } catch (e) {
      checkNote = "Keine Verbindung";
      error = `Update-Suche fehlgeschlagen: ${e}`;
    }
    clearTimeout(noteTimer);
    if (checkNote) noteTimer = setTimeout(() => (checkNote = ""), 3000);
  }

  async function runUpdate() {
    updating = 0;
    try {
      await installUpdate((p) => (updating = p));
    } catch (e) {
      error = `Update fehlgeschlagen: ${e}`;
      updating = null;
    }
  }

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
  async function addInput(device, label = "PC") {
    const next = await getConfig();
    next.inputs.push({ label, device });
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
      {#if version}
        <button class="version" onclick={checkNow} title="Nach Updates suchen">
          v{version}{#if checkNote}<span class="check-note">· {checkNote}</span>{/if}
        </button>
      {/if}
      {#if update}
        <button class="update" onclick={runUpdate} disabled={updating !== null} title="Herunterladen und installieren">
          {updating === null ? `Update auf ${update.version}` : `Lädt … ${Math.round(updating * 100)} %`}
        </button>
      {/if}
    </div>

    <div class="spacer" data-tauri-drag-region></div>


    <nav class="nav">
      <button onclick={() => (presetsOpen = true)} class:active={presetsOpen}>
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 2h8v12l-4-3-4 3z" /></svg>Presets
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
      <button onclick={() => (themeOpen = true)} class:active={themeOpen} title="Farben und Darstellung">
        <svg viewBox="0 0 16 16" aria-hidden="true"
          ><path d="M8 2a6 6 0 1 0 0 12c1 0 1.5-.6 1.5-1.3 0-.9-.8-1.2-.8-2 0-.7.6-1.2 1.4-1.2H12a2 2 0 0 0 2-2C14 4.4 11.3 2 8 2z" /><circle
            cx="5"
            cy="7"
            r=".9"
          /><circle cx="8" cy="5" r=".9" /><circle cx="11" cy="6.5" r=".9" /></svg
        >Design
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
      {#if view === "matrix"}
        <MatrixView
          inputs={state.inputs}
          outputs={state.outputs}
          gains={state.gains}
          lanes={state.lanes}
          onGain={setGain}
          onMute={setMuted}
          onFx={(i) => (fxOpen = i)}
          onPrograms={openApps}
        />
      {:else}
        <StripView
          {side}
          {hideAfter}
          inputs={state.inputs}
          outputs={state.outputs}
          gains={state.gains}
          lanes={state.lanes}
          onGain={setGain}
          onMute={setMuted}
          onMaster={setMasterGain}
          onMasterMute={setMasterMuted}
          onInputMute={setInputMuted}
          onFx={(i) => (fxOpen = i)}
          onPrograms={openApps}
        />
      {/if}
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
  <Apps inputs={config.inputs} outputs={config.outputs} onAddInput={addInput} onClose={() => (appsOpen = false)} />
{/if}

{#if themeOpen}
  <Theme {view} onView={setView} {side} onSide={setSide} {hideAfter} onHide={setHide} onClose={() => (themeOpen = false)} />
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
    padding-left: 12px;
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
    border-radius: 3px;
  }
  .name {
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  .version {
    padding: 2px 6px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: none;
    font-family: var(--mono);
    font-size: 11px;
    color: var(--muted);
    cursor: pointer;
  }
  .version:hover {
    border-color: var(--border);
    color: var(--text);
  }
  .check-note {
    margin-left: 4px;
    color: var(--signal);
  }
  .update {
    height: 26px;
    margin-left: 6px;
    padding: 0 10px;
    border: 1px solid var(--signal);
    border-radius: var(--radius-sm);
    background: none;
    color: var(--signal);
    font-size: 12px;
    font-weight: 700;
  }
  .update:hover:not(:disabled) {
    background: var(--signal);
    color: var(--bg);
  }
  .update:disabled {
    cursor: progress;
  }
  .nav {
    display: flex;
    gap: 6px;
  }
  .nav button {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 32px;
    padding: 0 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--panel-2);
    color: var(--text);
    font-size: 13px;
    font-weight: 600;
  }
  .nav button:hover {
    border-color: var(--accent);
  }
  .nav button.active {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  .nav svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.7;
    stroke-linejoin: round;
  }
  .nav button.active svg {
    stroke: currentColor;
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
  .spacer {
    flex: 1;
    align-self: stretch;
  }
  button {
    padding: 6px 14px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--panel-2);
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  main {
    flex: 1;
    display: flex;
    gap: 10px;
    padding: 10px;
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
    border-radius: var(--radius-sm);
    color: var(--danger);
  }
</style>
