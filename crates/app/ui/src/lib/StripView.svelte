<script>
  import Fader from "./Fader.svelte";
  import HFader from "./HFader.svelte";
  import Meter from "./Meter.svelte";
  import { fxActive } from "./api.js";
  import { DB, stripChanges, stripValue, OFF } from "./scale.js";

  /**
   * Voicemeeter-style desk: one strip per input with buttons for the outputs
   * it plays on, devices first, then programs. The outputs sit at the side
   * (left unless set otherwise) with a master fader each. Per-output levels
   * live in the matrix.
   */
  let { side = "left", inputs, outputs, gains, lanes, onGain, onMute, onMaster, onMasterMute, onFx, onPrograms } = $props();

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

{#snippet strip(input, i)}
  {@const r = row(i)}
  <section class="strip panel">
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
{/snippet}

<div class="desk">
  <div class="strips">
    {#if devices.length}
      <div class="group">
        <h2>Geräte</h2>
        <div class="row">
          {#each devices as { input, i } (i)}{@render strip(input, i)}{/each}
        </div>
      </div>
    {/if}
    <div class="group">
      <h2>Programme <button class="add" onclick={onPrograms} title="Programme als Eingang aufnehmen">+ Programm</button></h2>
      <div class="row">
        {#each programs as { input, i } (i)}
          {@render strip(input, i)}
        {:else}
          <p class="hint">Mit „+ Programm“ bekommt z. B. Discord oder Brave einen eigenen Streifen.</p>
        {/each}
      </div>
    </div>
  </div>

  <aside class="side panel" class:left={side === "left"}>
    <h2>Ausgänge</h2>
    {#each outputs as output, o}
      {@const m = output.master ?? { db: 0, muted: false }}
      <div class="output" class:offline={!output.connected && !output.released}>
        <div class="line">
          <span class="tag">A{o + 1}</span>
          <div class="titles">
            <div class="label">{output.label}</div>
            <div class="device">
              {output.connected ? output.deviceName : output.released ? "Freigegeben, kommt mit Ton zurück" : "Wartet auf Gerät …"}
            </div>
          </div>
          <span class="value" class:off={m.muted}>{DB.format(m.db)}</span>
          <button
            class="mute"
            class:on={m.muted}
            aria-pressed={m.muted}
            aria-label={`${output.label} stumm`}
            onclick={() => onMasterMute(o, !m.muted)}>M</button
          >
        </div>
        <div class="master">
          <HFader value={m.db} label={`${output.label} Gesamt`} dim={m.muted} onChange={(db) => onMaster(o, db)} />
        </div>
        <Meter vertical={false} db={output.connected ? output.levelDb : -120} />
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
    gap: 14px;
    min-width: 0;
    overflow-x: auto;
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-height: 0;
  }
  .group + .group {
    padding-left: 14px;
    border-left: 2px solid var(--border);
  }
  .row {
    flex: 1;
    display: flex;
    gap: 10px;
    min-height: 0;
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 24px;
    margin: 0;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--muted);
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
  .add {
    padding: 3px 10px;
    font-size: 12px;
    letter-spacing: 0;
    text-transform: none;
    border-radius: var(--radius-sm);
    border: 1px solid var(--accent);
    background: none;
    color: var(--accent);
  }
  .hint {
    width: 160px;
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }
  .side.left {
    order: -1;
  }
  .side {
    flex: none;
    width: 300px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    overflow-x: hidden;
    overflow-y: auto;
  }
  .output {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 8px;
    padding: 8px 10px;
    background: var(--panel-2);
    border-radius: var(--radius-sm);
  }
  .line {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }
  .master :global(.track) {
    margin: 0 11px;
  }
  .offline {
    opacity: 0.5;
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
  }
  .label {
    font-size: 13px;
    font-weight: 700;
  }
  .device {
    font-size: 11px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .value {
    font-family: var(--mono);
    font-size: 12px;
    line-height: 22px;
    color: var(--accent);
  }
  .value.off {
    color: var(--muted);
  }
  .mute {
    padding: 2px 8px;
    font-family: var(--mono);
    font-size: 12px;
    font-weight: 700;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--panel);
  }
  .mute.on {
    color: #fff;
    background: var(--danger);
    border-color: var(--danger);
  }
</style>
