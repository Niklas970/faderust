// Renders icons/*.svg to the PNGs in the plugin folder (1x and @2x).
// Needs Playwright with Chromium: node scripts/icons.cjs
const fs = require("node:fs");
const path = require("node:path");
const { chromium } = require("playwright");

const root = path.join(__dirname, "..");
const imgs = path.join(root, "com.niklas970.faderust.sdPlugin", "imgs");

// source → [target without extension, 1x size]
const icons = {
  "plugin-icon": ["plugin/icon", 256],
  category: ["plugin/category", 28],
  "volume-icon": ["actions/volume/icon", 20],
  "volume-key": ["actions/volume/key", 72],
  "mute-icon": ["actions/mute/icon", 20],
  "mute-on": ["actions/mute/on", 72],
  "mute-off": ["actions/mute/off", 72],
  "preset-icon": ["actions/preset/icon", 20],
  "preset-key": ["actions/preset/key", 72],
  "preset-active": ["actions/preset/active", 72],
  "device-icon": ["actions/device/icon", 20],
  "device-key": ["actions/device/key", 72],
  "device-active": ["actions/device/active", 72],
};

(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  for (const [src, [target, size]] of Object.entries(icons)) {
    const svg = fs.readFileSync(path.join(root, "icons", `${src}.svg`), "utf8");
    for (const [scale, suffix] of [[1, ""], [2, "@2x"]]) {
      const px = size * scale;
      await page.setViewportSize({ width: px, height: px });
      await page.setContent(
        `<style>html,body{margin:0;background:transparent}svg{display:block;width:${px}px;height:${px}px}</style>${svg}`,
      );
      const file = path.join(imgs, `${target}${suffix}.png`);
      fs.mkdirSync(path.dirname(file), { recursive: true });
      await page.locator("svg").screenshot({ path: file, omitBackground: true });
    }
  }
  await browser.close();
})();
