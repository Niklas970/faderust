<script>
  import { onMount } from "svelte";
  import { getAppLevels, getApps, setAppVolume } from "./api.js";
  import Fader from "./Fader.svelte";

  /**
   * Windows volume of every program playing audio, one fader each. Programs
   * recorded as inputs have their own faders in the output columns and are
   * left out. Folds to a narrow strip; the choice is remembered.
   */
  let { inputs = [], onRouting } = $props();

  const KEY = "faderust.programs.open";
  let open = $state(read());
  function read() {
    try {
      return localStorage.getItem(KEY) === "1";
    } catch {
      return false;
    }
  }
  function toggle() {
    open = !open;
    try {
      localStorage.setItem(KEY, open ? "1" : "0");
    } catch {
      // Not remembered; still toggles.
    }
  }

  const isInput = (app) => inputs.some((i) => i.device === `app:${app.exe.toLowerCase()}`);
  let apps = $derived(data?.apps.filter((a) => !isInput(a)) ?? []);

  let data = $state(null);
  let error = $state("");

  async function refresh() {
    try {
      data = await getApps();
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  // Peak per process; a program's meter shows its loudest process.
  let levels = $state(new Map());
  let polling = false;
  async function poll() {
    if (polling) return;
    polling = true;
    try {
      levels = new Map((await getAppLevels()).map((l) => [l.pid, l.db]));
    } catch {
      // Meters just stand still; the list refresh reports errors.
    } finally {
      polling = false;
    }
  }
  const levelOf = (app) => Math.max(-120, ...app.pids.map((p) => levels.get(p) ?? -120));

  /** Where the program's sound ends up: through Faderust (a cable) or straight out. */
  function route(app) {
    const device = data.devices.find((d) => d.id === app.device);
    if (device?.cable) return { text: "über Faderust", via: true };
    return { text: device ? "direkt" : "Windows-Standard", via: false };
  }

  // Programs come and go; look again every few seconds.
  onMount(() => {
    refresh();
    const list = setInterval(refresh, 3000);
    const meters = setInterval(poll, 100);
    return () => {
      clearInterval(list);
      clearInterval(meters);
    };
  });

  // One change per program on its way at a time; only the newest waits.
  const busy = new Set();
  const queued = new Map();
  async function send(app, volume, muted) {
    const key = app.exe;
    if (busy.has(key)) {
      const prev = queued.get(key) ?? {};
      queued.set(key, { volume: volume ?? prev.volume ?? null, muted: muted ?? prev.muted ?? null });
      return;
    }
    busy.add(key);
    try {
      await setAppVolume($state.snapshot(app.pids), volume, muted);
      error = "";
    } catch (e) {
      error = String(e);
    } finally {
      busy.delete(key);
      const next = queued.get(key);
      if (next) {
        queued.delete(key);
        send(app, next.volume, next.muted);
      }
    }
  }

  function volume(app, pct) {
    app.volume = pct / 100;
    send(app, app.volume, null);
  }

  function mute(app, muted) {
    app.muted = muted;
    send(app, null, muted);
  }
</script>

{#if data?.supported && !open}
  <button class="folded" onclick={toggle} title="Programm-Lautstärken einblenden">
    <span class="bus">WIN</span>
    <span class="vertical">Programme · {apps.length}</span>
  </button>
{:else if data?.supported}
  <section class="column">
    <header>
      <span class="bus">WIN</span>
      <div class="titles">
        <h2>Programme</h2>
        <div class="sub">Lautstärke in Windows</div>
      </div>
      <button class="route" onclick={onRouting} title="Ausgabegerät pro Programm">Ausgänge</button>
      <button class="route" onclick={toggle} title="Einklappen" aria-label="Einklappen">»</button>
    </header>

    {#if apps.length}
      <div class="faders">
        {#each apps as app (app.exe)}
          <Fader
            label={app.name}
            unit="pct"
            value={app.volume * 100}
            muted={app.muted}
            levelDb={levelOf(app)}
            note={route(app).text}
            noteOn={route(app).via}
            onChange={(v) => volume(app, v)}
            onMute={(m) => mute(app, m)}
          />
        {/each}
      </div>
    {:else}
      <p class="empty">
        {data.apps.length ? "Alle Programme, die Ton abspielen, sind Eingänge." : "Gerade spielt kein Programm Ton ab."}
      </p>
    {/if}

    <footer>
      {#if error}<span class="error">{error}</span>{:else}<span>{apps.length} Programme</span>{/if}
    </footer>
  </section>
{/if}

<style>
  .folded {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    flex: none;
    width: 40px;
    padding: 10px 0;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--muted);
    cursor: pointer;
  }
  .folded:hover {
    border-color: var(--accent);
    color: var(--text);
  }
  .vertical {
    writing-mode: vertical-rl;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .column {
    display: grid;
    grid-template-rows: auto 1fr auto;
    gap: 10px;
    padding: 10px;
    min-width: max-content;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border);
  }
  .bus {
    flex: none;
    padding: 2px 6px;
    font-family: var(--mono);
    font-size: 12px;
    font-weight: 700;
    color: var(--signal);
    border: 1px solid var(--signal);
    border-radius: var(--radius-sm);
  }
  .titles {
    flex: 1;
    min-width: 0;
  }
  h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }
  .sub {
    font-size: 11px;
    color: var(--muted);
  }
  .route {
    padding: 4px 10px;
    font-size: 12px;
    font-weight: 600;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--panel-2);
  }
  .route:hover {
    border-color: var(--accent);
  }
  .faders {
    display: flex;
    gap: 2px;
    min-height: 0;
  }
  .empty {
    align-self: center;
    width: 200px;
    margin: 0;
    color: var(--muted);
    text-align: center;
  }
  footer {
    font-size: 11px;
    color: var(--muted);
  }
  .error {
    color: var(--danger);
  }
</style>
