import React from "react";
import { Palette, Cpu } from "lucide-react";
import type { WindowType } from "../contexts/WindowManagerContext";

export interface LaunchableApp {
  type: WindowType;
  label: string;
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
  return (
    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="1" y="4" width="22" height="16" rx="2" ry="2" />
      <line x1="1" y1="10" x2="23" y2="10" />
      <path d="M17 14a2 2 0 1 0 0-4 2 2 0 0 0 0 4z" />
    </svg>
  );
}

function PixelArtIcon() {
  return <Palette size={24} />;
}

function SysinfoIcon() {
  return <Cpu size={24} />;
}

/** Launchable apps shown in the app launcher grid (excludes All Apps itself). */
export const LAUNCHABLE_APPS: LaunchableApp[] = [
  { type: "terminal", label: "Terminal", icon: <TerminalIcon /> },
  { type: "explorer", label: "Files", icon: <ExplorerIcon /> },
  { type: "browser", label: "Browser", icon: <BrowserIcon /> },
  { type: "editor", label: "Code", icon: <EditorIcon /> },
  { type: "theme", label: "Theme", icon: <ThemeIcon /> },
  { type: "email", label: "Mail", icon: <MailIcon /> },
  { type: "wallet", label: "Wallet", icon: <WalletIcon /> },
  { type: "pixelart", label: "Pixel Art", icon: <PixelArtIcon /> },
  { type: "sysinfo", label: "Nullfetch", icon: <SysinfoIcon /> },
];

/** Default window title for a given app type (optional username for Terminal). */
export function getAppTitle(type: WindowType, username?: string | null): string {
  if (type === "terminal") return username ? `${username}@nulltrace` : "Terminal";
  const titles: Record<WindowType, string> = {
    terminal: username ? `${username}@nulltrace` : "Terminal",
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
  };
  return titles[type];
}
