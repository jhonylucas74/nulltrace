import React, { createContext, useContext, useState, useCallback, useEffect } from "react";
import type { WindowType } from "./WindowManagerContext";

const STORAGE_KEY = "nulltrace-installed-apps";

/** App types that can be installed from the Store (not already in the launcher). */
export const INSTALLABLE_STORE_TYPES: WindowType[] = ["sound", "network", "minesweeper"];

function loadInstalled(): WindowType[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return (parsed as string[]).filter((t): t is WindowType =>
      INSTALLABLE_STORE_TYPES.includes(t as WindowType)
    );
  } catch {
    return [];
  }
}

function saveInstalled(types: WindowType[]) {
  if (typeof window === "undefined") return;
  localStorage.setItem(STORAGE_KEY, JSON.stringify(types));
}

interface InstalledAppsContextValue {
  installedAppTypes: WindowType[];
  install: (type: WindowType) => void;
  uninstall: (type: WindowType) => void;
  isInstalled: (type: WindowType) => boolean;
}

const InstalledAppsContext = createContext<InstalledAppsContextValue | null>(null);

export function InstalledAppsProvider({ children }: { children: React.ReactNode }) {
  const [installedAppTypes, setInstalledAppTypes] = useState<WindowType[]>(loadInstalled);

  useEffect(() => {
    saveInstalled(installedAppTypes);
  }, [installedAppTypes]);

  const install = useCallback((type: WindowType) => {
    if (!INSTALLABLE_STORE_TYPES.includes(type)) return;
    setInstalledAppTypes((prev) => (prev.includes(type) ? prev : [...prev, type]));
  }, []);

  const uninstall = useCallback((type: WindowType) => {
    setInstalledAppTypes((prev) => prev.filter((t) => t !== type));
  }, []);

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
