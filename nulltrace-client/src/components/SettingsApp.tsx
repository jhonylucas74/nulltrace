import { useTranslation } from "react-i18next";
import { Monitor } from "lucide-react";
import i18n, { setStoredLocale } from "../language/i18n";
import { useWindowConfig } from "../contexts/WindowConfigContext";
import styles from "./SettingsApp.module.css";

const SUPPORTED_LOCALES = ["en", "pt-br"] as const;

function currentLocale(): (typeof SUPPORTED_LOCALES)[number] {
  const lng = i18n.language?.toLowerCase();
  if (lng?.startsWith("pt")) return "pt-br";
  return "en";
}

export default function SettingsApp() {
  const { t } = useTranslation("settings");
  const { fullscreen, startMaximized, setFullscreen, setStartMaximized } = useWindowConfig();
  const locale = currentLocale();

  const handleLocaleChange = (value: string) => {
    if (value !== "en" && value !== "pt-br") return;
    i18n.changeLanguage(value);
    setStoredLocale(value);
  };

  return (
    <div className={styles.app}>
      <aside className={styles.sidebar}>
        <div className={styles.sidebarTitle}>{t("title")}</div>
        <div className={styles.navItemActive}>
          <span className={styles.navIcon}>
            <Monitor size={18} />
          </span>
          {t("config_window")}
        </div>
      </aside>
      <div className={styles.main}>
        <div className={styles.content}>
          <div className={styles.sectionHeader}>
            <h2 className={styles.sectionTitle}>{t("config_window")}</h2>
          </div>
          <p className={styles.hint}>
            {t("window_hint")}
          </p>
          <div className={styles.card}>
            <label className={styles.checkLabel}>
              <input
                type="checkbox"
                className={styles.checkbox}
                checked={fullscreen}
                onChange={(e) => setFullscreen(e.target.checked)}
              />
              {t("fullscreen")}
            </label>
            <p className={styles.cardHint}>
              {t("fullscreen_hint")}
            </p>
            <label className={styles.checkLabel}>
              <input
                type="checkbox"
                className={styles.checkbox}
                checked={startMaximized}
                onChange={(e) => setStartMaximized(e.target.checked)}
                disabled={fullscreen}
              />
              {t("start_maximized")}
            </label>
            <p className={styles.cardHint}>
              {fullscreen
                ? t("maximized_unavailable")
                : t("maximized_hint")}
            </p>
          </div>

          <div className={styles.sectionHeader}>
            <h2 className={styles.sectionTitle}>{t("language")}</h2>
          </div>
          <p className={styles.hint}>
            {t("language_hint")}
          </p>
          <div className={styles.card}>
            <label className={styles.selectLabel} htmlFor="settings-locale">
              {t("language")}
            </label>
            <select
              id="settings-locale"
              className={styles.localeSelect}
              value={locale}
              onChange={(e) => handleLocaleChange(e.target.value)}
              aria-label={t("language")}
            >
              <option value="en">{t("language_en")}</option>
              <option value="pt-br">{t("language_pt_br")}</option>
            </select>
          </div>
        </div>
      </div>
    </div>
  );
}
