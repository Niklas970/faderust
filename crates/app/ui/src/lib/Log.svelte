<script>
  import { getLog, openLogFolder } from "./api.js";
  import { t } from "./i18n.svelte.js";
  import { tick } from "svelte";

  /** The end of faderust.log, newest at the bottom; to copy into an issue. */
  let { onSelftest, onClose } = $props();

  let text = $state("");
  let error = $state("");
  let copied = $state(false);
  let onlyProblems = $state(false);
  let box = $state();

  // What the log writes for failures (German and English messages).
  const PROBLEM = /fehl|fehler|error|fail|panic|abbruch|absturz|denied|refused|timeout/i;
  const lines = $derived(text ? text.split("\n") : []);
  const shown = $derived(onlyProblems ? lines.filter((l) => PROBLEM.test(l)) : lines);

  async function load() {
    copied = false;
    try {
      text = await getLog(400);
      error = "";
      await tick();
      if (box) box.scrollTop = box.scrollHeight;
    } catch (e) {
      error = String(e);
    }
  }
  load();

  async function copy() {
    try {
      await navigator.clipboard.writeText(shown.join("\n"));
      copied = true;
    } catch (e) {
      error = String(e);
    }
  }

  async function folder() {
    try {
      await openLogFolder();
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="backdrop" role="presentation" onclick={onClose}></div>
<div class="dialog" role="dialog" aria-modal="true" aria-label={t("log.title")}>
  <header>
    <h2>{t("log.title")}</h2>
    <label class="only"><input type="checkbox" bind:checked={onlyProblems} /> {t("log.onlyProblems")}</label>
  </header>
  <p class="note">{t("log.hint")}</p>

  {#if error}<p class="error">{error}</p>{/if}

  <pre bind:this={box} class="log" tabindex="0">{shown.length ? shown.join("\n") : t("log.empty")}</pre>
  <p class="count">{t("log.lines", { count: shown.length })}</p>

  <footer>
    <button onclick={load}>{t("log.refresh")}</button>
    <button onclick={folder}>{t("log.folder")}</button>
    <button onclick={onSelftest}>{t("log.selftest")}</button>
    <span class="grow"></span>
    <button onclick={copy} disabled={!shown.length}>{copied ? t("log.copied") : t("log.copy")}</button>
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
    width: min(760px, calc(100vw - 32px));
    max-height: calc(100vh - 32px);
    display: flex;
    flex-direction: column;
    padding: 20px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  h2 {
    margin: 0;
    font-size: 18px;
  }
  .only {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--muted);
  }
  .note {
    margin: 6px 0 10px;
    font-size: 13px;
    color: var(--muted);
  }
  .error {
    color: var(--danger);
  }
  .log {
    flex: 1;
    min-height: 200px;
    max-height: 55vh;
    margin: 0;
    padding: 10px;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-word;
    font-family: var(--mono);
    font-size: 12px;
    line-height: 1.5;
    color: var(--text);
    background: var(--well);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    user-select: text;
  }
  .count {
    margin: 6px 0 0;
    font-size: 12px;
    color: var(--muted);
  }
  footer {
    display: flex;
    gap: 8px;
    margin-top: 14px;
  }
  .grow {
    flex: 1;
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
