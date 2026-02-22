import { useTranslation } from "react-i18next";
import { useTheme } from "../contexts/ThemeContext";
import type { ThemeId } from "../contexts/ThemeContext";
import styles from "./ThemeApp.module.css";

const THEME_IDS: ThemeId[] = [
  "latte", "frappe", "macchiato", "mocha", "onedark", "dracula",
  "githubdark", "monokai", "solardark",
];

/** Preview strip colors per theme */
const THEME_STRIP_COLORS: Record<ThemeId, string[]> = {
  latte: ["#dc8a78", "#ea76cb", "#40a02b", "#209fb5", "#1e66f5", "#8839ef"],
  frappe: ["#f2d5cf", "#f4b8e4", "#a6d189", "#81c8be", "#8caaee", "#ca9ee6"],
  macchiato: ["#f4dbd6", "#f5bde6", "#a6da95", "#8bd5ca", "#8aadf4", "#c6a0f6"],
  mocha: ["#f5e0dc", "#f5c2e7", "#a6e3a1", "#94e2d5", "#89b4fa", "#cba6f7"],
  onedark: ["#e06c75", "#d19a66", "#98c379", "#56b6c2", "#61afef", "#c678dd"],
  dracula: ["#ff5555", "#f1fa8c", "#50fa7b", "#8be9fd", "#bd93f9", "#ff79c6"],
  githubdark: ["#f85149", "#d29922", "#3fb950", "#58a6ff", "#a371f7", "#8b949e"],
  monokai: ["#f92672", "#e6db74", "#a6e22e", "#66d9ef", "#ae81ff", "#fd5ff0"],
  solardark: ["#dc322f", "#b58900", "#859900", "#268bd2", "#6c71c4", "#2aa198"],
};

export default function ThemeApp() {
  const { t } = useTranslation("theme");
  const { theme, setTheme } = useTheme();

  return (
    <div className={styles.app}>
      <p className={styles.intro}>
        {t("intro")}
      </p>
      <div className={styles.list}>
        {THEME_IDS.map((id) => {
          const isSelected = theme === id;
          return (
            <button
              key={id}
              type="button"
              className={`${styles.row} ${isSelected ? styles.rowSelected : ""}`}
              onClick={() => setTheme(id)}
            >
              <span className={styles.strip}>
                {THEME_STRIP_COLORS[id].map((c, i) => (
                  <span key={i} className={styles.stripSegment} style={{ backgroundColor: c }} />
                ))}
              </span>
              <span className={styles.label}>
                <span className={styles.name}>{t(`theme_${id}_name`)}</span>
                <span className={styles.subtitle}>{t(`theme_${id}_subtitle`)}</span>
              </span>
              {isSelected && (
                <span className={styles.check} aria-hidden>
                  ✓
                </span>
              )}
            </button>
          );
        })}
      </div>
    </div>
  );
}
