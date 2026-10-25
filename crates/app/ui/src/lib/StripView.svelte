<script>
  import Fader from "./Fader.svelte";
  import HFader from "./HFader.svelte";
  import Meter from "./Meter.svelte";
  import { fxActive } from "./api.js";
  import { stripChanges, stripValue, OFF } from "./scale.js";

  /**
   * Voicemeeter-style desk: one strip per device input with buttons for the
   * outputs it plays on. Programs recorded as inputs sit in a list on the
   * right, the outputs below them. Per-output levels live in the matrix.
   */
  let { inputs, outputs, gains, lanes, onGain, onMute, onFx, onPrograms } = $props();

  const isProgram = (input) => input.device.startsWith("app:");
  const devices = $derived(inputs.map((input, i) => ({ input, i })).filter(({ input }) => !isProgram(input)));
  const programs = $derived(inputs.map((input, i) => ({ input, i })).filter(({ input }) => isProgram(input)));

  const row = (i) => outputs.map((_, o) => gains[i]?.[o] ?? { db: 0, muted: true });

  function move(i, db) {
    for (const [o, v] of stripChanges(row(i), db)) onGain(i, o, v);
  }

  /** Switching an output on that sits at -∞ brings it to the strip's level. */
  function toggle(i, o) {
    const g = row(i)[o];
    if (!g.muted) return onMute(i, o, true);
    if (g.db <= OFF) {
      const level = stripValue(row(i));
      onGain(i, o, level > OFF ? level : 0);
    }
    onMute(i, o, false);
  }
</script>

{#snippet buses(i)}
  <div class="buses">
    {#each outputs as output, o}
      {@const on = !(gains[i]?.[o]?.muted ?? true)}
      {@const looped = lanes[i]?.[o]?.looped ?? false}
      <button
        class="bus"
        class:on={on && !looped}
        class:looped
        aria-pressed={on}
        title={looped ? `Schleife: ${output.label} spielt auf das Kabel dieses Eingangs` : `${output.label} ${on ? "aus" : "an"}`}
        onclick={() => toggle(i, o)}>A{o + 1}</button
      >
    {/each}
  </div>
{/snippet}

<div class="desk">
  <div class="strips">
    {#each devices as { input, i } (i)}
      {@const r = row(i)}
      <section class="strip panel">
        {@render buses(i)}
        <div class="grow">
          <Fader
          label={input.label}
          value={stripValue(r)}
          muted={r.every((g) => g.muted)}
          levelDb={input.levelDb}
          connected={input.connected}
          showMute={false}
          onChange={(db) => move(i, db)}
          onMute={() => {}}
        />
        </div>
        <button class="fx" class:on={fxActive(input.fx)} onclick={() => onFx(i)}>Effekte</button>
      </section>
    {/each}
  </div>

  <aside class="side panel">
    <div class="head">
      <h2>Programme</h2>
      <button class="add" onclick={onPrograms} title="Programme als Eingang aufnehmen">+ Programm</button>
    </div>
    {#each programs as { input, i } (i)}
      {@const r = row(i)}
      <div class="program" class:offline={!input.connected}>
        <div class="line">
          <button class="name" onclick={() => onFx(i)} title="Effekte">{input.label}</button>
          {@render buses(i)}
        </div>
        <div class="line">
          <HFader
            value={stripValue(r)}
            label={`${input.label} Pegel`}
            dim={r.every((g) => g.muted)}
            onChange={(db) => move(i, db)}
          />
        </div>
        <Meter vertical={false} db={input.connected ? input.levelDb : -120} />
      </div>
    {:else}
      <p class="hint">Noch keine Programme. Mit „+ Programm“ bekommt z. B. Discord oder Brave hier einen eigenen Regler.</p>
    {/each}

    <h2 class="outs">Ausgänge</h2>
    {#each outputs as output, o}
      <div class="output" class:offline={!output.connected && !output.released}>
        <span class="tag">A{o + 1}</span>
        <div class="titles">
          <div class="label">{output.label}</div>
          <div class="device">
            {output.connected ? output.deviceName : output.released ? "Freigegeben, kommt mit Ton zurück" : "Wartet auf Gerät …"}
          </div>
          <Meter vertical={false} db={output.connected ? output.levelDb : -120} />
        </div>
      </div>
    {/each}
  </aside>
</div>

<style>
  .desk {
    flex: 1;
    display: flex;
    gap: 10px;
    min-width: 0;
    min-height: 0;
  }
  .panel {
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .strips {
    flex: 1;
    display: flex;
    gap: 10px;
    min-width: 0;
    overflow-x: auto;
  }
  .strip {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
  }
  .grow {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .buses {
    display: flex;
    gap: 4px;
  }
  .bus {
    min-width: 34px;
    padding: 3px 6px;
    font-family: var(--mono);
    font-size: 12px;
    font-weight: 700;
    color: var(--accent);
    background: none;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
  }
  .bus.on {
    color: #fff;
    background: var(--accent);
  }
  .bus.looped {
    color: var(--danger);
    border-color: var(--danger);
    text-decoration: line-through;
  }
  .fx.on {
    color: var(--accent);
    border-color: var(--accent);
  }
  .fx {
    padding: 4px 12px;
    font-size: 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--panel-2);
  }
  .side {
    flex: none;
    width: 300px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    overflow-y: auto;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  h2 {
    margin: 0;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .outs {
    margin-top: 8px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }
  .add {
    padding: 3px 10px;
    font-size: 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--accent);
    background: none;
    color: var(--accent);
  }
  .program {
    display: grid;
    gap: 6px;
    padding: 8px;
    background: var(--panel-2);
    border-radius: var(--radius-sm);
  }
  .program :global(.track) {
    margin: 0 11px;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .name {
    flex: 1;
    min-width: 0;
    padding: 0;
    text-align: left;
    background: none;
    border: none;
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .name:hover {
    color: var(--accent);
  }
  .offline {
    opacity: 0.5;
  }
  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }
  .output {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 8px;
    background: var(--panel-2);
    border-radius: var(--radius-sm);
  }
  .tag {
    padding: 2px 6px;
    font-family: var(--mono);
    font-size: 12px;
    font-weight: 700;
    color: #fff;
    background: var(--accent);
    border-radius: var(--radius-sm);
  }
  .titles {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 3px;
  }
  .label {
    font-size: 13px;
    font-weight: 600;
  }
  .device {
    font-size: 11px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
