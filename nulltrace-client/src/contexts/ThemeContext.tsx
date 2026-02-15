import React, { createContext, useContext, useState, useCallback, useEffect } from "react";
import { useAuth } from "./AuthContext";
import { useGrpc } from "./GrpcContext";

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
export const VALID_THEMES: ThemeId[] = [
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
  const { token } = useAuth();
  const { getPlayerProfile, setPreferredTheme } = useGrpc();

  useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  // When we have a token, fetch profile and apply server preferred_theme.
  // Only overwrite local theme when the server has a real saved preference (not default/empty).
  // This way if the user had chosen a theme but the save failed, we keep their localStorage choice.
  const DEFAULT_THEME: ThemeId = "githubdark";
  useEffect(() => {
    if (!token) return;
    getPlayerProfile(token)
      .then((profile) => {
        const serverTheme = profile.preferred_theme?.trim();
        if (
          serverTheme &&
          VALID_THEMES.includes(serverTheme as ThemeId) &&
          serverTheme !== DEFAULT_THEME
        ) {
          setThemeState(serverTheme as ThemeId);
          localStorage.setItem(STORAGE_KEY, serverTheme);
          applyTheme(serverTheme as ThemeId);
        }
        // If server has default or empty, keep current theme from localStorage (don't overwrite)
      })
      .catch(() => {
        // Keep current theme (localStorage or default)
      });
  }, [token, getPlayerProfile]);

  const setTheme = useCallback(
    (next: ThemeId) => {
      setThemeState(next);
      localStorage.setItem(STORAGE_KEY, next);
      applyTheme(next);
      if (token) {
        // Persist to server so theme is restored after re-login or on another device.
        setPreferredTheme(token, next).catch(() => {
          // Theme is still applied locally and in localStorage
        });
      }
    },
    [token, setPreferredTheme]
  );

  const value: ThemeContextValue = { theme, setTheme };

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme must be used within ThemeProvider");
  return ctx;
}
