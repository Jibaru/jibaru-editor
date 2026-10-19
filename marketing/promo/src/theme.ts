import { loadFont as loadSpaceGrotesk } from "@remotion/google-fonts/SpaceGrotesk";
import { loadFont as loadInter } from "@remotion/google-fonts/Inter";
import { loadFont as loadMono } from "@remotion/google-fonts/JetBrainsMono";

const display = loadSpaceGrotesk("normal", { weights: ["500", "700"], subsets: ["latin"] });
const ui = loadInter("normal", { weights: ["400", "500", "600", "700", "800"], subsets: ["latin"] });
const mono = loadMono("normal", { weights: ["400", "700"], subsets: ["latin"] });

export const fonts = {
  display: display.fontFamily,
  ui: ui.fontFamily,
  mono: mono.fontFamily,
};

/** Editor palette (matches the desktop app's dark theme). */
export const C = {
  bg: "#0b0d0f",
  gutter: "#0f1214",
  panel: "#181c1f",
  panel2: "#20262a",
  line: "#2a3135",
  text: "#e8eef0",
  dim: "#8a979c",
  accent: "#3fd6b2",
  accentDeep: "#1f5c50",
  clip: "#1f5c50",
  clipHeader: "#2a7a69",
  audio: "#1d4f45",
  playhead: "#ff6161",
  white: "#ffffff",
};

export const FPS = 30;
