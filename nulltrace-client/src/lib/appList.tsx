import React from "react";
import type { TFunction } from "i18next";
import { Palette, Cpu, Keyboard, Activity, Cloud, Trophy, Rocket, Image, Settings, Wallet, Route, ShoppingBag, Package, GraduationCap, HardDrive } from "lucide-react";
import type { WindowType } from "../contexts/WindowManagerContext";

/** Maps WindowType to apps.json label key (e.g. explorer -> "files"). */
export const APP_LABEL_KEY: Record<WindowType, string> = {
  terminal: "terminal",
  explorer: "files",
  browser: "browser",
  editor: "code",
  theme: "theme",
  email: "mail",
  wallet: "wallet",
  sysinfo: "nullfetch",
  shortcuts: "shortcuts",
  sysmon: "system_monitor",
  nullcloud: "nullcloud",
  hackerboard: "hackerboard",
  startup: "startup",
  wallpaper: "background",
  settings: "settings",
  traceroute: "traceroute",
  store: "store",
  pixelart: "pixel_art",
  packet: "packet",
  codelab: "codelab",
  diskmanager: "disk_manager",
  apps: "all_apps",
  sound: "sound",
  network: "network",
  minesweeper: "minesweeper",
  pspy: "proc_spy",
  devtools: "devtools",
};

export function getAppLabelKey(type: WindowType): string {
  return APP_LABEL_KEY[type] ?? type;
}

/** Apps namespace key for description (e.g. desc_terminal, desc_files). */
export function getDescKey(type: WindowType): string {
  return "desc_" + getAppLabelKey(type);
}

export interface LaunchableApp {
  type: WindowType;
  label: string;
  labelKey: string;
  icon: React.ReactNode;
}

function TerminalIcon() {
  return (
    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <polyline points="4 17 10 11 4 5" />
      <line x1="12" y1="19" x2="20" y2="19" />
    </svg>
  );
}

function ExplorerIcon() {
  return (
    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
      <line x1="12" y1="11" x2="12" y2="17" />
      <line x1="9" y1="14" x2="15" y2="14" />
    </svg>
  );
}

function BrowserIcon() {
  return (
    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="10" />
      <line x1="2" y1="12" x2="22" y2="12" />
      <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
    </svg>
  );
}

function EditorIcon() {
  return (
    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <polyline points="16 18 22 12 16 6" />
      <polyline points="8 6 2 12 8 18" />
    </svg>
  );
}

function ThemeIcon() {
  return (
    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="10" />
      <path d="M12 2a10 10 0 0 0 0 20V2z" fill="currentColor" />
    </svg>
  );
}

/** Icon for the All Apps launcher entry (used in Dock only). */
export function AppsIcon() {
  return (
    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3" y="3" width="7" height="7" rx="1" />
      <rect x="14" y="3" width="7" height="7" rx="1" />
      <rect x="3" y="14" width="7" height="7" rx="1" />
      <rect x="14" y="14" width="7" height="7" rx="1" />
    </svg>
  );
}

function MailIcon() {
  return (
    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z" />
      <polyline points="22,6 12,13 2,6" />
    </svg>
  );
}

function WalletIcon() {
  return <Wallet size={24} />;
}

function PixelArtIcon() {
  return <Palette size={24} />;
}

function SysinfoIcon() {
  return <Cpu size={24} />;
}

function ShortcutsIcon() {
  return <Keyboard size={24} />;
}

function SysmonIcon() {
  return <Activity size={24} />;
}

function NullCloudIcon() {
  return <Cloud size={24} />;
}

function HackerboardIcon() {
  return <Trophy size={24} />;
}

function StartupIcon() {
  return <Rocket size={24} />;
}

function BackgroundIcon() {
  return <Image size={24} />;
}

function SettingsIcon() {
  return <Settings size={24} />;
}

function TraceRouteIcon() {
  return <Route size={24} />;
}

function StoreIcon() {
  return <ShoppingBag size={24} />;
}

