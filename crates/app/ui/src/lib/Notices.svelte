<script>
  import { onMount } from "svelte";
  import { onNotice } from "./api.js";

  /** Device changes and errors from the engine, fading out on their own. */
  let items = $state([]);
  let next = 0;

  function dismiss(id) {
    items = items.filter((n) => n.id !== id);
  }

  onMount(() => {
    let unlisten;
    onNotice((n) => {
      const id = next++;
      items = [...items.slice(-3), { ...n, id }];
      setTimeout(() => dismiss(id), n.level === "error" ? 15000 : 6000);
    }).then((u) => (unlisten = u));
    return () => unlisten?.();
  });
</script>

<div class="notices" aria-live="polite">
  {#each items as n (n.id)}
    <div class="notice {n.level}">
      <span>{n.text}</span>
      <button title="Schließen" onclick={() => dismiss(n.id)}>✕</button>
    </div>
  {/each}
</div>

<style>
  .notices {
    position: fixed;
    right: 16px;
    bottom: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: min(380px, calc(100vw - 32px));
    z-index: 20;
  }
  .notice {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-left: 4px solid var(--teal);
    border-radius: var(--radius-sm);
    box-shadow: 0 6px 18px rgb(0 0 0 / 0.3);
    line-height: 1.4;
  }
  .notice.warn {
    border-left-color: var(--copper);
  }
  .notice.error {
    border-left-color: var(--danger);
  }
  span {
    flex: 1;
  }
  button {
    border: 0;
    background: none;
    color: var(--muted);
    padding: 0 2px;
  }
</style>
