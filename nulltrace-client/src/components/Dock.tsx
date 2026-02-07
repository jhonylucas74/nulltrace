import { useWindowManager } from "../contexts/WindowManagerContext";
import type { WindowType } from "../contexts/WindowManagerContext";
import styles from "./Dock.module.css";

interface DockApp {
  type: WindowType;
  label: string;
  icon: React.ReactNode;
}

const APPS: DockApp[] = [
  {
    type: "terminal",
    label: "Terminal",
    icon: <TerminalIcon />,
  },
  {
    type: "explorer",
    label: "Files",
    icon: <ExplorerIcon />,
  },
  {
    type: "browser",
    label: "Browser",
    icon: <BrowserIcon />,
  },
  {
    type: "apps",
    label: "All Apps",
    icon: <AppsIcon />,
  },
  {
    type: "editor",
    label: "Code",
    icon: <EditorIcon />,
  },
  {
    type: "theme",
    label: "Theme",
    icon: <ThemeIcon />,
  },
];

interface DockProps {
  username?: string | null;
}

export default function Dock({ username }: DockProps) {
  const { open, setFocus, getWindowIdsByType } = useWindowManager();

  function getTitle(type: WindowType): string {
    if (type === "terminal") return username ? `${username}@nulltrace` : "Terminal";
    const titles: Record<WindowType, string> = {
      terminal: username ? `${username}@nulltrace` : "Terminal",
      explorer: "Files",
      browser: "Browser",
      apps: "All Apps",
      editor: "Code",
      theme: "Theme",
    };
    return titles[type];
  }

  function handleAppClick(type: WindowType) {
    const ids = getWindowIdsByType(type);
    if (ids.length > 0) {
      setFocus(ids[ids.length - 1]);
    } else {
      open(type, { title: getTitle(type) });
    }
  }

  return (
    <footer className={styles.dock}>
      <div className={styles.dockInner}>
        {APPS.map((app) => {
          const windowIds = getWindowIdsByType(app.type);
          const hasOpen = windowIds.length > 0;
          return (
            <button
              key={app.type}
              type="button"
              className={styles.dockItem}
              onClick={() => handleAppClick(app.type)}
              title={app.label}
            >
              <span className={styles.dockIcon}>{app.icon}</span>
              {hasOpen && <span className={styles.indicator} />}
            </button>
          );
        })}
      </div>
    </footer>
  );
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

function AppsIcon() {
  return (
    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3" y="3" width="7" height="7" rx="1" />
      <rect x="14" y="3" width="7" height="7" rx="1" />
      <rect x="3" y="14" width="7" height="7" rx="1" />
      <rect x="14" y="14" width="7" height="7" rx="1" />
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
      <circle cx="12" cy="12" r="3" />
      <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
    </svg>
  );
}
