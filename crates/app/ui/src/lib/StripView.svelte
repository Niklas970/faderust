<script>
  import Fader from "./Fader.svelte";
  import Meter from "./Meter.svelte";
  import { tick } from "svelte";
  import { fitWindowWidth, fxActive } from "./api.js";
  import { t } from "./i18n.svelte.js";
  import { stripChanges, stripValue, OFF } from "./scale.js";

  /**
   * Voicemeeter-style desk: one strip per input with buttons for the outputs
   * it plays on, devices first, then programs. The outputs sit at the side
   * (left unless set otherwise) with a master fader each. Per-output levels
   * live in the matrix.
   */
  let { compact = false, side = "left", hideAfter = 120, inputs, outputs, gains, lanes, onGain, onMute, onMaster, onMasterMute, onInputMute, onFx, onPrograms } = $props();

  const isProgram = (input) => input.device.startsWith("app:");
  const devices = $derived(inputs.map((input, i) => ({ input, i })).filter(({ input }) => !isProgram(input)));
  const allPrograms = $derived(inputs.map((input, i) => ({ input, i })).filter(({ input }) => isProgram(input)));

  // Programs the user wants to stay on the desk even when silent.
  const KEEP_KEY = "faderust.keepShown";
  // Outputs hidden from the desk (by label); they keep playing.
  const HIDE_KEY = "faderust.hiddenOutputs";
  const load = (key) => {
    try {
      const v = JSON.parse(localStorage.getItem(key) ?? "[]");
      return Array.isArray(v) ? v : [];
    } catch {
      return [];
    }
  };
  const save = (key, list) => {
    try {
      localStorage.setItem(key, JSON.stringify(list));
    } catch {
      // Not saved; it still applies until the window closes.
    }
  };
  let keep = $state(load(KEEP_KEY));
  let hiddenOut = $state(load(HIDE_KEY));
  function toggleKeep(device) {
    keep = keep.includes(device) ? keep.filter((d) => d !== device) : [...keep, device];
    save(KEEP_KEY, keep);
  }
  const hidden = (o) => hiddenOut.includes(outputs[o]?.label);
  const hiddenCount = $derived(outputs.filter((out) => hiddenOut.includes(out.label)).length);
  function setHidden(label, value) {
    hiddenOut = value ? [...hiddenOut, label] : hiddenOut.filter((l) => l !== label);
    save(HIDE_KEY, hiddenOut);
  }

  // Small menu under a header button: silent programs to show one by one,
  // or hidden outputs to bring back.
  let menu = $state(null);
  function openMenu(e, kind) {
    e.stopPropagation();
    if (menu?.kind === kind) return void (menu = null);
    const r = e.currentTarget.getBoundingClientRect();
    menu = { kind, x: r.left, y: r.bottom + 4 };
  }
  // Silent programs shown on request, until the window closes.
  let revealed = $state([]);

  // When each program was last heard; silent ones hide. A program not heard
  // yet counts as heard a moment ago only, so after a start the silent ones
  // are gone after a few seconds instead of after the full time.
  const GRACE = 5000;
  // Programs first seen while the app starts are not given the grace: the
  // window would open wide for all of them and shrink a moment later.
  const STARTUP = 4000;
  const mounted = Date.now();
  const heard = new Map();
  let showAll = $state(false);
  const audible = $derived.by(() => {
    const now = Date.now();
    for (const { input } of allPrograms) {
      if (input.connected && input.levelDb > -60) heard.set(input.device, now);
      else if (!heard.has(input.device)) {
        const grace = now - mounted < STARTUP ? 0 : GRACE;
        heard.set(input.device, now - Math.max(0, hideAfter * 1000 - grace));
      }
    }
    if (!hideAfter) return allPrograms;
    return allPrograms.filter(
      ({ input }) =>
        keep.includes(input.device) || revealed.includes(input.device) || now - heard.get(input.device) < hideAfter * 1000,
    );
  });
  const programs = $derived(showAll ? allPrograms : audible);
  const silentList = $derived(allPrograms.filter((p) => !audible.includes(p)));
  const silent = $derived(silentList.length);

  const row = (i) => outputs.map((_, o) => gains[i]?.[o] ?? { db: 0, muted: true });

  // The window follows the strips: wider for each new one, narrower when
  // one goes. Measured after the DOM has the new strips.
  let stripsEl = $state();
  $effect(() => {
    const key = `${outputs.length - hiddenCount}/${devices.length}/${programs.length}/${side}/${compact}`;
    if (!stripsEl || !key) return;
    tick().then(() => {
      if (!stripsEl) return;
      const groups = [...stripsEl.children];
      const gap = parseFloat(getComputedStyle(stripsEl).columnGap) || 0;
      const content = groups.reduce((w, g) => w + g.offsetWidth, 0) + gap * Math.max(0, groups.length - 1);
      fitWindowWidth(content + window.innerWidth - stripsEl.clientWidth);
    });
  });

  // Moving a strip shifts all its outputs by the same amount. The levels
  // from before the move are kept while only the strip fader moves them, so
  // pulling it down and up again restores the differences between outputs
  // (an output pushed to -∞ on the way would otherwise stay there).
  const anchors = new Map();

  function move(i, db) {
    const now = row(i);
    const anchor = anchors.get(i);
    const untouched =
      anchor && anchor.applied.length === now.length && anchor.base.every((g, o) => now[o]?.muted === g.muted) &&
      anchor.applied.every(([o, v]) => now[o]?.db === v);
    const base = untouched ? anchor.base : now.map((g) => ({ ...g }));
    const changes = stripChanges(base, db);
    for (const [o, v] of changes) onGain(i, o, v);
    anchors.set(i, { base, applied: changes });
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

{#snippet strip(input, i, program = false)}
  {@const narrow = program || compact}
  {@const r = row(i)}
  <section class="strip panel" class:narrow>
    <div class="buses">
      {#each outputs as output, o}
        {#if !hidden(o)}
        {@const on = !(gains[i]?.[o]?.muted ?? true)}
        {@const looped = lanes[i]?.[o]?.looped ?? false}
        <button
          class="bus"
          class:on={on && !looped}
          class:looped
          aria-pressed={on}
          title={looped ? t("strip.loopHint", { output: output.label }) : t(on ? "strip.off" : "strip.on", { output: output.label })}
          onclick={() => toggle(i, o)}>A{o + 1}</button
        >
        {/if}
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
    {#if narrow}
      <div class="foot">
        <button class="fx" class:on={fxActive(input.fx)} onclick={() => onFx(i)} title={t("fx.title")}>FX</button>
        {#if program}
        <button
          class="keep"
          class:on={keep.includes(input.device)}
          aria-pressed={keep.includes(input.device)}
          title={keep.includes(input.device) ? t("strip.keepOn") : t("strip.keepOff")}
          aria-label={keep.includes(input.device) ? t("strip.keepOn") : t("strip.keepOff")}
          onclick={() => toggleKeep(input.device)}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true"
            ><path d="M1.5 8S4 3.5 8 3.5 14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z" /><circle cx="8" cy="8" r="2" /></svg
          >
        </button>
        {/if}
      </div>
    {:else}
      <button class="fx" class:on={fxActive(input.fx)} onclick={() => onFx(i)}>{t("fx.title")}</button>
    {/if}
  </section>
{/snippet}

<svelte:window onclick={() => (menu = null)} />

{#if menu}
  <div class="menu panel" style="left: {menu.x}px; top: {menu.y}px">
    {#if menu.kind === "silent"}
      {#each silentList as { input } (input.device)}
        <button onclick={() => (revealed = [...revealed, input.device])}>{input.label}</button>
      {/each}
      <button class="all" onclick={() => (showAll = true)}>{t("strip.showAllSilent")}</button>
    {:else}
      {#each outputs.filter((out) => hiddenOut.includes(out.label)) as out (out.label)}
        <button onclick={() => setHidden(out.label, false)}>{out.label}</button>
      {/each}
    {/if}
  </div>
{/if}

<div class="desk" class:compact>
  <div class="strips" bind:this={stripsEl}>
    <div class="group outs" class:left={side === "left"}>
      <h2>
        {t("settings.outputs")}
        {#if hiddenCount}
          <button class="more" onclick={(e) => openMenu(e, "outputs")}>{t("strip.hiddenOutputs", { count: hiddenCount })}</button>
        {/if}
      </h2>
      <div class="row">
        {#each outputs as output, o}
          {#if !hidden(o)}
          {@const m = output.master ?? { db: 0, muted: false }}
          <section class="strip panel">
            <div class="tagrow">
              <span class="tag">A{o + 1}</span>
              <button class="hide" title={t("strip.hideOutput")} aria-label={t("strip.hideOutput")} onclick={() => setHidden(output.label, true)}>×</button>
            </div>
            <div class="grow">
              <Fader
                label={output.label}
                note={compact ? "" : output.connected ? output.deviceName : output.released ? t("out.releasedShort") : t("out.waitingShort")}
                noteOn={output.connected}
                noteTitle={output.released ? t("out.released") : (output.deviceName ?? "")}
                value={m.db}
                muted={m.muted}
                levelDb={output.levelDb}
                connected={output.connected || output.released}
                postFader
                narrow={compact}
                onChange={(db) => onMaster(o, db)}
                onMute={(v) => onMasterMute(o, v)}
              />
            </div>
            <!-- Keeps the fader level with the inputs' "Effekte" row. -->
            <span class="fx spacer" aria-hidden="true">{t("fx.title")}</span>
          </section>
          {/if}
        {/each}
      </div>
    </div>
    {#if devices.length}
      <div class="group">
        <h2>{t("settings.title")}</h2>
        <div class="row">
          {#each devices as { input, i } (i)}{@render strip(input, i)}{/each}
        </div>
      </div>
    {/if}
    <div class="group">
      <h2>
        {t("programs.title")}
        <button class="add" onclick={onPrograms} title={t("programs.addHint")}>{t("programs.add")}</button>
        {#if silent}
          <button class="more" onclick={(e) => openMenu(e, "silent")}>{t("strip.showSilent", { count: silent })}</button>
        {/if}
      </h2>
      <div class="row">
        {#each programs as { input, i } (i)}
          {@render strip(input, i, true)}
        {:else}
          {#if !allPrograms.length}
            <p class="hint">{t("strip.programsHint")}</p>
          {/if}
        {/each}
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
    min-height: 28px;
    margin: 0;
    font-size: 15px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text);
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
    font-size: 12px;
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
    padding: 3px 10px;
    font-size: 12px;
    letter-spacing: 0;
    text-transform: none;
    color: var(--muted);
    background: none;
    border: 1px dashed var(--border);
    border-radius: var(--radius-sm);
  }
  .tagrow {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .hide {
    width: 20px;
    height: 20px;
    padding: 0;
    line-height: 1;
    color: var(--muted);
    background: none;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .menu {
    position: fixed;
    z-index: 20;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 4px;
    max-height: 60vh;
    overflow-y: auto;
  }
  .menu button {
    padding: 5px 12px;
    text-align: left;
    font-size: 13px;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
  }
  .menu button:hover {
    background: var(--panel-2);
  }
  .menu .all {
    color: var(--muted);
    border-top: 1px solid var(--border);
  }
  .foot {
    display: flex;
    gap: 4px;
  }
  .foot .fx {
    padding: 4px 8px;
  }
  .keep {
    display: grid;
    place-items: center;
    width: 26px;
    padding: 0;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--panel-2);
    color: var(--muted);
  }
  .keep svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.5;
    stroke-linejoin: round;
  }
  .keep.on {
    color: var(--accent);
    border-color: var(--accent);
  }
  .hint {
    width: 160px;
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }
  /* Compact: no group headings, tighter strips. */
  .compact h2 {
    display: none;
  }
  .compact .strip {
    gap: 6px;
    padding: 6px 8px;
  }
  .compact .strips,
  .compact .row {
    gap: 6px;
  }
  .compact .group + .group,
  .compact .outs:not(.left) {
    padding-left: 8px;
  }
  .compact .fx {
    padding: 2px 6px;
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
