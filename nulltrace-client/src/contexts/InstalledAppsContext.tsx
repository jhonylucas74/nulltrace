import React, { createContext, useContext, useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAuth } from "./AuthContext";
import type { WindowType } from "./WindowManagerContext";

/** App types that can be installed from the Store (store-only; not in LAUNCHABLE_APPS by default). */
export const INSTALLABLE_STORE_TYPES: WindowType[] = ["sound", "network", "minesweeper", "pixelart", "pspy"];

interface InstalledAppsContextValue {
  installedAppTypes: WindowType[];
  install: (type: WindowType) => void;
  uninstall: (type: WindowType) => void;
  isInstalled: (type: WindowType) => boolean;
}

const InstalledAppsContext = createContext<InstalledAppsContextValue | null>(null);

export function InstalledAppsProvider({ children }: { children: React.ReactNode }) {
  const { playerId, token } = useAuth();
  const [installedAppTypes, setInstalledAppTypes] = useState<WindowType[]>([]);

  // Fetch installed store apps from VM file when authenticated.
  useEffect(() => {
    if (!playerId || !token) {
      setInstalledAppTypes([]);
      return;
    }
    let cancelled = false;
    invoke<{ app_types: string[]; error_message: string }>("grpc_get_installed_store_apps", {
      token,
    })
      .then((res) => {
        if (cancelled) return;
        const types = (res.app_types || []).filter((t): t is WindowType =>
          INSTALLABLE_STORE_TYPES.includes(t as WindowType)
        );
        setInstalledAppTypes(types);
      })
      .catch(() => {
        if (!cancelled) setInstalledAppTypes([]);
      });
    return () => {
      cancelled = true;
    };
  }, [playerId, token]);

  const install = useCallback(
    (type: WindowType) => {
      if (!INSTALLABLE_STORE_TYPES.includes(type)) return;
      if (!playerId || !token) return;
      invoke<{ success: boolean; error_message: string }>("grpc_install_store_app", {
        appType: type,
        token,
      })
        .then((res) => {
          if (res.success) {
            setInstalledAppTypes((prev) => (prev.includes(type) ? prev : [...prev, type]));
          } else if (res.error_message) {
            console.error("[InstalledApps] Install failed:", res.error_message);
          }
        })
        .catch((e) => {
          console.error("[InstalledApps] Install error:", e);
        });
    },
    [playerId, token]
  );

  const uninstall = useCallback(
    (type: WindowType) => {
      if (!playerId || !token) return;
      invoke<{ success: boolean; error_message: string }>("grpc_uninstall_store_app", {
        appType: type,
        token,
      })
        .then((res) => {
          if (res.success) {
            setInstalledAppTypes((prev) => prev.filter((t) => t !== type));
          } else if (res.error_message) {
            console.error("[InstalledApps] Uninstall failed:", res.error_message);
          }
        })
        .catch((e) => {
          console.error("[InstalledApps] Uninstall error:", e);
        });
    },
    [playerId, token]
  );

  const isInstalled = useCallback(
    (type: WindowType) => installedAppTypes.includes(type),
    [installedAppTypes]
  );

  const value: InstalledAppsContextValue = {
    installedAppTypes,
    install,
    uninstall,
    isInstalled,
  };

  return (
    <InstalledAppsContext.Provider value={value}>
      {children}
    </InstalledAppsContext.Provider>
  );
}

export function useInstalledApps(): InstalledAppsContextValue {
  const ctx = useContext(InstalledAppsContext);
  if (!ctx) throw new Error("useInstalledApps must be used within InstalledAppsProvider");
  return ctx;
}
