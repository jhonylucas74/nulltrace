import { useRef, useEffect, useMemo } from "react";
import { useWindowManager } from "../contexts/WindowManagerContext";
import { useWorkspaceLayout } from "../contexts/WorkspaceLayoutContext";
import { useAppLauncher } from "../contexts/AppLauncherContext";
import { usePaymentFeedbackOptional } from "../contexts/PaymentFeedbackContext";
import type { WindowType } from "../contexts/WindowManagerContext";
import { useInstalledApps } from "../contexts/InstalledAppsContext";
import { LAUNCHABLE_APPS, AppsIcon, getAppTitle, getAppByType } from "../lib/appList";
import { STORE_CATALOG } from "../lib/storeCatalog";
import type { LaunchableApp } from "../lib/appList";
import styles from "./Dock.module.css";

/** App types that never appear in the dock (not fixed, not temporary when running). */
const DOCK_EXCLUDED_TYPES = new Set<WindowType>(["codelab", "diskmanager"]);

/** Fixed dock apps: exclude Theme, Pixel Art, Sysinfo, Shortcuts, Sysmon, Nullcloud, Startup, Code, Wallpaper, Hackerboard, Settings, TraceRoute, Store, Packet, Codelab, Disk Manager. */
const FIXED_DOCK_APPS: LaunchableApp[] = LAUNCHABLE_APPS.filter(
  (app) =>
    !DOCK_EXCLUDED_TYPES.has(app.type) &&
    app.type !== "theme" &&
    app.type !== "pixelart" &&
    app.type !== "sysinfo" &&
    app.type !== "shortcuts" &&
    app.type !== "sysmon" &&
    app.type !== "nullcloud" &&
    app.type !== "startup" &&
    app.type !== "editor" &&
    app.type !== "wallpaper" &&
    app.type !== "hackerboard" &&
    app.type !== "settings" &&
    app.type !== "traceroute" &&
    app.type !== "store" &&
    app.type !== "packet"
);

const ALL_APPS_ENTRY: LaunchableApp = { type: "apps", label: "All Apps", icon: <AppsIcon /> };

interface DockProps {
  username?: string | null;
}

function getAppEntryForDock(type: WindowType, isInstalled: (t: WindowType) => boolean): LaunchableApp | undefined {
  const builtIn = getAppByType(type);
  if (builtIn) return builtIn;
  if (!isInstalled(type)) return undefined;
  const entry = STORE_CATALOG.find((e) => e.type === type);
  return entry ? { type: entry.type, label: entry.name, icon: entry.icon } : undefined;
}

export default function Dock({ username }: DockProps) {
  const { openApp, setActiveWorkspace, activeWorkspaceId, workspaces } = useWorkspaceLayout();
  const { windows, setFocus, getWindowIdsByType } = useWindowManager();
  const { open: openAppLauncher } = useAppLauncher();
  const { isInstalled } = useInstalledApps();
  const paymentFeedback = usePaymentFeedbackOptional();
  const walletIconRef = useRef<HTMLButtonElement>(null);
  const firstWorkspaceId = workspaces[0]?.id ?? "";

  const dockApps = useMemo(() => {
    const fixedTypes = new Set(FIXED_DOCK_APPS.map((a) => a.type));
    const runningOrder: WindowType[] = [];
    for (const w of windows) {
      if (w.type === "apps") continue;
      if (!runningOrder.includes(w.type)) runningOrder.push(w.type);
    }
    const temporary = runningOrder
      .filter((type) => !fixedTypes.has(type) && !DOCK_EXCLUDED_TYPES.has(type))
      .map((type) => getAppEntryForDock(type, isInstalled))
      .filter((app): app is LaunchableApp => app != null);
    return [...FIXED_DOCK_APPS, ...temporary, ALL_APPS_ENTRY];
  }, [windows, isInstalled]);

  useEffect(() => {
    paymentFeedback?.registerWalletIconElement(walletIconRef.current ?? null);
    return () => paymentFeedback?.registerWalletIconElement(null);
  }, [paymentFeedback]);

  function handleAppClick(type: WindowType) {
    if (type === "apps") {
      openAppLauncher();
      return;
    }
    const ids = getWindowIdsByType(type);
    if (ids.length > 0) {
      const winId = ids[ids.length - 1];
      const win = windows.find((w) => w.id === winId);
      if (win) {
        const targetWorkspaceId = win.workspaceId === "" ? firstWorkspaceId : win.workspaceId;
        if (targetWorkspaceId && targetWorkspaceId !== activeWorkspaceId) {
          setActiveWorkspace(targetWorkspaceId);
        }
      }
      setFocus(winId);
    } else {
      openApp(type, { title: getAppTitle(type, username) });
    }
  }

  return (
    <footer className={styles.dock}>
      <div className={styles.dockInner}>
        {dockApps.map((app) => {
          const windowIds = getWindowIdsByType(app.type);
          const hasOpen = windowIds.length > 0;
          const isWallet = app.type === "wallet";
          const impactClass = isWallet && paymentFeedback?.walletIconImpact ? styles.dockItemWalletImpact : "";
          return (
            <button
              key={app.type}
              ref={isWallet ? walletIconRef : undefined}
              type="button"
              className={`${styles.dockItem} ${impactClass}`.trim()}
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
