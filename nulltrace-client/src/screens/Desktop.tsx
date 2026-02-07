import React from "react";
import { useAuth } from "../contexts/AuthContext";
import { WindowManagerProvider, useWindowManager } from "../contexts/WindowManagerContext";
import type { WindowType } from "../contexts/WindowManagerContext";
import TopBar from "../components/TopBar";
import Dock from "../components/Dock";
import Window from "../components/Window";
import Terminal from "../components/Terminal";
import ThemeApp from "../components/ThemeApp";
import Explorer from "../components/Explorer";
import styles from "./Desktop.module.css";

function TerminalIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <polyline points="4 17 10 11 4 5" />
      <line x1="12" y1="19" x2="20" y2="19" />
    </svg>
  );
}

function ExplorerIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
      <line x1="12" y1="11" x2="12" y2="17" />
      <line x1="9" y1="14" x2="15" y2="14" />
    </svg>
  );
}

function BrowserIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <circle cx="12" cy="12" r="10" />
      <line x1="2" y1="12" x2="22" y2="12" />
      <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
    </svg>
  );
}

function AppsIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <rect x="3" y="3" width="7" height="7" rx="1" />
      <rect x="14" y="3" width="7" height="7" rx="1" />
      <rect x="3" y="14" width="7" height="7" rx="1" />
      <rect x="14" y="14" width="7" height="7" rx="1" />
    </svg>
  );
}

function EditorIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <polyline points="16 18 22 12 16 6" />
      <polyline points="8 6 2 12 8 18" />
    </svg>
  );
}

function ThemeIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <circle cx="12" cy="12" r="3" />
      <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
    </svg>
  );
}

const WINDOW_ICONS: Record<WindowType, React.ReactNode> = {
  terminal: <TerminalIcon />,
  explorer: <ExplorerIcon />,
  browser: <BrowserIcon />,
  apps: <AppsIcon />,
  editor: <EditorIcon />,
  theme: <ThemeIcon />,
};

function PlaceholderContent({ title }: { title: string }) {
  return (
    <div className={styles.placeholder}>
      <p>{title}</p>
      <p className={styles.placeholderSub}>Coming soon</p>
    </div>
  );
}

function DesktopContent() {
  const { username } = useAuth();
  const { windows, focusedId, close, minimize, maximize, setFocus, move } = useWindowManager();

  function renderWindowContent(win: { type: WindowType; title: string }) {
    if (win.type === "terminal") {
      return <Terminal username={username ?? "user"} />;
    }
    if (win.type === "theme") {
      return <ThemeApp />;
    }
    if (win.type === "explorer") {
      return <Explorer />;
    }
    return <PlaceholderContent title={win.title} />;
  }

  return (
    <div className={styles.desktop}>
      <div className={styles.wallpaper} />
      <TopBar />
      <div className={styles.workspace}>
        {windows
          .filter((w) => !w.minimized)
          .map((win) => (
            <Window
              key={win.id}
              id={win.id}
              title={win.title}
              position={win.position}
              size={win.size}
              onMove={move}
              onClose={close}
              onMinimize={minimize}
              onMaximize={maximize}
              focused={focusedId === win.id}
              onFocus={() => setFocus(win.id)}
              minimized={win.minimized}
              maximized={win.maximized}
              zIndex={win.zIndex}
              icon={WINDOW_ICONS[win.type]}
            >
              {renderWindowContent(win)}
            </Window>
          ))}
      </div>
      <Dock username={username} />
    </div>
  );
}

export default function Desktop() {
  return (
    <WindowManagerProvider>
      <DesktopContent />
    </WindowManagerProvider>
  );
}
