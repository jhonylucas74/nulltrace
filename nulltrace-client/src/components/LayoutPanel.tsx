import { useState, useEffect, useRef } from "react";
import { LayoutTemplate } from "lucide-react";
import { useWorkspaceLayout } from "../contexts/WorkspaceLayoutContext";
import type { LayoutPreset } from "../contexts/WorkspaceLayoutContext";
import styles from "./LayoutPanel.module.css";

const PRESETS: { value: LayoutPreset }[] = [
  { value: "3x2" },
  { value: "2x2" },
  { value: "2x1" },
  { value: "2+1" },
  { value: "1+2" },
  { value: "1x1" },
];

/** Number of cells and layout hint per preset for mini-grid. */
function getPresetCells(preset: LayoutPreset): { count: number; layout: string } {
  switch (preset) {
    case "3x2":
      return { count: 6, layout: "3x2" };
    case "2x2":
      return { count: 4, layout: "2x2" };
    case "2x1":
      return { count: 2, layout: "2x1" };
    case "2+1":
      return { count: 3, layout: "2p1" };
    case "1+2":
      return { count: 3, layout: "1p2" };
    case "1x1":
      return { count: 1, layout: "1x1" };
    default:
      return { count: 6, layout: "3x2" };
  }
}

function PresetVisual({ preset }: { preset: LayoutPreset }) {
  const { count, layout } = getPresetCells(preset);
  return (
    <div className={`${styles.presetVisual} ${styles[`presetVisual_${layout}`]}`}>
      {Array.from({ length: count }, (_, i) => (
        <div key={i} className={styles.presetCell} />
      ))}
    </div>
  );
}

export default function LayoutPanel() {
  const [open, setOpen] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);
  const { gridModeEnabled, layoutPreset, setGridMode, setLayoutPreset } = useWorkspaceLayout();

  useEffect(() => {
    if (!open) return;
    function handleEscape(e: KeyboardEvent) {
      if (e.key === "Escape") setOpen(false);
    }
    document.addEventListener("keydown", handleEscape);
    return () => document.removeEventListener("keydown", handleEscape);
  }, [open]);

  useEffect(() => {
    if (!open) return;
    function handleClickOutside(e: MouseEvent) {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [open]);

  return (
    <div className={styles.wrap} ref={panelRef}>
      <button
        type="button"
        className={styles.trigger}
        onClick={() => setOpen((o) => !o)}
        title="Layout options"
        aria-expanded={open}
        aria-haspopup="true"
      >
        <span className={styles.icon}>
          <LayoutTemplate size={24} />
        </span>
      </button>
      {open && (
        <div className={styles.popup} role="menu">
          <label className={styles.checkLabel}>
            <input
              type="checkbox"
              checked={gridModeEnabled}
              onChange={(e) => setGridMode(e.target.checked)}
              className={styles.checkbox}
            />
            <span>Grid layout</span>
          </label>
          <div className={styles.presets}>
            <span className={styles.presetsLabel}>Layout</span>
            <div className={styles.presetGrid}>
              {PRESETS.map((p) => (
                <button
                  key={p.value}
                  type="button"
                  className={styles.presetBtn}
                  disabled={!gridModeEnabled}
                  onClick={() => setLayoutPreset(p.value)}
                  aria-pressed={layoutPreset === p.value}
                  title={p.value}
                >
                  <PresetVisual preset={p.value} />
                </button>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
