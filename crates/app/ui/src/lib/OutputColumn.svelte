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
    <span class="dot" class:on={output.connected} title={output.connected ? "Verbunden" : "Nicht verbunden"}></span>
    <div class="titles">
      <h2>{output.label}</h2>
      <div class="device" title={output.deviceName ?? output.device}>
        {output.connected ? output.deviceName : "Wartet auf Gerät …"}
      </div>
    </div>
  </header>

  <div class="faders">
    {#each inputs as input, i}
      <Fader
        label={input.label}
        db={gains[i]?.[index]?.db ?? 0}
        muted={gains[i]?.[index]?.muted ?? false}
        levelDb={input.levelDb}
        connected={input.connected}
        onGain={(db) => onGain(i, index, db)}
        onMute={(m) => onMute(i, index, m)}
      />
    {/each}
    <div class="master">
      <div class="label">Summe</div>
      <Meter db={output.connected ? output.levelDb : -120} />
      <div class="db">{output.connected && output.levelDb > -60 ? output.levelDb.toFixed(0) : "−∞"}</div>
      <div class="spacer"></div>
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
    gap: 12px;
    padding: 14px;
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
  }
  .titles {
    min-width: 0;
  }
  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  .device {
    font-size: 12px;
    color: var(--muted);
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dot {
    flex: none;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--border);
  }
  .dot.on {
    background: var(--teal);
    box-shadow: 0 0 8px var(--teal);
  }
  .faders {
    display: flex;
    gap: 4px;
    min-height: 0;
  }
  .master {
    display: grid;
    grid-template-rows: auto 1fr auto auto;
    justify-items: center;
    gap: 6px;
    padding-left: 10px;
    margin-left: 6px;
    border-left: 1px solid var(--border);
  }
  .master .label,
  .master .db {
    font-size: 12px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .spacer {
    height: 24px;
  }
  footer {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: var(--muted);
  }
  .warn {
    color: var(--copper);
  }
</style>
