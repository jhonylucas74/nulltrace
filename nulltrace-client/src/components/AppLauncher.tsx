import { useState, useEffect, useRef, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useWorkspaceLayout } from "../contexts/WorkspaceLayoutContext";
import { useAppLauncher } from "../contexts/AppLauncherContext";
import { useAuth } from "../contexts/AuthContext";
import { useInstalledApps } from "../contexts/InstalledAppsContext";
import { LAUNCHABLE_APPS, getAppTitle, getAppLabelKey } from "../lib/appList";
import { STORE_CATALOG } from "../lib/storeCatalog";
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
    ).map((entry) => ({ type: entry.type, label: entry.name, labelKey: getAppLabelKey(entry.type), icon: entry.icon }));
    return [...LAUNCHABLE_APPS, ...installedOnly];
  }, [installedAppTypes]);
}

export default function AppLauncher() {
  const { t } = useTranslation("common");
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

  const { t: tApps } = useTranslation("apps");
  function handleAppClick(type: LaunchableApp["type"]) {
    openApp(type, { title: getAppTitle(type, username, tApps) });
    close();
  }

  if (!isOpen) return null;

  return (
    <div
      className={styles.overlay}
      role="dialog"
      aria-modal="true"
      aria-label={t("app_launcher")}
      onClick={(e) => e.target === e.currentTarget && close()}
    >
      <div className={styles.panel} onClick={(e) => e.stopPropagation()}>
        <input
          ref={inputRef}
          type="text"
          className={styles.search}
          placeholder={t("search_apps")}
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && filtered.length > 0) {
              e.preventDefault();
              handleAppClick(filtered[0].type);
            }
          }}
          aria-label={t("search_apps")}
        />
        <div className={styles.grid}>
          {filtered.length === 0 ? (
            <p className={styles.empty}>{t("no_results")}</p>
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
                <span className={styles.appLabel}>{tApps(app.labelKey)}</span>
              </button>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
