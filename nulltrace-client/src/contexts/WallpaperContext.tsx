import React, { createContext, useContext, useState, useCallback, useEffect } from "react";

const STORAGE_KEY = "nulltrace-wallpaper";
const GRID_STORAGE_KEY = "nulltrace-wallpaper-grid";

function readStoredWallpaper(): string | null {
  if (typeof window === "undefined") return null;
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "" || stored === null) return null;
    if (typeof stored === "string" && stored.startsWith("http")) return stored;
    return null;
  } catch {
    return null;
  }
}

function readStoredGridEnabled(): boolean {
  if (typeof window === "undefined") return true;
  try {
    const stored = localStorage.getItem(GRID_STORAGE_KEY);
    if (stored === null) return true;
    return stored !== "0" && stored !== "false";
  } catch {
    return true;
  }
}

interface WallpaperContextValue {
  wallpaperUrl: string | null;
  setWallpaper: (url: string | null) => void;
  gridEnabled: boolean;
  setGridEnabled: (enabled: boolean) => void;
}

const WallpaperContext = createContext<WallpaperContextValue | null>(null);

export function WallpaperProvider({ children }: { children: React.ReactNode }) {
  const [wallpaperUrl, setWallpaperState] = useState<string | null>(readStoredWallpaper);
  const [gridEnabled, setGridEnabledState] = useState<boolean>(readStoredGridEnabled);

  useEffect(() => {
    setWallpaperState(readStoredWallpaper());
  }, []);

  useEffect(() => {
    setGridEnabledState(readStoredGridEnabled());
  }, []);

  const setWallpaper = useCallback((url: string | null) => {
    setWallpaperState(url);
    if (typeof window === "undefined") return;
    if (url === null) {
      localStorage.removeItem(STORAGE_KEY);
    } else {
      localStorage.setItem(STORAGE_KEY, url);
    }
  }, []);

  const setGridEnabled = useCallback((enabled: boolean) => {
    setGridEnabledState(enabled);
    if (typeof window !== "undefined") {
      localStorage.setItem(GRID_STORAGE_KEY, enabled ? "1" : "0");
    }
  }, []);

  const value: WallpaperContextValue = { wallpaperUrl, setWallpaper, gridEnabled, setGridEnabled };

  return (
    <WallpaperContext.Provider value={value}>
      {children}
    </WallpaperContext.Provider>
  );
}

export function useWallpaper(): WallpaperContextValue {
  const ctx = useContext(WallpaperContext);
  if (!ctx) throw new Error("useWallpaper must be used within WallpaperProvider");
  return ctx;
}
