import { t } from "./i18n.svelte.js";
import { ownCable } from "./api.js";

/**
 * What Windows calls "Speakers (Faderust Cable 2)" reads as "Faderust Kabel 2
 * (Wiedergabe)": the cable's name first, then which end it is. Other devices
 * keep their name.
 */
export function deviceLabel(name) {
  const own = ownCable(name);
  if (own === null) return name;
  const n = own.match(/(\d+)\s*$/)?.[1];
  const nr = n && n !== "1" ? ` ${n}` : "";
  const head = name.slice(0, name.toLowerCase().lastIndexOf("(faderust cable")).trim();
  if (/^(speakers|lautsprecher|headphones|kopfhörer|line|digital|playback)/i.test(head)) return t("cable.playback", { n: nr });
  if (/^(microphone|mikrofon|recording)/i.test(head)) return t("cable.recording", { n: nr });
  return t("cable.plain", { n: nr });
}
