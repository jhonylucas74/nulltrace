import React from "react";
import { Volume2, Wifi, Grid3X3, Palette, Terminal } from "lucide-react";
import type { WindowType } from "../contexts/WindowManagerContext";
import { LAUNCHABLE_APPS, getAppByType, getDescKey } from "./appList";

export interface StoreEntry {
  type: WindowType;
  name: string;
  description: string;
  /** Key in apps namespace for display name (e.g. terminal, files). */
  nameKey: string;
  /** Key in apps namespace for description (e.g. desc_terminal, desc_files). */
  descKey: string;
  icon: React.ReactNode;
}

/** All apps shown in the Store (built-in + installable). Names and descriptions are resolved via apps namespace (nameKey, descKey). */
export const STORE_CATALOG: StoreEntry[] = [
  ...LAUNCHABLE_APPS.map((app) => ({
    type: app.type,
    name: app.label,
    description: "",
    nameKey: app.labelKey,
    descKey: getDescKey(app.type),
    icon: app.icon,
  })),
  {
    type: "sound",
    name: "Sound",
    description: "",
    nameKey: "sound",
    descKey: "desc_sound",
    icon: <Volume2 size={24} />,
  },
  {
    type: "network",
    name: "Network",
    description: "",
    nameKey: "network",
    descKey: "desc_network",
    icon: <Wifi size={24} />,
  },
  {
    type: "minesweeper",
    name: "Minesweeper",
    description: "",
    nameKey: "minesweeper",
    descKey: "desc_minesweeper",
    icon: <Grid3X3 size={24} />,
  },
  {
    type: "pixelart",
    name: "Pixel Art",
    description: "",
    nameKey: "pixel_art",
    descKey: "desc_pixel_art",
    icon: <Palette size={24} />,
  },
  {
    type: "pspy",
    name: "Proc Spy",
    description: "",
    nameKey: "proc_spy",
    descKey: "desc_proc_spy",
    icon: <Terminal size={24} />,
  },
];

/** Whether this app type is in the default launcher (no install needed). */
export function isBuiltInLauncherApp(type: WindowType): boolean {
  return getAppByType(type) != null;
}

/** Basic/system apps hidden from Discover (shown only in Installed). */
export const DISCOVER_HIDDEN_TYPES: WindowType[] = ["sound", "network", "settings", "store"];

/** Whether this app type should be hidden in the Discover tab. */
export function isHiddenFromDiscover(type: WindowType): boolean {
  return DISCOVER_HIDDEN_TYPES.includes(type);
}
