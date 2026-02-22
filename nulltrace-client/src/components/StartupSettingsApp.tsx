import { useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { ChevronUp, ChevronDown, Trash2, Rocket, LayoutTemplate } from "lucide-react";
import { useStartupConfig } from "../contexts/StartupConfigContext";
import { getAppTitle } from "../lib/appList";
import type { WindowType } from "../contexts/WindowManagerContext";
import type { LayoutPreset } from "../contexts/WorkspaceLayoutContext";
import styles from "./StartupSettingsApp.module.css";

const LAYOUT_PRESETS: LayoutPreset[] = ["3x2", "2x2", "2x1", "2+1", "1+2", "1x1"];

type Section = "programs" | "grid";

export default function StartupSettingsApp() {
  const { t } = useTranslation("startup");
  const { t: tApps } = useTranslation("apps");
  const [section, setSection] = useState<Section>("programs");
  const {
    startupAppTypes,
    setStartupAppTypes,
    centerFirstWindow,
    setCenterFirstWindow,
    gridEnabledByDefault,
    setGridEnabledByDefault,
    defaultLayoutPreset,
    setDefaultLayoutPreset,
    allowedStartupAppTypes,
  } = useStartupConfig();

  const addApp = useCallback(
    (type: WindowType) => {
      if (startupAppTypes.includes(type)) return;
      setStartupAppTypes([...startupAppTypes, type]);
    },
    [startupAppTypes, setStartupAppTypes]
  );

  const removeApp = useCallback(
    (index: number) => {
      setStartupAppTypes(startupAppTypes.filter((_, i) => i !== index));
    },
    [startupAppTypes, setStartupAppTypes]
  );

  const moveUp = useCallback(
    (index: number) => {
      if (index <= 0) return;
      const next = [...startupAppTypes];
      [next[index - 1], next[index]] = [next[index], next[index - 1]];
      setStartupAppTypes(next);
    },
    [startupAppTypes, setStartupAppTypes]
  );

  const moveDown = useCallback(
    (index: number) => {
      if (index >= startupAppTypes.length - 1) return;
      const next = [...startupAppTypes];
      [next[index], next[index + 1]] = [next[index + 1], next[index]];
      setStartupAppTypes(next);
    },
    [startupAppTypes, setStartupAppTypes]
  );

  const availableToAdd = allowedStartupAppTypes.filter((t) => !startupAppTypes.includes(t));

  const handleGridDefaultChange = useCallback(
    (enabled: boolean) => {
      setGridEnabledByDefault(enabled);
    },
    [setGridEnabledByDefault]
  );

  return (
    <div className={styles.app}>
      <aside className={styles.sidebar}>
        <div className={styles.sidebarTitle}>{t("sidebar_title")}</div>
        <button
          type="button"
          className={`${styles.navItem} ${section === "programs" ? styles.navItemActive : ""}`}
          onClick={() => setSection("programs")}
        >
          <span className={styles.navIcon}>
            <Rocket size={18} />
          </span>
          {t("nav_programs")}
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "grid" ? styles.navItemActive : ""}`}
          onClick={() => setSection("grid")}
        >
          <span className={styles.navIcon}>
            <LayoutTemplate size={18} />
          </span>
          {t("nav_grid")}
        </button>
      </aside>
      <div className={styles.main}>
        <div className={styles.content}>
          {section === "programs" && (
            <>
              <div className={styles.sectionHeader}>
                <h2 className={styles.sectionTitle}>{t("section_programs")}</h2>
              </div>
              <p className={styles.hint}>
                {t("hint_programs")}
              </p>
              <div className={styles.card}>
                <div className={styles.list}>
                  {startupAppTypes.length === 0 ? (
                    <p className={styles.emptyState}>{t("empty_programs")}</p>
                  ) : (
                    startupAppTypes.map((type, index) => (
                      <div key={`${type}-${index}`} className={styles.row}>
                        <span className={styles.rowLabel}>{getAppTitle(type, undefined, tApps)}</span>
                        <div className={styles.rowActions}>
                          <button
                            type="button"
                            className={styles.iconBtn}
                            onClick={() => moveUp(index)}
                            disabled={index === 0}
                            aria-label={t("move_up")}
                          >
                            <ChevronUp size={18} />
                          </button>
                          <button
                            type="button"
                            className={styles.iconBtn}
                            onClick={() => moveDown(index)}
                            disabled={index === startupAppTypes.length - 1}
                            aria-label={t("move_down")}
                          >
                            <ChevronDown size={18} />
                          </button>
                          <button
                            type="button"
                            className={styles.iconBtn}
                            onClick={() => removeApp(index)}
                            aria-label={t("remove_program", { name: getAppTitle(type, undefined, tApps) })}
                          >
                            <Trash2 size={18} />
                          </button>
                        </div>
                      </div>
                    ))
                  )}
                </div>
                <div className={styles.addRow}>
                  <select
                    className={styles.addProgramSelect}
                    value=""
                    onChange={(e) => {
                      const v = e.target.value as WindowType;
                      if (v) addApp(v);
                      e.target.value = "";
                    }}
                    aria-label={t("add_program_label")}
                  >
                    <option value="">{t("add_program_placeholder")}</option>
                    {availableToAdd.map((type) => (
                      <option key={type} value={type}>
                        {getAppTitle(type, undefined, tApps)}
                      </option>
                    ))}
                  </select>
                  <button
                    type="button"
                    className={styles.addBtn}
                    disabled={availableToAdd.length === 0}
                    onClick={() => {
                      const first = availableToAdd[0];
                      if (first) addApp(first);
                    }}
                  >
                    {t("add_btn")}
                  </button>
                </div>
              </div>
              <div className={styles.card} style={{ marginTop: "1rem" }}>
                <label className={styles.checkLabel}>
                  <input
                    type="checkbox"
                    className={styles.checkbox}
                    checked={centerFirstWindow}
                    onChange={(e) => setCenterFirstWindow(e.target.checked)}
                    disabled={gridEnabledByDefault}
                  />
                  {t("center_first_window")}
                </label>
                <p className={styles.cardHint}>
                  {gridEnabledByDefault ? t("hint_center_unavailable") : t("hint_center")}
                </p>
              </div>
            </>
          )}
          {section === "grid" && (
            <>
              <div className={styles.sectionHeader}>
                <h2 className={styles.sectionTitle}>{t("section_grid")}</h2>
              </div>
              <p className={styles.hint}>
                {t("hint_grid")}
              </p>
              <div className={styles.card}>
                <label className={styles.checkLabel}>
                  <input
                    type="checkbox"
                    className={styles.checkbox}
                    checked={gridEnabledByDefault}
                    onChange={(e) => handleGridDefaultChange(e.target.checked)}
                  />
                  {t("grid_by_default")}
                </label>
                <p className={styles.cardHint}>
                  {t("hint_grid_by_default")}
                </p>
                <div className={styles.presetWrap}>
                  <label className={styles.presetLabel} htmlFor="startup-default-layout">
                    {t("default_layout")}
                  </label>
                  <select
                    id="startup-default-layout"
                    className={styles.presetSelect}
                    value={defaultLayoutPreset}
                    onChange={(e) => setDefaultLayoutPreset(e.target.value as LayoutPreset)}
                    disabled={!gridEnabledByDefault}
                  >
                    {LAYOUT_PRESETS.map((p) => (
                      <option key={p} value={p}>
                        {t(`preset_${p}` as "preset_3x2")}
                      </option>
                    ))}
                  </select>
                </div>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
