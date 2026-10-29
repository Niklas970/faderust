<script>
  import { getWindowsAudio, setDefaultDevice, setDeviceVolume } from "./api.js";
  import Volume from "./Volume.svelte";
  import { t } from "./i18n.svelte.js";

  /** The Windows sound settings: default devices and device volumes. */
  let { onClose } = $props();

  let data = $state(null);
  let error = $state("");

  async function refresh() {
    try {
      data = await getWindowsAudio();
      error = "";
    } catch (e) {
      error = String(e);
    }
  }
  refresh();

  async function makeDefault(id, communications) {
    try {
      await setDefaultDevice(id, communications);
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }

  async function volume(id, v, m) {
    try {
      await setDeviceVolume(id, v, m);
    } catch (e) {
      error = String(e);
    }
  }

  const sections = [
    { key: "playback", title: "win.playback" },
    { key: "recording", title: "win.recording" },
  ];
</script>

<div class="backdrop" role="presentation" onclick={onClose}></div>
<div class="dialog" role="dialog" aria-modal="true" aria-label={t("win.title")}>
  <header>
    <h2>{t("win.title")}</h2>
    <button onclick={refresh}>{t("common.refresh")}</button>
  </header>

  {#if error}<p class="error">{error}</p>{/if}

  {#if data && !data.supported}
    <p class="note">{t("win.unsupported")}</p>
  {:else if data}
    {#each sections as s}
      {@const list = data[s.key]}
      <section>
        <h3>{t(s.title)}</h3>
        <div class="defaults">
          <label>
            <span>{t("win.defaultDevice")}</span>
            <select
              value={list.find((d) => d.isDefault)?.id ?? ""}
              onchange={(e) => makeDefault(e.currentTarget.value, false)}
            >
              {#if !list.some((d) => d.isDefault)}<option value="">{t("win.none")}</option>{/if}
              {#each list as d}<option value={d.id}>{d.name}</option>{/each}
            </select>
          </label>
          <label>
            <span>{t("win.forCalls")}</span>
            <select
              value={list.find((d) => d.isDefaultCommunications)?.id ?? ""}
              onchange={(e) => makeDefault(e.currentTarget.value, true)}
            >
              {#if !list.some((d) => d.isDefaultCommunications)}<option value="">{t("win.none")}</option>{/if}
              {#each list as d}<option value={d.id}>{d.name}</option>{/each}
            </select>
          </label>
        </div>
        <ul>
          {#each list as d (d.id)}
            <li>
              <span class="name" title={d.name}>
                {d.name}
                {#if d.isDefault}<em>{t("win.badgeDefault")}</em>{/if}
                {#if d.isDefaultCommunications && !d.isDefault}<em>{t("win.badgeCalls")}</em>{/if}
              </span>
              <Volume
                volume={d.volume}
                muted={d.muted}
                label={d.name}
                onChange={(v, m) => volume(d.id, v, m)}
              />
            </li>
          {/each}
        </ul>
      </section>
    {/each}
    <p class="note small">
      {t("win.note")}
    </p>
  {/if}

  <footer>
    <button onclick={onClose}>{t("common.close")}</button>
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
    width: min(680px, calc(100vw - 32px));
    max-height: calc(100vh - 32px);
    overflow: auto;
    padding: 20px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }
  h2 {
    margin: 0;
    font-size: 18px;
  }
  h3 {
    margin: 16px 0 8px;
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--accent);
  }
  .defaults {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    margin-bottom: 12px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    font-size: 12px;
    color: var(--muted);
  }
  select {
    min-width: 0;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--panel-2);
    color: var(--text);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1.2fr);
    align-items: center;
    gap: 12px;
    margin-bottom: 8px;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  em {
    margin-left: 6px;
    padding: 1px 6px;
    border-radius: var(--radius-sm);
    font-style: normal;
    font-size: 12px;
    color: var(--signal);
    border: 1px solid var(--signal);
  }
  .note {
    color: var(--muted);
  }
  .small {
    font-size: 12px;
    margin-top: 16px;
  }
  .error {
    color: var(--danger);
  }
  button {
    padding: 6px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--panel-2);
    white-space: nowrap;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    margin-top: 16px;
  }
  @media (max-width: 520px) {
    .defaults,
    li {
      grid-template-columns: 1fr;
    }
  }
</style>
