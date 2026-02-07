/**
 * Pixel art data model and helpers for the in-game Pixel Art app.
 * Saved as JSON via the virtual file system (width, height, pixels matrix).
 */

export interface PixelArtData {
  width: number;
  height: number;
  /** pixels[y][x] = "#rrggbb" */
  pixels: string[][];
}

export const CANVAS_SIZES = [16, 32] as const;

/** Default palette: black, white, grays, and a few primaries (in-game, no real brands). */
export const DEFAULT_PALETTE: string[] = [
  "#000000",
  "#ffffff",
  "#888888",
  "#cccccc",
  "#444444",
  "#c0392b",
  "#27ae60",
  "#2980b9",
  "#f39c12",
  "#8e44ad",
  "#1abc9c",
  "#e74c3c",
  "#2ecc71",
  "#3498db",
  "#e67e22",
  "#9b59b6",
];

/** Named palette for the palette selector (inspired by common pixel-art styles). */
export interface PaletteEntry {
  id: string;
  name: string;
  colors: string[];
}

/** At least 10 palettes for the app selector (generic names, no real brands). */
export const PALETTES: PaletteEntry[] = [
  {
    id: "default",
    name: "Default",
    colors: DEFAULT_PALETTE,
  },
  {
    id: "pastel",
    name: "Pastel",
    colors: [
      "#fef6e4", "#f8d5a3", "#f4bc7a", "#e8a87c", "#c38d9e",
      "#a8d5ba", "#7fcdc7", "#6bb3bf", "#e8d5b7", "#d4a5a5",
      "#9f7a9f", "#6c5b7b", "#35477d", "#f5e6d3", "#dfe0e0",
    ],
  },
  {
    id: "ega",
    name: "EGA 16",
    colors: [
      "#000000", "#0000aa", "#00aa00", "#00aaaa", "#aa0000",
      "#aa00aa", "#aa5500", "#aaaaaa", "#555555", "#5555ff",
      "#55ff55", "#55ffff", "#ff5555", "#ff55ff", "#ffff55",
      "#ffffff",
    ],
  },
  {
    id: "neon",
    name: "Neon",
    colors: [
      "#0d0221", "#ff3864", "#2de2e6", "#ff6c11", "#f9f002",
      "#7b2cbf", "#00f5d4", "#e63946", "#06ffa5", "#9b5de5",
      "#00bbf9", "#fee440", "#9b59b6", "#1dd1a1", "#ee5a24",
      "#ffffff",
    ],
  },
  {
    id: "gameboy",
    name: "Pocket (4-shade)",
    colors: ["#0f380f", "#306230", "#8bac0f", "#9bbc0f"],
  },
  {
    id: "warm",
    name: "Warm",
    colors: [
      "#1a0a0a", "#4a1c1c", "#8b4513", "#cd853f", "#deb887",
      "#ff6347", "#ff4500", "#ffa500", "#ffd700", "#f4a460",
      "#d2691e", "#b22222", "#8b0000", "#fff8dc", "#faebd7",
      "#ffffff",
    ],
  },
  {
    id: "cool",
    name: "Cool",
    colors: [
      "#0a0a1a", "#1c1c4a", "#191970", "#4169e1", "#87ceeb",
      "#00ced1", "#20b2aa", "#48d1cc", "#7b68ee", "#9370db",
      "#ba55d3", "#dda0dd", "#e6e6fa", "#b0c4de", "#add8e6",
      "#ffffff",
    ],
  },
  {
    id: "earth",
    name: "Earth",
    colors: [
      "#2d1b0e", "#5c4033", "#8b7355", "#a0826d", "#c4a77d",
      "#3d5c2e", "#6b8e23", "#9acd32", "#556b2f", "#2e4a1f",
      "#8b4513", "#a0522d", "#d2691e", "#daa520", "#f5deb3",
      "#faf0e6",
    ],
  },
  {
    id: "grayscale",
    name: "Grayscale",
    colors: [
      "#000000", "#111111", "#222222", "#333333", "#444444",
      "#555555", "#666666", "#777777", "#888888", "#999999",
      "#aaaaaa", "#bbbbbb", "#cccccc", "#dddddd", "#eeeeee",
      "#ffffff",
    ],
  },
  {
    id: "sunset",
    name: "Sunset",
    colors: [
      "#2c1810", "#4a2c2a", "#6b3a2e", "#8b4513", "#cd853f",
      "#daa520", "#ff8c00", "#ff6347", "#ff4500", "#dc143c",
      "#b22222", "#8b0000", "#4a0a0a", "#f4a460", "#ffe4b5",
      "#fff5ee",
    ],
  },
  {
    id: "ocean",
    name: "Ocean",
    colors: [
      "#001a1a", "#003333", "#004d4d", "#006666", "#008080",
      "#20b2aa", "#40e0d0", "#00ced1", "#4682b4", "#5f9ea0",
      "#87ceeb", "#b0e0e6", "#add8e6", "#e0ffff", "#f0f8ff",
      "#ffffff",
    ],
  },
  {
    id: "forest",
    name: "Forest",
    colors: [
      "#0d1f0d", "#1b3d1b", "#2d5a2d", "#3d7a3d", "#4a904a",
      "#5fa85f", "#6b9e6b", "#228b22", "#2e8b2e", "#32cd32",
      "#3cb371", "#2e4a2e", "#1a3d1a", "#8fbc8f", "#90ee90",
      "#98fb98",
    ],
  },
  {
    id: "candy",
    name: "Candy",
    colors: [
      "#fff0f5", "#ffe4ec", "#ffc0cb", "#ffb6c1", "#ff69b4",
      "#ff1493", "#db7093", "#c71585", "#98fb98", "#90ee90",
      "#7fffd4", "#40e0d0", "#e0ffff", "#add8e6", "#dda0dd",
      "#ffffff",
    ],
  },
  {
    id: "vintage",
    name: "Vintage",
    colors: [
      "#2c1810", "#3d2817", "#4a3728", "#5c4033", "#6b5344",
      "#8b7355", "#a0826d", "#c4a77d", "#d4b896", "#e8d5b7",
      "#8b6914", "#9b7a2e", "#b8860b", "#daa520", "#f5deb3",
      "#faf0e6",
    ],
  },
];