/** Launchable apps shown in the app launcher grid (excludes All Apps itself). */
export const LAUNCHABLE_APPS: LaunchableApp[] = [
  { type: "terminal", label: "Terminal", labelKey: "terminal", icon: <TerminalIcon /> },
  { type: "explorer", label: "Files", labelKey: "files", icon: <ExplorerIcon /> },
  { type: "browser", label: "Browser", labelKey: "browser", icon: <BrowserIcon /> },
  { type: "editor", label: "Code", labelKey: "code", icon: <EditorIcon /> },
  { type: "theme", label: "Theme", labelKey: "theme", icon: <ThemeIcon /> },
  { type: "email", label: "Mail", labelKey: "mail", icon: <MailIcon /> },
  { type: "wallet", label: "Wallet", labelKey: "wallet", icon: <WalletIcon /> },
  { type: "sysinfo", label: "Nullfetch", labelKey: "nullfetch", icon: <SysinfoIcon /> },
  { type: "shortcuts", label: "Shortcuts", labelKey: "shortcuts", icon: <ShortcutsIcon /> },
  { type: "sysmon", label: "System Monitor", labelKey: "system_monitor", icon: <SysmonIcon /> },
  { type: "nullcloud", label: "NullCloud", labelKey: "nullcloud", icon: <NullCloudIcon /> },
  { type: "hackerboard", label: "Hackerboard", labelKey: "hackerboard", icon: <HackerboardIcon /> },
  { type: "startup", label: "Startup", labelKey: "startup", icon: <StartupIcon /> },
  { type: "wallpaper", label: "Background", labelKey: "background", icon: <BackgroundIcon /> },
  { type: "settings", label: "Settings", labelKey: "settings", icon: <SettingsIcon /> },
  { type: "traceroute", label: "TraceRoute", labelKey: "traceroute", icon: <TraceRouteIcon /> },
  { type: "store", label: "Store", labelKey: "store", icon: <StoreIcon /> },
  { type: "pixelart", label: "Pixel Art", labelKey: "pixel_art", icon: <PixelArtIcon /> },
  { type: "packet", label: "Packet", labelKey: "packet", icon: <Package size={24} /> },
  { type: "codelab", label: "Codelab", labelKey: "codelab", icon: <GraduationCap size={24} /> },
  { type: "diskmanager", label: "Disk Manager", labelKey: "disk_manager", icon: <HardDrive size={24} /> },
];

/** Get launchable app entry by type (for dock icon/label). */
export function getAppByType(type: WindowType): LaunchableApp | undefined {
  return LAUNCHABLE_APPS.find((a) => a.type === type);
}

/** Default window title for a given app type (optional username for Terminal). Uses t for i18n. */
export function getAppTitle(type: WindowType, username?: string | null, t?: TFunction): string {
  if (type === "terminal") return username ? `${username}@nulltrace` : (t ? t("apps:terminal") : "Terminal");
  if (!t) {
    const fallback: Record<WindowType, string> = {
      terminal: "Terminal",
      explorer: "Files",
      browser: "Browser",
      apps: "All Apps",
      editor: "Code",
      theme: "Theme",
      sound: "Sound",
      network: "Network",
      email: "Mail",
      wallet: "Wallet",
      pixelart: "Pixel Art",
      sysinfo: "Nullfetch",
      shortcuts: "Shortcuts",
      sysmon: "System Monitor",
      nullcloud: "NullCloud",
      hackerboard: "Hackerboard",
      startup: "Startup",
      wallpaper: "Background",
      settings: "Settings",
      traceroute: "TraceRoute",
      store: "Store",
      minesweeper: "Minesweeper",
      packet: "Packet",
      codelab: "Codelab",
      diskmanager: "Disk Manager",
      pspy: "Proc Spy",
      devtools: "DevTools",
    };
    return fallback[type];
  }
  const key = getAppLabelKey(type);
  return t("apps:" + key);
}
