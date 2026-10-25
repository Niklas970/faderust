// The user's colours: an accent for controls, a signal colour for meters and
// light or dark. Kept in the webview's storage, so they apply before the
// first frame and need nothing from the backend.

export const ACCENTS = [
  { name: "color.copper", color: "#d9834f" },
  { name: "color.amber", color: "#e0a43a" },
  { name: "color.red", color: "#e0605a" },
  { name: "color.magenta", color: "#d9609e" },
  { name: "color.violet", color: "#9b7ce8" },
  { name: "color.blue", color: "#4f9bd9" },
  { name: "color.green", color: "#5fb56a" },
  { name: "color.silver", color: "#a7b0ba" },
];

export const SIGNALS = [
  { name: "color.green", color: "#5ccf6a" },
  { name: "color.teal", color: "#3fc8b4" },
  { name: "color.blue", color: "#4fb3e8" },
  { name: "color.yellow", color: "#e6d04a" },
  { name: "color.white", color: "#d8dde3" },
];

export const MODES = [
  { value: "system", name: "mode.system" },
  { value: "dark", name: "mode.dark" },
  { value: "light", name: "mode.light" },
];

export const DEFAULT_THEME = {
  accent: ACCENTS[0].color,
  signal: SIGNALS[0].color,
  mode: "dark",
};

const KEY = "faderust.theme";
const HEX = /^#[0-9a-f]{6}$/i;

export function loadTheme() {
  try {
    const saved = JSON.parse(localStorage.getItem(KEY) ?? "{}");
    return {
      accent: HEX.test(saved.accent) ? saved.accent : DEFAULT_THEME.accent,
      signal: HEX.test(saved.signal) ? saved.signal : DEFAULT_THEME.signal,
      mode: MODES.some((m) => m.value === saved.mode)
        ? saved.mode
        : DEFAULT_THEME.mode,
    };
  } catch {
    return { ...DEFAULT_THEME };
  }
}

export function applyTheme(theme) {
  const root = document.documentElement;
  root.style.setProperty("--accent-base", theme.accent);
  root.style.setProperty("--signal-base", theme.signal);
  if (theme.mode === "system") delete root.dataset.mode;
  else root.dataset.mode = theme.mode;
}

export function saveTheme(theme) {
  applyTheme(theme);
  try {
    localStorage.setItem(KEY, JSON.stringify(theme));
  } catch {
    // Storage blocked: the colours still apply until the app closes.
  }
}
