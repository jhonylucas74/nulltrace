import { Monitor } from "lucide-react";
import { useWindowConfig } from "../contexts/WindowConfigContext";
import styles from "./SettingsApp.module.css";

export default function SettingsApp() {
  const { fullscreen, startMaximized, setFullscreen, setStartMaximized } = useWindowConfig();

  return (
    <div className={styles.app}>
      <aside className={styles.sidebar}>
        <div className={styles.sidebarTitle}>Settings</div>
        <div className={styles.navItemActive}>
          <span className={styles.navIcon}>
            <Monitor size={18} />
          </span>
          Config Window
        </div>
      </aside>
      <div className={styles.main}>
        <div className={styles.content}>
          <div className={styles.sectionHeader}>
            <h2 className={styles.sectionTitle}>Config Window</h2>
          </div>
          <p className={styles.hint}>
            Options for the game window. Fullscreen applies immediately; start maximized applies on next launch.
          </p>
          <div className={styles.card}>
            <label className={styles.checkLabel}>
              <input
                type="checkbox"
                className={styles.checkbox}
                checked={fullscreen}
                onChange={(e) => setFullscreen(e.target.checked)}
              />
              Fullscreen
            </label>
            <p className={styles.cardHint}>
              Run the game in fullscreen. Applies immediately when toggled.
            </p>
            <label className={styles.checkLabel}>
              <input
                type="checkbox"
                className={styles.checkbox}
                checked={startMaximized}
                onChange={(e) => setStartMaximized(e.target.checked)}
                disabled={fullscreen}
              />
              Start maximized
            </label>
            <p className={styles.cardHint}>
              {fullscreen
                ? "Unavailable when fullscreen is enabled."
                : "Maximize the window when the app launches."}
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
