// Renders preview stills of a composition at the given frames, bundling once.
// Usage: node scripts/stills.mjs PromoVertical 0,30,60 [scale]
import { bundle } from "@remotion/bundler";
import { renderStill, selectComposition } from "@remotion/renderer";
import path from "node:path";

const [id = "PromoVertical", frames = "0", scale = "0.5"] = process.argv.slice(2);
const root = path.resolve(import.meta.dirname, "..");
const serveUrl = await bundle({ entryPoint: path.join(root, "src/index.ts"), publicDir: path.join(root, "public") });
const composition = await selectComposition({ serveUrl, id });
for (const f of frames.split(",").map(Number)) {
  const output = path.join(root, "out/stills", `${id}-${String(f).padStart(4, "0")}.jpg`);
  await renderStill({ serveUrl, composition, frame: f, output, imageFormat: "jpeg", jpegQuality: 85, scale: Number(scale) });
  console.log(output);
}
