import React, { createContext, useContext, useState, useCallback, useEffect } from "react";
import type { WindowType } from "./WindowManagerContext";
import type { LayoutPreset } from "./WorkspaceLayoutContext";

const STORAGE_KEY = "nulltrace-startup-config";

/** App types that can be added to startup (launchable, excluding apps and startup). */
const ALLOWED_STARTUP_APP_TYPES: WindowType[] = [
  "terminal",
  "explorer",
  "browser",
  "editor",
  "theme",
  "sound",
  "network",
  "email",
  "wallet",
  "pixelart",
  "sysinfo",
  "shortcuts",
  "sysmon",
  "nullcloud",
  "hackerboard",
];

export interface StartupConfig {
  startupAppTypes: WindowType[];
  centerFirstWindow: boolean;
  gridEnabledByDefault: boolean;
  defaultLayoutPreset: LayoutPreset;
}

const DEFAULT_CONFIG: StartupConfig = {
  startupAppTypes: ["sysinfo"],
  centerFirstWindow: true,
  gridEnabledByDefault: false,
  defaultLayoutPreset: "2x2",
};

function loadConfig(): StartupConfig {
  if (typeof window === "undefined") return DEFAULT_CONFIG;
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT_CONFIG;
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    const startupAppTypes = Array.isArray(parsed.startupAppTypes)
      ? (parsed.startupAppTypes as string[]).filter((t): t is WindowType =>
          ALLOWED_STARTUP_APP_TYPES.includes(t as WindowType)
        )
      : DEFAULT_CONFIG.startupAppTypes;
    const centerFirstWindow =
      typeof parsed.centerFirstWindow === "boolean"
        ? parsed.centerFirstWindow
        : DEFAULT_CONFIG.centerFirstWindow;
    const gridEnabledByDefault =
      typeof parsed.gridEnabledByDefault === "boolean"
        ? parsed.gridEnabledByDefault
        : DEFAULT_CONFIG.gridEnabledByDefault;
    const defaultLayoutPreset = [
      "3x2",
      "2x2",
      "2x1",
      "2+1",
      "1+2",
      "1x1",
    ].includes(parsed.defaultLayoutPreset as string)
      ? (parsed.defaultLayoutPreset as LayoutPreset)
      : DEFAULT_CONFIG.defaultLayoutPreset;
    return {
      startupAppTypes,
      centerFirstWindow,
      gridEnabledByDefault,
      defaultLayoutPreset,
    };
  } catch {
    return DEFAULT_CONFIG;
  }
}

function saveConfig(config: StartupConfig) {
  if (typeof window === "undefined") return;
  localStorage.setItem(STORAGE_KEY, JSON.stringify(config));
}

/** Exported for WorkspaceLayoutContext to read initial grid state without React context. */
export function getStartupConfigFromStorage(): Pick<
  StartupConfig,
  "gridEnabledByDefault" | "defaultLayoutPreset"
> {
  const c = loadConfig();
  return {
    gridEnabledByDefault: c.gridEnabledByDefault,
    defaultLayoutPreset: c.defaultLayoutPreset,
  };
}

interface StartupConfigValue extends StartupConfig {
  setStartupAppTypes: (types: WindowType[]) => void;
  setCenterFirstWindow: (value: boolean) => void;
  setGridEnabledByDefault: (value: boolean) => void;
  setDefaultLayoutPreset: (preset: LayoutPreset) => void;
  allowedStartupAppTypes: WindowType[];
}

const StartupConfigContext = createContext<StartupConfigValue | null>(null);

export function StartupConfigProvider({ children }: { children: React.ReactNode }) {
  const [config, setConfigState] = useState<StartupConfig>(loadConfig);

  useEffect(() => {
    setConfigState(loadConfig());
  }, []);

  const setStartupAppTypes = useCallback((startupAppTypes: WindowType[]) => {
    setConfigState((prev) => {
      const filtered = startupAppTypes.filter((t) =>
        ALLOWED_STARTUP_APP_TYPES.includes(t)
      );
      const next = { ...prev, startupAppTypes: filtered };
      saveConfig(next);
      return next;
    });
  }, []);

  const setCenterFirstWindow = useCallback((centerFirstWindow: boolean) => {
    setConfigState((prev) => {
      const next = { ...prev, centerFirstWindow };
      saveConfig(next);
      return next;
    });
  }, []);

  const setGridEnabledByDefault = useCallback((gridEnabledByDefault: boolean) => {
    setConfigState((prev) => {
      const next = { ...prev, gridEnabledByDefault };
      saveConfig(next);
      return next;
    });
  }, []);

  const setDefaultLayoutPreset = useCallback((defaultLayoutPreset: LayoutPreset) => {
    setConfigState((prev) => {
      const next = { ...prev, defaultLayoutPreset };
      saveConfig(next);
      return next;
    });
  }, []);

  const value: StartupConfigValue = {
    ...config,
    setStartupAppTypes,
    setCenterFirstWindow,
    setGridEnabledByDefault,
    setDefaultLayoutPreset,
    allowedStartupAppTypes: ALLOWED_STARTUP_APP_TYPES,
  };

  return (
    <StartupConfigContext.Provider value={value}>
      {children}
    </StartupConfigContext.Provider>
  );
}

export function useStartupConfig(): StartupConfigValue {
  const ctx = useContext(StartupConfigContext);
  if (!ctx) throw new Error("useStartupConfig must be used within StartupConfigProvider");
  return ctx;
}
