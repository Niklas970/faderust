<script>
  import { t } from "./i18n.svelte.js";
  import { deletePreset, getConfig, loadPreset, savePreset } from "./api.js";

  /** Menu to store and recall fader positions. */
  let { onError, onClose } = $props();

  let presets = $state([]);
  let name = $state("");

  async function refresh() {
    presets = (await getConfig()).presets ?? [];
  }
  refresh();

  async function run(action) {
    try {
      await action();
      await refresh();
      onError("");
    } catch (e) {
      onError(String(e));
    }
  }

  function save(e) {
    e.preventDefault();
    if (!name.trim()) return;
    run(async () => {
      await savePreset(name.trim());
      name = "";
    });
  }
</script>

<div class="backdrop" role="presentation" onclick={onClose}></div>
<div class="menu" role="dialog" aria-label={t("nav.presets")}>
  {#if presets.length}
    <ul>
      {#each presets as p (p.name)}
        <li>
          <button class="load" title={t("presets.load")} onclick={() => run(() => loadPreset(p.name)).then(onClose)}>{p.name}</button>
          <button class="icon" title={t("presets.overwrite")} onclick={() => run(() => savePreset(p.name))}>⤓</button>
          <button class="icon" title={t("presets.delete")} onclick={() => run(() => deletePreset(p.name))}>✕</button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="empty">{t("presets.empty")}</p>
  {/if}
  <form onsubmit={save}>
    <input bind:value={name} placeholder={t("presets.placeholder")} aria-label={t("presets.name")} />
    <button type="submit" class="primary" disabled={!name.trim()}>{t("common.save")}</button>
  </form>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
  }
  .menu {
    position: fixed;
    top: 52px;
    right: 16px;
    width: 300px;
    padding: 10px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.35);
    z-index: 10;
  }
  ul {
    list-style: none;
    margin: 0 0 8px;
    padding: 0;
  }
  li {
    display: flex;
    gap: 4px;
    margin-bottom: 4px;
  }
  .load {
    flex: 1;
    text-align: left;
  }
  .empty {
    margin: 4px 0 10px;
    color: var(--muted);
    font-size: 13px;
  }
  form {
    display: flex;
    gap: 6px;
  }
  input {
    flex: 1;
    min-width: 0;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--panel-2);
  }
  button {
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--panel-2);
  }
  button:disabled {
    opacity: 0.5;
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
</style>
