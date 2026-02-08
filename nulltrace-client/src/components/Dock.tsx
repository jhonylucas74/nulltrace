import { useRef, useEffect } from "react";
import { useWindowManager } from "../contexts/WindowManagerContext";
import { useWorkspaceLayout } from "../contexts/WorkspaceLayoutContext";
import { useAppLauncher } from "../contexts/AppLauncherContext";
import { usePaymentFeedbackOptional } from "../contexts/PaymentFeedbackContext";
import type { WindowType } from "../contexts/WindowManagerContext";
import { LAUNCHABLE_APPS, AppsIcon, getAppTitle } from "../lib/appList";
import styles from "./Dock.module.css";

/** Apps shown on the dock: exclude Theme, Pixel Art, Sysinfo, Shortcuts, Sysmon, Nullcloud. */
const DOCK_LAUNCHABLE = LAUNCHABLE_APPS.filter(
  (app) =>
    app.type !== "theme" &&
    app.type !== "pixelart" &&
    app.type !== "sysinfo" &&
    app.type !== "shortcuts" &&
    app.type !== "sysmon" &&
    app.type !== "nullcloud"
);

const WALLET_APP = LAUNCHABLE_APPS.find((a) => a.type === "wallet")!;

/** Dock order: dock apps (no Wallet in list), then Wallet fixed, then All Apps. */
const DOCK_APPS_WITHOUT_WALLET = DOCK_LAUNCHABLE.filter((a) => a.type !== "wallet");
const DOCK_APPS = [
  ...DOCK_APPS_WITHOUT_WALLET,
  WALLET_APP,
  { type: "apps" as const, label: "All Apps", icon: <AppsIcon /> },
];

interface DockProps {
  username?: string | null;
}

export default function Dock({ username }: DockProps) {
  const { openApp } = useWorkspaceLayout();
  const { setFocus, getWindowIdsByType } = useWindowManager();
  const { open: openAppLauncher } = useAppLauncher();
  const paymentFeedback = usePaymentFeedbackOptional();
  const walletIconRef = useRef<HTMLButtonElement>(null);

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
      setFocus(ids[ids.length - 1]);
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
