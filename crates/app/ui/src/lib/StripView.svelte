<script>
  import Fader from "./Fader.svelte";
  import Meter from "./Meter.svelte";
  import { tick } from "svelte";
  import { fitWindowWidth, fxActive } from "./api.js";
  import { stripChanges, stripValue, OFF } from "./scale.js";

  /**
   * Voicemeeter-style desk: one strip per input with buttons for the outputs
   * it plays on, devices first, then programs. The outputs sit at the side
   * (left unless set otherwise) with a master fader each. Per-output levels
   * live in the matrix.
   */
  let { side = "left", hideAfter = 120, inputs, outputs, gains, lanes, onGain, onMute, onMaster, onMasterMute, onInputMute, onFx, onPrograms } = $props();

  const isProgram = (input) => input.device.startsWith("app:");
  const devices = $derived(inputs.map((input, i) => ({ input, i })).filter(({ input }) => !isProgram(input)));
  const allPrograms = $derived(inputs.map((input, i) => ({ input, i })).filter(({ input }) => isProgram(input)));

  // When each program was last heard (or first seen); silent ones hide.
  const heard = new Map();
  let showAll = $state(false);
  const audible = $derived.by(() => {
    const now = Date.now();
    for (const { input } of allPrograms) {
      if (!heard.has(input.device) || (input.connected && input.levelDb > -60)) heard.set(input.device, now);
    }
    if (!hideAfter) return allPrograms;
    return allPrograms.filter(({ input }) => now - heard.get(input.device) < hideAfter * 1000);
  });
  const programs = $derived(showAll ? allPrograms : audible);
  const silent = $derived(allPrograms.length - audible.length);

  const row = (i) => outputs.map((_, o) => gains[i]?.[o] ?? { db: 0, muted: true });

  // The window follows the strips: wider for each new one, narrower when
  // one goes. Measured after the DOM has the new strips.
  let stripsEl = $state();
  $effect(() => {
    const key = `${outputs.length}/${devices.length}/${programs.length}/${side}`;
    if (!stripsEl || !key) return;
    tick().then(() => {
      if (!stripsEl) return;
      const groups = [...stripsEl.children];
      const gap = parseFloat(getComputedStyle(stripsEl).columnGap) || 0;
      const content = groups.reduce((w, g) => w + g.offsetWidth, 0) + gap * Math.max(0, groups.length - 1);
      fitWindowWidth(content + window.innerWidth - stripsEl.clientWidth);
    });
  });

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

{#snippet strip(input, i, narrow = false)}
  {@const r = row(i)}
  <section class="strip panel" class:narrow>
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
        muted={input.muted ?? false}
        levelDb={input.levelDb}
        connected={input.connected}
        {narrow}
        onChange={(db) => move(i, db)}
        onMute={(v) => onInputMute(i, v)}
      />
    </div>
    <button class="fx" class:on={fxActive(input.fx)} onclick={() => onFx(i)}>Effekte</button>
  </section>
{/snippet}

<div class="desk">
  <div class="strips" bind:this={stripsEl}>
    <div class="group outs" class:left={side === "left"}>
      <h2>Ausgänge</h2>
      <div class="row">
        {#each outputs as output, o}
          {@const m = output.master ?? { db: 0, muted: false }}
          <section class="strip panel">
            <span class="tag">A{o + 1}</span>
            <div class="grow">
              <Fader
                label={output.label}
                note={output.connected ? output.deviceName : output.released ? "Freigegeben" : "Wartet auf Gerät"}
                noteOn={output.connected}
                noteTitle={output.released ? "Freigegeben, kommt mit Ton zurück" : (output.deviceName ?? "")}
                value={m.db}
                muted={m.muted}
                levelDb={output.levelDb}
                connected={output.connected || output.released}
                postFader
                onChange={(db) => onMaster(o, db)}
                onMute={(v) => onMasterMute(o, v)}
              />
            </div>
            <!-- Keeps the fader level with the inputs' "Effekte" row. -->
            <span class="fx spacer" aria-hidden="true">Effekte</span>
          </section>
        {/each}
      </div>
    </div>
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
          {@render strip(input, i, true)}
        {:else}
          {#if !allPrograms.length}
            <p class="hint">Mit „+ Programm“ bekommt z. B. Discord oder Brave einen eigenen Streifen.</p>
          {/if}
        {/each}
        {#if silent}
          <button class="more" onclick={() => (showAll = !showAll)}
            >{showAll ? "Stille ausblenden" : `${silent} still, zeigen`}</button
          >
        {/if}
      </div>
    </div>
  </div>

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
  /* Outputs come first in the markup; on the right they move to the end. */
  .outs:not(.left) {
    order: 1;
    padding-left: 14px;
    border-left: 2px solid var(--border);
  }
  .outs:not(.left) + .group {
    padding-left: 0;
    border-left: none;
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
  .strip.narrow {
    padding: 10px 8px;
  }
  .narrow .bus {
    min-width: 0;
    padding: 3px 4px;
    font-size: 11px;
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
  .fx.spacer {
    visibility: hidden;
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
  .more {
    align-self: flex-start;
    padding: 4px 10px;
    font-size: 12px;
    color: var(--muted);
    background: none;
    border: 1px dashed var(--border);
    border-radius: var(--radius-sm);
  }
  .hint {
    width: 160px;
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }
  .offline {
    opacity: 0.5;
  }
  /* The device name sits over the scale, so the faders stay level. */
  .outs :global(.note) {
    position: absolute;
    top: 25px;
    left: 0;
    right: 0;
    margin: 0;
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tag {
    align-self: center;
    padding: 3px 7px;
    border: 1px solid var(--accent);
    font-family: var(--mono);
    font-size: 12px;
    font-weight: 700;
    color: #fff;
    background: var(--accent);
    border-radius: var(--radius-sm);
  }
</style>
