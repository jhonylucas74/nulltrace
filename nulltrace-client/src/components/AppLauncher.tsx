import { useState, useEffect, useRef, useMemo } from "react";
import { useWorkspaceLayout } from "../contexts/WorkspaceLayoutContext";
import { useAppLauncher } from "../contexts/AppLauncherContext";
import { useAuth } from "../contexts/AuthContext";
import { useInstalledApps } from "../contexts/InstalledAppsContext";
import { LAUNCHABLE_APPS, getAppTitle } from "../lib/appList";
import { STORE_CATALOG, isBuiltInLauncherApp } from "../lib/storeCatalog";
import type { LaunchableApp } from "../lib/appList";
import styles from "./AppLauncher.module.css";

/** Built-in apps plus installed store-only apps (label + icon from catalog). */
function useLaunchableAppsList(): LaunchableApp[] {
  const { installedAppTypes } = useInstalledApps();
  return useMemo(() => {
    const builtInTypes = new Set(LAUNCHABLE_APPS.map((a) => a.type));
    const installedSet = new Set(installedAppTypes);
    const installedOnly = STORE_CATALOG.filter(
      (entry) => !builtInTypes.has(entry.type) && installedSet.has(entry.type)
    ).map((entry) => ({ type: entry.type, label: entry.name, icon: entry.icon }));
    return [...LAUNCHABLE_APPS, ...installedOnly];
  }, [installedAppTypes]);
}

export default function AppLauncher() {
  const { isOpen, close } = useAppLauncher();
  const { openApp } = useWorkspaceLayout();
  const { username } = useAuth();
  const launchableApps = useLaunchableAppsList();
  const [searchTerm, setSearchTerm] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const filtered = searchTerm.trim()
    ? launchableApps.filter((app) =>
        app.label.toLowerCase().includes(searchTerm.trim().toLowerCase())
      )
    : launchableApps;

  useEffect(() => {
    if (isOpen) {
      setSearchTerm("");
      inputRef.current?.focus();
    }
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) return;
    function handleEscape(e: KeyboardEvent) {
      if (e.key === "Escape") close();
    }
    document.addEventListener("keydown", handleEscape);
    return () => document.removeEventListener("keydown", handleEscape);
  }, [isOpen, close]);

  function handleAppClick(type: LaunchableApp["type"]) {
    openApp(type, { title: getAppTitle(type, username) });
    close();
  }

  if (!isOpen) return null;

  return (
    <div
      className={styles.overlay}
      role="dialog"
      aria-modal="true"
      aria-label="App launcher"
      onClick={(e) => e.target === e.currentTarget && close()}
    >
      <div className={styles.panel} onClick={(e) => e.stopPropagation()}>
        <input
          ref={inputRef}
          type="text"
          className={styles.search}
          placeholder="Search apps…"
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          aria-label="Search apps"
        />
        <div className={styles.grid}>
          {filtered.length === 0 ? (
            <p className={styles.empty}>No results</p>
          ) : (
            filtered.map((app, index) => (
              <button
                key={app.type}
                type="button"
                className={styles.appItem}
                style={{ animationDelay: `${index * 45}ms` }}
                onClick={() => handleAppClick(app.type)}
              >
                <span className={styles.appIcon}>{app.icon}</span>
                <span className={styles.appLabel}>{app.label}</span>
              </button>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
