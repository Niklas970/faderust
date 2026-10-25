// Interface texts in German and English. `t("key", { name: value })` looks a
// text up in the current language; reading it in a template makes the page
// follow a language change.
import { setBackendLanguage } from "./api.js";
import de from "./locales/de.js";
import en from "./locales/en.js";

const LANGS = { de, en };
const KEY = "faderust.lang";

function detect() {
  try {
    const saved = localStorage.getItem(KEY);
    if (saved && saved in LANGS) return saved;
  } catch {
    // No stored choice.
  }
  const system = typeof navigator === "undefined" ? "" : (navigator.language ?? "");
  return system.toLowerCase().startsWith("de") ? "de" : "en";
}

let lang = $state(detect());

if (typeof document !== "undefined") document.documentElement.lang = lang;
setBackendLanguage(lang).catch(() => {});

/** The current language, "de" or "en". */
export function getLang() {
  return lang;
}

export function setLang(next) {
  if (!(next in LANGS)) return;
  lang = next;
  if (typeof document !== "undefined") document.documentElement.lang = next;
  setBackendLanguage(next).catch(() => {});
  try {
    localStorage.setItem(KEY, next);
  } catch {
    // Remembered for this session only.
  }
}

export const LANGUAGES = [
  { value: "de", name: "Deutsch" },
  { value: "en", name: "English" },
];

/** Text for `key`; `{name}` placeholders take their value from `params`. */
export function t(key, params) {
  const text = LANGS[lang][key] ?? LANGS.de[key] ?? key;
  return params ? text.replace(/\{(\w+)\}/g, (_, k) => String(params[k] ?? "")) : text;
}
