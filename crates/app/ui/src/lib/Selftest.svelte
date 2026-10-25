<script>
  import { runSelftest } from "./api.js";
  import { t } from "./i18n.svelte.js";

  /** Looks at devices, buffers, remote control and plugin and lists the result. */
  let { onClose } = $props();

  let checks = $state(null);
  let error = $state("");
  let copied = $state(false);

  async function run() {
    checks = null;
    copied = false;
    try {
      checks = await runSelftest();
      error = "";
    } catch (e) {
      error = String(e);
    }
  }
  run();

  const MARK = { ok: "✓", warn: "!", fail: "✕", info: "i" };
  const problems = $derived(checks?.filter((c) => c.status === "warn" || c.status === "fail").length ?? 0);

  // Plain text for an issue or a message.
  async function copy() {
    const text = checks.map((c) => `[${c.status.toUpperCase()}] ${c.title}: ${c.detail}`).join("\n");
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="backdrop" role="presentation" onclick={onClose}></div>
<div class="dialog" role="dialog" aria-modal="true" aria-label={t("selftest.title")}>
  <header>
    <h2>{t("selftest.title")}</h2>
    <button onclick={run}>{t("selftest.again")}</button>
  </header>

  {#if error}<p class="error">{error}</p>{/if}

  {#if checks}
    <p class="summary" class:good={!problems}>
      {problems ? t("selftest.problems", { count: problems }) : t("selftest.allGood")}
    </p>
    <ul>
      {#each checks as c}
        <li class={c.status}>
          <span class="mark" aria-label={c.status}>{MARK[c.status]}</span>
          <div>
            <strong>{c.title}</strong>
            <span class="detail">{c.detail}</span>
          </div>
        </li>
      {/each}
    </ul>
  {:else if !error}
    <p class="note">{t("selftest.running")}</p>
  {/if}

  <footer>
    <button onclick={copy} disabled={!checks}>{copied ? t("selftest.copied") : t("selftest.copy")}</button>
    <button class="primary" onclick={onClose}>{t("common.close")}</button>
  </footer>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.5);
    z-index: 30;
  }
  .dialog {
    position: fixed;
    z-index: 31;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: min(640px, calc(100vw - 32px));
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
  .summary {
    margin: 0 0 12px;
    color: var(--danger);
    font-weight: 600;
  }
  .summary.good {
    color: var(--signal);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr);
    gap: 10px;
    padding: 8px 0;
    border-top: 1px solid var(--border);
  }
  .mark {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border: 1px solid currentColor;
    border-radius: 50%;
    font-size: 12px;
    font-weight: 700;
  }
  .ok .mark {
    color: var(--signal);
  }
  .warn .mark {
    color: #e0a43a;
  }
  .fail .mark {
    color: var(--danger);
  }
  .info .mark {
    color: var(--muted);
  }
  strong {
    display: block;
    font-size: 13px;
  }
  .detail {
    display: block;
    margin-top: 2px;
    font-size: 12px;
    color: var(--muted);
  }
  .note {
    color: var(--muted);
  }
  .error {
    color: var(--danger);
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }
  button {
    padding: 6px 14px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--panel-2);
    color: var(--text);
  }
  button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
</style>
