<script>
  import { t } from "./i18n.svelte.js";
  import { getUsbCable, setUsbCable, setupUsbCable } from "./api.js";

  /** Sets up (or just turns on) Faderust's own USB cable; hidden when it runs. */
  let usb = $state(null);
  let busy = $state(false);
  let error = $state("");
  const look = () => getUsbCable().then((u) => (usb = u), () => {});
  look();

  async function go() {
    busy = true;
    try {
      if (!usb.installed) await setupUsbCable();
      await setUsbCable(true);
      error = "";
    } catch (e) {
      error = String(e);
    }
    busy = false;
    look();
  }
</script>

{#if usb?.supported && !usb.running}
  <button class="primary" disabled={busy} onclick={go}>
    {busy ? t("usbcable.working") : t("usbcable.setupButton")}
  </button>
  {#if error}<span class="error">{error}</span>{/if}
{:else if usb?.running && !usb.attached.some(Boolean)}
  <span>{t("usbcable.waiting")}</span>
{/if}

<style>
  .error {
    color: var(--danger);
  }
</style>
