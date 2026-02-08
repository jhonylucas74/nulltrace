/**
 * Generates ASCII art from nulltrace_icon.png: resizes the image then maps
 * pixel luminance to characters. Updates src/lib/nulltraceAsciiArt.ts automatically.
 * Run: node scripts/generate-ascii-art.mjs
 */

import { readFile, writeFile } from "fs/promises";
import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { Jimp } from "jimp";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "..");
const IMAGE_PATH = join(ROOT, "public", "nulltrace_icon.png");
const OUTPUT_TS = join(ROOT, "src", "lib", "nulltraceAsciiArt.ts");

// Character set dark→light; opacity 0 → space for both versions
const CHARS = " .:-=+*#%@";
const NUM_CHARS = CHARS.length;

async function main() {
  const buf = await readFile(IMAGE_PATH);
  const image = await Jimp.read(buf);

  // Resize to a small width; height 30% smaller than proportional
  const TARGET_WIDTH = 36;
  const HEIGHT_SCALE = 0.7; // 30% shorter
  const w = image.bitmap.width;
  const h = image.bitmap.height;
  const targetHeight = Math.max(1, Math.round((TARGET_WIDTH * h) / w * HEIGHT_SCALE));
  image.resize({ w: TARGET_WIDTH, h: targetHeight });

  const linesNormal = [];
  const linesInverted = [];
  for (let y = 0; y < targetHeight; y++) {
    let lineNorm = "";
    let lineInv = "";
    for (let x = 0; x < TARGET_WIDTH; x++) {
      const idx = (y * TARGET_WIDTH + x) << 2;
      const r = image.bitmap.data[idx];
      const g = image.bitmap.data[idx + 1];
      const b = image.bitmap.data[idx + 2];
      const a = image.bitmap.data[idx + 3];
      if (a === 0) {
        lineNorm += CHARS[0];
        lineInv += CHARS[0];
        continue;
      }
      const luminance = a < 128 ? 0 : Math.round(0.299 * r + 0.587 * g + 0.114 * b);
      let charIndex = Math.min(
        Math.floor((luminance / 256) * NUM_CHARS),
        NUM_CHARS - 1
      );
      lineNorm += CHARS[charIndex];
      lineInv += CHARS[NUM_CHARS - 1 - charIndex];
    }
    linesNormal.push(lineNorm);
    linesInverted.push(lineInv);
  }

  const artNormal = linesNormal.join("\n");
  const artInverted = linesInverted.join("\n");
  const tsContent = `/**
 * Pre-generated ASCII art from nulltrace_icon.png (resized, height scale ${HEIGHT_SCALE}, then luminance-to-char).
 * Normal + inverted; regenerate with: node scripts/generate-ascii-art.mjs
 */
export const NULLTRACE_ASCII_ART = \`
${artNormal}
\`;

export const NULLTRACE_ASCII_ART_INVERTED = \`
${artInverted}
\`;
`;
  await writeFile(OUTPUT_TS, tsContent, "utf8");
  console.log("Updated", OUTPUT_TS);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
