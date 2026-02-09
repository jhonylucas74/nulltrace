import { useRef, useEffect } from "react";
import { useWindowManager } from "../contexts/WindowManagerContext";
import { useWorkspaceLayout } from "../contexts/WorkspaceLayoutContext";
import { useAppLauncher } from "../contexts/AppLauncherContext";
import { usePaymentFeedbackOptional } from "../contexts/PaymentFeedbackContext";
import type { WindowType } from "../contexts/WindowManagerContext";
import { LAUNCHABLE_APPS, AppsIcon, getAppTitle } from "../lib/appList";
import styles from "./Dock.module.css";

/** Apps shown on the dock: exclude Theme, Pixel Art, Sysinfo, Shortcuts, Sysmon, Nullcloud, Startup. */
const DOCK_LAUNCHABLE = LAUNCHABLE_APPS.filter(
  (app) =>
    app.type !== "theme" &&
    app.type !== "pixelart" &&
    app.type !== "sysinfo" &&
    app.type !== "shortcuts" &&
    app.type !== "sysmon" &&
    app.type !== "nullcloud" &&
    app.type !== "startup"
);

const WALLET_APP = LAUNCHABLE_APPS.find((a) => a.type === "wallet")!;
const HACKERBOARD_APP = LAUNCHABLE_APPS.find((a) => a.type === "hackerboard")!;

/** Dock order: dock apps (no Wallet, Hackerboard), then Wallet, Hackerboard, then All Apps. */
const DOCK_APPS_WITHOUT_WALLET_OR_HACKERBOARD = DOCK_LAUNCHABLE.filter(
  (a) => a.type !== "wallet" && a.type !== "hackerboard"
);
const DOCK_APPS = [
  ...DOCK_APPS_WITHOUT_WALLET_OR_HACKERBOARD,
  WALLET_APP,
  HACKERBOARD_APP,
  { type: "apps" as const, label: "All Apps", icon: <AppsIcon /> },
];

interface DockProps {
  username?: string | null;
}

export default function Dock({ username }: DockProps) {
  const { openApp, setActiveWorkspace, activeWorkspaceId, workspaces } = useWorkspaceLayout();
  const { windows, setFocus, getWindowIdsByType } = useWindowManager();
  const { open: openAppLauncher } = useAppLauncher();
  const paymentFeedback = usePaymentFeedbackOptional();
  const walletIconRef = useRef<HTMLButtonElement>(null);
  const firstWorkspaceId = workspaces[0]?.id ?? "";

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
        {DOCK_APPS.map((app) => {
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
