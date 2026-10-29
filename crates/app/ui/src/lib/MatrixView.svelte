<script>
  import HFader from "./HFader.svelte";
  import Meter from "./Meter.svelte";
  import { DB } from "./scale.js";
  import { t } from "./i18n.svelte.js";

  /** Every input on every output at a glance: rows are inputs, columns outputs. */
  let { inputs, outputs, gains, lanes, onGain, onMute, onFx, onPrograms } = $props();
</script>

<section class="matrix" style:--cols={outputs.length}>
  <div class="corner"></div>
  {#each outputs as output, o}
    <header class:offline={!output.connected && !output.released}>
      <span class="bus">A{o + 1}</span>
      <div class="titles">
        <h2>{output.label}</h2>
        <div class="device">
          {output.connected ? output.deviceName : output.released ? t("out.released") : t("out.waiting")}
        </div>
        <Meter vertical={false} db={output.connected ? output.levelDb : -120} />
      </div>
    </header>
  {/each}

  {#each inputs as input, i}
    <button class="input" class:offline={!input.connected} onclick={() => onFx(i)} title={t("fx.title")}>
      <span class="name">{input.label}</span>
      <Meter vertical={false} db={input.connected ? input.levelDb : -120} />
    </button>
    {#each outputs as _, o}
      {@const g = gains[i]?.[o] ?? { db: 0, muted: false }}
      {@const looped = lanes[i]?.[o]?.looped ?? false}
      <div class="cell" class:off={g.muted || looped}>
        <HFader
          value={g.db}
          label={t("matrix.cell", { input: input.label, n: o + 1 })}
          dim={g.muted || looped}
          onChange={(db) => onGain(i, o, db)}
        />
        <span class="value">{looped ? t("matrix.loop") : DB.format(g.db)}</span>
        <button
          class="mute"
          class:on={g.muted}
          aria-pressed={g.muted}
          aria-label={t("matrix.cellMute", { input: input.label, n: o + 1 })}
          disabled={looped}
          onclick={() => onMute(i, o, !g.muted)}>M</button
        >
      </div>
    {/each}
  {/each}

  <button class="add" onclick={onPrograms} title={t("programs.addHint")}>{t("programs.add")}</button>
</section>

<style>
  .add {
    justify-self: start;
    padding: 3px 10px;
    font-size: 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--accent);
    background: none;
    color: var(--accent);
  }
  .matrix {
    flex: 1;
    display: grid;
    grid-template-columns: minmax(130px, 170px) repeat(var(--cols), minmax(220px, 1fr));
    gap: 12px 22px;
    align-content: start;
    padding: 16px;
    overflow: auto;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  header {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding-bottom: 10px;
    border-bottom: 1px solid var(--border);
  }
  header.offline {
    opacity: 0.6;
  }
  .bus {
    flex: none;
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
    gap: 4px;
  }
  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  .device {
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .input {
    display: grid;
    gap: 4px;
    padding: 6px 8px;
    text-align: left;
    background: var(--panel-2);
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
  }
  .input:hover {
    border-color: var(--accent);
  }
  .input.offline {
    opacity: 0.5;
  }
  .name {
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .cell {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .cell :global(.track) {
    margin-left: 12px;
    margin-right: 4px;
  }
  .value {
    width: 62px;
    text-align: right;
    font-family: var(--mono);
    font-size: 12px;
    font-weight: 700;
    color: var(--accent-light);
  }
  .off .value {
    color: var(--muted);
  }
  .mute {
    flex: none;
    width: 30px;
    height: 26px;
    padding: 0;
    border-radius: 4px;
    border: 1px solid #101113;
    background: linear-gradient(#3b3f45, #24272b);
    font-size: 12px;
    font-weight: 700;
    color: #aeb4bb;
  }
  .mute.on {
    color: #fff;
    background: linear-gradient(#e8665f, #b03630);
    border-color: #5a1a17;
  }
  .mute:disabled {
    opacity: 0.4;
  }
</style>
