import React, { createContext, useContext, useState, useCallback, useEffect } from "react";

export type ThemeId =
  | "latte"
  | "frappe"
  | "macchiato"
  | "mocha"
  | "onedark"
  | "dracula"
  | "githubdark"
  | "monokai"
  | "solardark";

const STORAGE_KEY = "nulltrace-theme";
const VALID_THEMES: ThemeId[] = [
  "latte", "frappe", "macchiato", "mocha",
  "onedark", "dracula", "githubdark", "monokai", "solardark",
];

function readStoredTheme(): ThemeId {
  if (typeof window === "undefined") return "githubdark";
  const stored = localStorage.getItem(STORAGE_KEY);
  if (VALID_THEMES.includes(stored as ThemeId)) return stored as ThemeId;
  return "githubdark";
}

function applyTheme(theme: ThemeId) {
  document.documentElement.setAttribute("data-theme", theme);
}

// Apply stored theme immediately so first paint uses it (before React mounts)
if (typeof document !== "undefined") {
  applyTheme(readStoredTheme()); // default: githubdark (Nulltrace)
}

interface ThemeContextValue {
  theme: ThemeId;
  setTheme: (theme: ThemeId) => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const [theme, setThemeState] = useState<ThemeId>(readStoredTheme);

  useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  const setTheme = useCallback((next: ThemeId) => {
    setThemeState(next);
    localStorage.setItem(STORAGE_KEY, next);
    applyTheme(next);
  }, []);

  const value: ThemeContextValue = { theme, setTheme };

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme must be used within ThemeProvider");
  return ctx;
}
