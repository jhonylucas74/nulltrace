import React from "react";
import { Volume2, Wifi, Grid3X3, Palette, Terminal } from "lucide-react";
import type { WindowType } from "../contexts/WindowManagerContext";
import { LAUNCHABLE_APPS, getAppByType } from "./appList";

export interface StoreEntry {
  type: WindowType;
  name: string;
  description: string;
  icon: React.ReactNode;
}

/** Short descriptions for store listing (generic, no real brands). */
const DESCRIPTION_BY_TYPE: Partial<Record<WindowType, string>> = {
  terminal: "Command-line terminal for the system.",
  explorer: "Browse and manage files and folders.",
  browser: "Browse the web.",
  editor: "Edit code and text files.",
  theme: "Change desktop theme and appearance.",
  email: "Send and read mail.",
  wallet: "Manage balance and payments.",
  pixelart: "Create and edit pixel art.",
  sysinfo: "View system information.",
  shortcuts: "View and customize keyboard shortcuts.",
  sysmon: "Monitor CPU, memory, and processes.",
  pspy: "Spy on VM processes: view and inject stdin and stdout in real time.",
  nullcloud: "Cloud machines and VPS.",
  hackerboard: "Leaderboard and challenges.",
  startup: "Choose which apps start with the system.",
  wallpaper: "Set desktop background and grid.",
  settings: "Window and display preferences.",
  traceroute: "Trace network routes on a world map.",
  sound: "System sound mixer and volume.",
  network: "Network connections and status.",
  minesweeper: "Classic minesweeper game.",
};

function getDescription(type: WindowType): string {
  return DESCRIPTION_BY_TYPE[type] ?? "Official app.";
}

/** All apps shown in the Store (built-in + installable). */
export const STORE_CATALOG: StoreEntry[] = [
  ...LAUNCHABLE_APPS.map((app) => ({
    type: app.type,
    name: app.label,
    description: getDescription(app.type),
    icon: app.icon,
  })),
  {
    type: "sound",
    name: "Sound",
    description: "System sound mixer and volume.",
    icon: <Volume2 size={24} />,
  },
  {
    type: "network",
    name: "Network",
    description: "Network connections and status.",
    icon: <Wifi size={24} />,
  },
  {
    type: "minesweeper",
    name: "Minesweeper",
    description: "Classic minesweeper game.",
    icon: <Grid3X3 size={24} />,
  },
  {
    type: "pixelart",
    name: "Pixel Art",
    description: "Create and edit pixel art.",
    icon: <Palette size={24} />,
  },
  {
    type: "pspy",
    name: "Proc Spy",
    description: "Spy on VM processes: view and inject stdin and stdout in real time.",
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
