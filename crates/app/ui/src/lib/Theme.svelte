<script>
  import { ACCENTS, DEFAULT_THEME, MODES, SIGNALS, loadTheme, saveTheme } from "./theme.js";

  /** Layout, colours and light/dark; every change shows right away. */
  let { view, onView, side, onSide, onClose } = $props();

  const SIDES = [
    { value: "left", name: "Links" },
    { value: "right", name: "Rechts" },
  ];

  const VIEWS = [
    { value: "strips", name: "Pult", hint: "Ein Streifen pro Eingang" },
    { value: "matrix", name: "Matrix", hint: "Alle Ein- und Ausgänge als Tabelle" },
  ];

  let theme = $state(loadTheme());

  function set(key, value) {
    theme[key] = value;
    saveTheme($state.snapshot(theme));
  }

  function reset() {
    theme = { ...DEFAULT_THEME };
    saveTheme(theme);
  }

  const groups = [
    { key: "accent", title: "Akzent", hint: "Regler, Knöpfe, Kappen", list: ACCENTS },
    { key: "signal", title: "Signal", hint: "Pegelanzeigen, Status", list: SIGNALS },
  ];
</script>

<div class="backdrop" role="presentation" onclick={onClose}></div>
<div class="dialog" role="dialog" aria-modal="true" aria-label="Design">
  <h2>Design</h2>

  <section>
    <h3>Ansicht</h3>
    <div class="modes" role="radiogroup" aria-label="Ansicht">
      {#each VIEWS as v}
        <button class:on={view === v.value} role="radio" aria-checked={view === v.value} title={v.hint} onclick={() => onView(v.value)}
          >{v.name}</button
        >
      {/each}
    </div>
  </section>

  {#if view === "strips"}
    <section>
      <h3>Ausgänge im Pult</h3>
      <div class="modes" role="radiogroup" aria-label="Ausgänge im Pult">
        {#each SIDES as v}
          <button class:on={side === v.value} role="radio" aria-checked={side === v.value} onclick={() => onSide(v.value)}>{v.name}</button>
        {/each}
      </div>
    </section>
  {/if}

  {#each groups as g}
    {@const custom = !g.list.some((c) => c.color === theme[g.key])}
    <section>
      <h3>{g.title} <span>{g.hint}</span></h3>
      <div class="swatches" role="radiogroup" aria-label={g.title}>
        {#each g.list as c}
          <button
            class="swatch"
            class:on={theme[g.key] === c.color}
            style:--c={c.color}
            role="radio"
            aria-checked={theme[g.key] === c.color}
            title={c.name}
            aria-label={c.name}
            onclick={() => set(g.key, c.color)}
          ></button>
        {/each}
        <label class="swatch picker" class:on={custom} style:--c={theme[g.key]} title="Eigene Farbe">
          <input
            type="color"
            value={theme[g.key]}
            oninput={(e) => set(g.key, e.currentTarget.value)}
            aria-label={`${g.title}: eigene Farbe`}
          />
        </label>
      </div>
    </section>
  {/each}

  <section>
    <h3>Darstellung</h3>
    <div class="modes" role="radiogroup" aria-label="Darstellung">
      {#each MODES as m}
        <button
          class:on={theme.mode === m.value}
          role="radio"
          aria-checked={theme.mode === m.value}
          onclick={() => set("mode", m.value)}>{m.name}</button
        >
      {/each}
    </div>
  </section>

  <div class="preview" aria-hidden="true">
    <span class="knob"></span>
    <span class="bar"></span>
    <span class="leds"></span>
  </div>

  <footer>
    <button onclick={reset}>Standard</button>
    <button class="primary" onclick={onClose}>Fertig</button>
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
    width: min(440px, calc(100vw - 32px));
    max-height: calc(100vh - 32px);
    overflow: auto;
    padding: 20px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  h2 {
    margin: 0 0 4px;
    font-size: 18px;
  }
  h3 {
    margin: 16px 0 8px;
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--accent);
  }
  h3 span {
    margin-left: 6px;
    font-size: 11px;
    font-weight: 400;
    text-transform: none;
    letter-spacing: 0;
    color: var(--muted);
  }
  .swatches {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .swatch {
    position: relative;
    width: 32px;
    height: 32px;
    padding: 0;
    border-radius: var(--radius-sm);
    border: 1px solid rgb(0 0 0 / 0.5);
    background: var(--c);
    box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.25);
    cursor: pointer;
  }
  .swatch.on {
    outline: 2px solid var(--text);
    outline-offset: 2px;
  }
  .picker {
    display: grid;
    place-items: center;
    background:
      linear-gradient(var(--c), var(--c)) center / 14px 14px no-repeat,
      conic-gradient(#e0605a, #e0a43a, #5fb56a, #3fc8b4, #4f9bd9, #9b7ce8, #d9609e, #e0605a);
  }
  .picker input {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
    cursor: pointer;
  }
  .modes {
    display: flex;
    gap: 4px;
  }
  .modes button {
    flex: 1;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--panel-2);
    font-weight: 600;
  }
  .modes button.on {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  /* A tiny strip: cap edge, lit bar and LEDs in the chosen colours. */
  .preview {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 20px;
    padding: 12px;
    background: var(--well);
    border-radius: var(--radius-sm);
  }
  .knob {
    width: 40px;
    height: 22px;
    border-radius: 3px;
    border-left: 3px solid var(--accent);
    border-right: 3px solid var(--accent);
    background: linear-gradient(#55595f, #2f3338 45%, #23262a 55%, #33373c);
  }
  .bar {
    flex: 1;
    height: 4px;
    border-radius: 2px;
    background: var(--accent);
    box-shadow: 0 0 8px color-mix(in srgb, var(--accent) 60%, transparent);
  }
  .leds {
    width: 90px;
    height: 10px;
    background: linear-gradient(to right, var(--signal) 0 75%, var(--amber) 75% 92%, var(--danger) 92%);
    mask: repeating-linear-gradient(to right, #000 0 4px, transparent 4px 6px);
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }
  footer button {
    padding: 6px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--panel-2);
  }
  footer .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
</style>
