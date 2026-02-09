import React, { createContext, useContext, useState, useCallback, useEffect } from "react";

const STORAGE_KEY = "nulltrace-window-config";

export interface WindowConfig {
  fullscreen: boolean;
  startMaximized: boolean;
}

const DEFAULT_CONFIG: WindowConfig = {
  fullscreen: false,
  startMaximized: false,
};

function loadConfig(): WindowConfig {
  if (typeof window === "undefined") return DEFAULT_CONFIG;
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT_CONFIG;
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    const fullscreen =
      typeof parsed.fullscreen === "boolean" ? parsed.fullscreen : DEFAULT_CONFIG.fullscreen;
    const startMaximized =
      typeof parsed.startMaximized === "boolean"
        ? parsed.startMaximized
        : DEFAULT_CONFIG.startMaximized;
    return { fullscreen, startMaximized };
  } catch {
    return DEFAULT_CONFIG;
  }
}

function saveConfig(config: WindowConfig) {
  if (typeof window === "undefined") return;
  localStorage.setItem(STORAGE_KEY, JSON.stringify(config));
}

/** Exported for apply-on-load in Desktop (read without React context). */
export function getWindowConfigFromStorage(): WindowConfig {
  return loadConfig();
}

interface WindowConfigValue extends WindowConfig {
  setFullscreen: (value: boolean) => void;
  setStartMaximized: (value: boolean) => void;
}

const WindowConfigContext = createContext<WindowConfigValue | null>(null);

export function WindowConfigProvider({ children }: { children: React.ReactNode }) {
  const [config, setConfigState] = useState<WindowConfig>(loadConfig);

  useEffect(() => {
    setConfigState(loadConfig());
  }, []);

  const setFullscreen = useCallback((fullscreen: boolean) => {
    setConfigState((prev) => {
      const next = { ...prev, fullscreen };
      saveConfig(next);
      return next;
    });
    // Apply immediately when running in Tauri
    const tauri = typeof window !== "undefined" && (window as unknown as { __TAURI__?: unknown }).__TAURI__;
    if (tauri) {
      import("@tauri-apps/api/window")
        .then(({ getCurrentWindow }) => getCurrentWindow().setFullscreen(fullscreen))
        .catch(() => {});
    }
  }, []);

  const setStartMaximized = useCallback((startMaximized: boolean) => {
    setConfigState((prev) => {
      const next = { ...prev, startMaximized };
      saveConfig(next);
      return next;
    });
  }, []);

  const value: WindowConfigValue = {
    ...config,
    setFullscreen,
    setStartMaximized,
  };

  return (
    <WindowConfigContext.Provider value={value}>
      {children}
    </WindowConfigContext.Provider>
  );
}

export function useWindowConfig(): WindowConfigValue {
  const ctx = useContext(WindowConfigContext);
  if (!ctx) throw new Error("useWindowConfig must be used within WindowConfigProvider");
  return ctx;
}
