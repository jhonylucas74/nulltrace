import React, { createContext, useContext, useState, useCallback } from "react";

interface AppLauncherContextValue {
  isOpen: boolean;
  open: () => void;
  close: () => void;
}

const AppLauncherContext = createContext<AppLauncherContextValue | null>(null);

export function AppLauncherProvider({ children }: { children: React.ReactNode }) {
  const [isOpen, setIsOpen] = useState(false);

  const open = useCallback(() => setIsOpen(true), []);
  const close = useCallback(() => setIsOpen(false), []);

  return (
    <AppLauncherContext.Provider value={{ isOpen, open, close }}>
      {children}
    </AppLauncherContext.Provider>
  );
}

export function useAppLauncher(): AppLauncherContextValue {
  const ctx = useContext(AppLauncherContext);
  if (!ctx) throw new Error("useAppLauncher must be used within AppLauncherProvider");
  return ctx;
}
