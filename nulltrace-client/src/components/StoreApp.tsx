import { useState, useMemo } from "react";
import { Package, Download, Search } from "lucide-react";
import { useAuth } from "../contexts/AuthContext";
import { useInstalledApps } from "../contexts/InstalledAppsContext";
import { useWorkspaceLayout } from "../contexts/WorkspaceLayoutContext";
import { getAppTitle } from "../lib/appList";
import {
  STORE_CATALOG,
  isBuiltInLauncherApp,
  isHiddenFromDiscover,
} from "../lib/storeCatalog";
import styles from "./StoreApp.module.css";

type Section = "discover" | "installed";

export default function StoreApp() {
  const { username } = useAuth();
  const { install, isInstalled, installedAppTypes } = useInstalledApps();
  const { openApp } = useWorkspaceLayout();
  const [section, setSection] = useState<Section>("discover");
  const [searchTerm, setSearchTerm] = useState("");

  const canOpen = (type: (typeof STORE_CATALOG)[number]["type"]) =>
    isBuiltInLauncherApp(type) || isInstalled(type);

  const discoverEntries = useMemo(
    () => STORE_CATALOG.filter((e) => !isHiddenFromDiscover(e.type)),
    []
  );

  const installedEntries = useMemo(
    () =>
      STORE_CATALOG.filter(
        (e) => isBuiltInLauncherApp(e.type) || installedAppTypes.includes(e.type)
      ),
    [installedAppTypes]
  );

  const filterBySearch = (entry: (typeof STORE_CATALOG)[number]) => {
    const q = searchTerm.trim().toLowerCase();
    if (!q) return true;
    return (
      entry.name.toLowerCase().includes(q) ||
      entry.description.toLowerCase().includes(q)
    );
  };

  const discoverFiltered = useMemo(() => {
    const filtered = discoverEntries.filter(filterBySearch);
    return [...filtered].sort((a, b) => {
      const aInstalled = canOpen(a.type);
      const bInstalled = canOpen(b.type);
      if (!aInstalled && bInstalled) return -1;
      if (aInstalled && !bInstalled) return 1;
      return 0;
    });
  }, [discoverEntries, searchTerm, installedAppTypes]);

  const installedFiltered = useMemo(
    () => installedEntries.filter(filterBySearch),
    [installedEntries, searchTerm]
  );

  const entriesToShow = section === "discover" ? discoverFiltered : installedFiltered;

  const handleOpen = (type: (typeof STORE_CATALOG)[number]["type"]) => {
    openApp(type, { title: getAppTitle(type, username) });
  };

  const handleInstall = (type: (typeof STORE_CATALOG)[number]["type"]) => {
    install(type);
    openApp(type, { title: getAppTitle(type, username) });
  };

  return (
    <div className={styles.appWithSidebar}>
      <aside className={styles.sidebar}>
        <div className={styles.sidebarSection}>Store</div>
        <button
          type="button"
          className={`${styles.navItem} ${section === "discover" ? styles.navItemActive : ""}`}
          onClick={() => setSection("discover")}
        >
          <span className={styles.navIcon}>
            <Package size={18} />
          </span>
          Discover
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "installed" ? styles.navItemActive : ""}`}
          onClick={() => setSection("installed")}
        >
          <span className={styles.navIcon}>
            <Download size={18} />
          </span>
          Installed
        </button>
      </aside>
      <main className={styles.main}>
        <h2 className={styles.mainTitle}>
          {section === "discover" ? "Discover" : "Installed"}
        </h2>
        <p className={styles.mainSubtitle}>
          {section === "discover"
            ? "Official apps for Nulltrace. Install to add them to your app launcher."
            : "Apps available in your launcher."}
        </p>
        <div className={styles.searchWrap}>
          <Search size={18} className={styles.searchIcon} aria-hidden />
          <input
            type="text"
            className={styles.searchInput}
            placeholder="Search…"
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            aria-label="Search apps"
          />
        </div>
        <div className={styles.grid}>
          {entriesToShow.length === 0 ? (
            <p className={styles.empty}>
              {searchTerm.trim()
                ? "No apps match your search."
                : section === "installed"
                  ? "No installed apps to show."
                  : "No apps to show."}
            </p>
          ) : (
            entriesToShow.map((entry) => {
              const showOpen = canOpen(entry.type);
              return (
                <div key={entry.type} className={styles.card}>
                  <div className={styles.cardIconWrap}>{entry.icon}</div>
                  <div className={styles.cardTitleRow}>
                    <span className={styles.cardTitle}>{entry.name}</span>
                  </div>
                  <p className={styles.cardDesc}>{entry.description}</p>
                  {showOpen ? (
                    <button
                      type="button"
                      className={styles.btnPrimary}
                      onClick={() => handleOpen(entry.type)}
                    >
                      Open
                    </button>
                  ) : (
                    <button
                      type="button"
                      className={styles.btnPrimary}
                      onClick={() => handleInstall(entry.type)}
                    >
                      Install
                    </button>
                  )}
                </div>
              );
            })
          )}
        </div>
      </main>
    </div>
  );
}
