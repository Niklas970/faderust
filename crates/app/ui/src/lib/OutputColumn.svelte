<script>
  import Fader from "./Fader.svelte";
  import Meter from "./Meter.svelte";

  /** One output: its own mix of every input. */
  let { output, index, inputs, gains, lanes, onGain, onMute } = $props();

  const underruns = $derived(
    inputs.reduce((sum, _, i) => sum + (lanes[i]?.[index]?.underruns ?? 0), 0),
  );
</script>

<section class="column" class:offline={!output.connected}>
  <header>
    <span class="bus">A{index + 1}</span>
    <div class="titles">
      <h2>{output.label}</h2>
      <div class="device" title={output.deviceName ?? output.device}>
        {output.connected ? output.deviceName : "Wartet auf Gerät …"}
      </div>
    </div>
    <span class="led" class:on={output.connected} title={output.connected ? "Verbunden" : "Nicht verbunden"}></span>
  </header>

  <div class="faders">
    {#each inputs as input, i}
      <Fader
        label={input.label}
        value={gains[i]?.[index]?.db ?? 0}
        muted={gains[i]?.[index]?.muted ?? false}
        levelDb={input.levelDb}
        connected={input.connected}
        onChange={(db) => onGain(i, index, db)}
        onMute={(m) => onMute(i, index, m)}
      />
    {/each}
    <div class="master">
      <div class="label">Out</div>
      <div class="master-meter"><Meter db={output.connected ? output.levelDb : -120} /></div>
      <div class="readout">{output.connected && output.levelDb > -60 ? output.levelDb.toFixed(0) : "−∞"}</div>
    </div>
  </div>

  <footer>
    <span>Puffer {output.bufferMs} ms</span>
    {#if underruns > 0}<span class="warn">{underruns} Aussetzer</span>{/if}
  </footer>
</section>

<style>
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
  .column.offline {
    border-style: dashed;
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border);
  }
  .bus {
    flex: none;
    padding: 2px 6px;
    font-family: var(--mono);
    font-size: 12px;
    font-weight: 700;
    color: var(--copper);
    border: 1px solid var(--copper);
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
  .device {
    font-size: 11px;
    color: var(--muted);
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .led {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 1px;
    background: var(--well);
    border: 1px solid var(--border);
  }
  .led.on {
    background: var(--teal);
    border-color: var(--teal);
  }
  .faders {
    display: flex;
    gap: 2px;
    min-height: 0;
  }
  .master {
    display: grid;
    grid-template-rows: auto 1fr auto;
    justify-items: center;
    gap: 8px;
    padding-left: 10px;
    margin-left: 6px;
    border-left: 1px solid var(--border);
  }
  .master-meter {
    height: 100%;
    padding: 11px 0;
  }
  .master-meter :global(.meter) {
    width: 14px;
  }
  .master .label {
    padding: 3px 6px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .readout {
    display: grid;
    place-items: center;
    width: 34px;
    height: 26px;
    font-family: var(--mono);
    font-size: 12px;
    color: var(--teal);
    background: var(--well);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  footer {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--muted);
  }
  .warn {
    color: var(--amber);
  }
</style>
