<script>
  /** Level in dBFS. Shown from -60 dB (empty) to 0 dB (full). */
  let { db = -120, vertical = true, thin = false } = $props();

  const pct = $derived(Math.max(0, Math.min(100, ((db + 60) / 60) * 100)));
  const hot = $derived(db > -3);
</script>

<div class="meter" class:vertical class:thin aria-hidden="true">
  <div class="fill" class:hot style={vertical ? `height:${pct}%` : `width:${pct}%`}></div>
</div>

<style>
  .meter {
    position: relative;
    background: var(--panel-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    overflow: hidden;
  }
  .meter.vertical {
    width: 10px;
    height: 100%;
  }
  .meter.vertical.thin {
    width: 6px;
  }
  .meter:not(.vertical) {
    height: 6px;
    width: 100%;
  }
  .fill {
    position: absolute;
    left: 0;
    bottom: 0;
    background: var(--teal);
    transition: height 60ms linear, width 60ms linear;
  }
  .vertical .fill {
    width: 100%;
  }
  .meter:not(.vertical) .fill {
    height: 100%;
    top: 0;
  }
  .fill.hot {
    background: var(--copper);
  }
</style>
