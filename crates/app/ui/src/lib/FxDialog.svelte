<script>
  import { untrack } from "svelte";
  import { defaultFx, setFx } from "./api.js";

  /** Effects of one input: low cut, three-band EQ, gate. Changes apply live. */
  let { index, input, onClose } = $props();

  let fx = $state(untrack(() => ({ ...defaultFx(), ...input.fx })));
  let error = $state("");

  async function apply() {
    try {
      await setFx(index, $state.snapshot(fx));
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  function reset() {
    fx = defaultFx();
    apply();
  }

  const bands = [
    { key: "lowDb", label: "Bass", hint: "120 Hz" },
    { key: "midDb", label: "Präsenz", hint: "2,5 kHz" },
    { key: "highDb", label: "Höhen", hint: "8 kHz" },
  ];
  const lowCuts = [0, 80, 120, 160];
  const fmt = (db) => (db > 0 ? `+${db.toFixed(1)}` : db.toFixed(1));

  // Level bar for setting the gate: -90 … 0 dBFS.
  const pos = (db) => `${Math.min(100, Math.max(0, ((db + 90) / 90) * 100))}%`;
</script>

<div class="backdrop" role="presentation" onclick={onClose}></div>
<div class="dialog" role="dialog" aria-modal="true" aria-label={`Effekte für ${input.label}`}>
  <h2>Effekte: {input.label}</h2>

  <h3>Low-Cut</h3>
  <div class="segmented" role="radiogroup" aria-label="Low-Cut">
    {#each lowCuts as hz}
      <button
        role="radio"
        aria-checked={fx.lowCutHz === hz}
        class:on={fx.lowCutHz === hz}
        onclick={() => {
          fx.lowCutHz = hz;
          apply();
        }}>{hz ? `${hz} Hz` : "Aus"}</button
      >
    {/each}
  </div>
  <p class="hint">Schneidet Brummen und Trittschall unterhalb der Frequenz ab.</p>

  <h3>EQ</h3>
  {#each bands as band}
    <label class="band">
      <span class="name">{band.label} <small>{band.hint}</small></span>
      <input
        type="range"
        min="-12"
        max="12"
        step="0.5"
        bind:value={fx[band.key]}
        oninput={apply}
        ondblclick={() => {
          fx[band.key] = 0;
          apply();
        }}
        aria-label={band.label}
      />
      <span class="value">{fmt(fx[band.key])} dB</span>
    </label>
  {/each}

  <h3>Gate</h3>
  <label class="check">
    <input type="checkbox" bind:checked={fx.gate} onchange={apply} />
    Stummschalten, wenn es leiser ist als die Schwelle
  </label>
  <label class="band" class:disabled={!fx.gate}>
    <span class="name">Schwelle</span>
    <input
      type="range"
      min="-90"
      max="-10"
      step="1"
      bind:value={fx.gateDb}
      oninput={apply}
      disabled={!fx.gate}
      aria-label="Gate-Schwelle"
    />
    <span class="value">{fx.gateDb} dB</span>
  </label>
  <div class="level" title="Aktueller Pegel und Schwelle">
    <div class="bar" style:width={pos(input.levelDb)}></div>
    {#if fx.gate}<div class="mark" style:left={pos(fx.gateDb)}></div>{/if}
  </div>
  <p class="hint">Schwelle knapp über das Grundrauschen legen, wenn du nicht sprichst.</p>

  {#if error}<p class="error">{error}</p>{/if}

  <footer>
    <button onclick={reset}>Zurücksetzen</button>
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
    width: min(520px, calc(100vw - 32px));
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
    margin: 18px 0 8px;
    font-size: 13px;
    font-weight: 600;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .segmented {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }
  .segmented button {
    border: 0;
    border-radius: 0;
    padding: 6px 14px;
    background: var(--panel-2);
  }
  .segmented button + button {
    border-left: 1px solid var(--border);
  }
  .segmented button.on {
    background: var(--copper);
    color: #fff;
  }
  .band {
    display: grid;
    grid-template-columns: 130px 1fr 70px;
    align-items: center;
    gap: 10px;
    margin-bottom: 6px;
  }
  .band.disabled {
    opacity: 0.5;
  }
  .name small {
    color: var(--muted);
  }
  .value {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  input[type="range"] {
    accent-color: var(--copper);
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }
  .level {
    position: relative;
    height: 8px;
    margin: 6px 0 0 140px;
    border-radius: 4px;
    background: var(--panel-2);
    overflow: hidden;
  }
  .bar {
    height: 100%;
    background: var(--teal);
    transition: width 50ms linear;
  }
  .mark {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    background: var(--copper);
  }
  .hint {
    margin: 6px 0 0;
    font-size: 12px;
    color: var(--muted);
  }
  .error {
    color: var(--danger);
  }
  button {
    padding: 6px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--panel-2);
  }
  .primary {
    background: var(--copper);
    border-color: var(--copper);
    color: #fff;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 20px;
  }
</style>