const DEFAULT_FILL = "#ffffff";

export function createEmptyData(
  width: number,
  height: number,
  fill: string = DEFAULT_FILL
): PixelArtData {
  const pixels: string[][] = [];
  for (let y = 0; y < height; y++) {
    pixels[y] = Array.from({ length: width }, () => fill);
  }
  return { width, height, pixels };
}

export function serializePixelArt(data: PixelArtData): string {
  return JSON.stringify(data);
}

export function parsePixelArt(json: string): PixelArtData | null {
  try {
    const obj = JSON.parse(json);
    if (
      typeof obj?.width !== "number" ||
      typeof obj?.height !== "number" ||
      !Array.isArray(obj?.pixels)
    )
      return null;
    const h = obj.height;
    const w = obj.width;
    if (h < 1 || w < 1 || obj.pixels.length !== h) return null;
    const pixels = obj.pixels as unknown[];
    for (let y = 0; y < h; y++) {
      const row = pixels[y];
      if (!Array.isArray(row) || row.length !== w) return null;
      for (let x = 0; x < w; x++) {
        const v = row[x];
        if (typeof v !== "string" || !/^#[0-9a-fA-F]{6}$/.test(v)) return null;
      }
    }
    return { width: w, height: h, pixels: pixels as string[][] };
  } catch {
    return null;
  }
}

/**
 * Render pixel art data to a base64 PNG data URL.
 * Uses a tiny canvas (size is 16/32), so it's lightweight.
 */
export function renderPixelArtToDataUrl(data: PixelArtData): string {
  const canvas = document.createElement("canvas");
  canvas.width = data.width;
  canvas.height = data.height;
  const ctx = canvas.getContext("2d");
  if (!ctx) return "";
  const image = ctx.createImageData(data.width, data.height);
  let i = 0;
  for (let y = 0; y < data.height; y++) {
    for (let x = 0; x < data.width; x++) {
      const hex = data.pixels[y][x] ?? "#ffffff";
      const r = parseInt(hex.slice(1, 3), 16);
      const g = parseInt(hex.slice(3, 5), 16);
      const b = parseInt(hex.slice(5, 7), 16);
      image.data[i++] = r;
      image.data[i++] = g;
      image.data[i++] = b;
      image.data[i++] = 255;
    }
  }
  ctx.putImageData(image, 0, 0);
  return canvas.toDataURL("image/png");
}
