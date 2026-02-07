import { useWindowManager } from "../contexts/WindowManagerContext";
import { useAppLauncher } from "../contexts/AppLauncherContext";
import type { WindowType } from "../contexts/WindowManagerContext";
import { LAUNCHABLE_APPS, AppsIcon, getAppTitle } from "../lib/appList";
import styles from "./Dock.module.css";

/** Apps shown on the dock: exclude Theme, Wallet, and Pixel Art (launcher only). */
const DOCK_LAUNCHABLE = LAUNCHABLE_APPS.filter(
  (app) => app.type !== "theme" && app.type !== "wallet" && app.type !== "pixelart"
);

/** Dock order: dock apps + All Apps in the middle. */
const DOCK_APPS = [
  ...DOCK_LAUNCHABLE.slice(0, 3),
  { type: "apps" as const, label: "All Apps", icon: <AppsIcon /> },
  ...DOCK_LAUNCHABLE.slice(3),
];

interface DockProps {
  username?: string | null;
}

export default function Dock({ username }: DockProps) {
  const { open, setFocus, getWindowIdsByType } = useWindowManager();
  const { open: openAppLauncher } = useAppLauncher();

  function handleAppClick(type: WindowType) {
    if (type === "apps") {
      openAppLauncher();
      return;
    }
    const ids = getWindowIdsByType(type);
    if (ids.length > 0) {
      setFocus(ids[ids.length - 1]);
    } else {
      open(type, { title: getAppTitle(type, username) });
    }
  }

  return (
    <footer className={styles.dock}>
      <div className={styles.dockInner}>
        {DOCK_APPS.map((app) => {
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
