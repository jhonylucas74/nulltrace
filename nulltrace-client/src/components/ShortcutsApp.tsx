import { useEffect, useCallback } from "react";
import { useShortcuts } from "../contexts/ShortcutsContext";
import KeyIcon from "./KeyIcon";
import styles from "./ShortcutsApp.module.css";

/** Compare two key combos (order-independent for modifiers). */
function sameCombo(a: string[], b: string[]): boolean {
  const order = ["Meta", "Control", "Alt", "Shift"];
  const sort = (arr: string[]) => {
    const mods = arr.filter((k) => order.includes(k)).sort((x, y) => order.indexOf(x) - order.indexOf(y));
    const rest = arr.filter((k) => !order.includes(k));
    return [...mods, ...rest].join("+");
  };
  return a.length === b.length && sort(a) === sort(b);
}

export default function ShortcutsApp() {
  const {
    getShortcuts,
    setShortcut,
    resetShortcut,
    resetAllShortcuts,
    startRecording,
    stopRecording,
    recordingActionId,
  } = useShortcuts();

  const shortcuts = getShortcuts();

  const MODIFIERS = ["Meta", "Control", "Alt", "Shift"];

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (recordingActionId === null) return;
      e.preventDefault();
      e.stopPropagation();
      // Ignore key repeat (holding a key fires repeated keydown events)
      if (e.repeat) return;
      const mods: string[] = [];
      if (e.metaKey) mods.push("Meta");
      if (e.ctrlKey) mods.push("Control");
      if (e.altKey) mods.push("Alt");
      if (e.shiftKey) mods.push("Shift");
      const key = e.key === " " ? " " : e.key;
      if (e.key === "Escape") {
        stopRecording();
        return;
      }
      // Don't add key twice when the key itself is a modifier (e.g. Alt + ArrowRight: first keydown is Alt only)
      const combo = mods.includes(key) ? [...mods] : [...mods, key];
      // Wait for a non-modifier key before saving (e.g. Alt then ArrowRight, not just Alt)
      if (combo.length > 0 && combo.every((k) => MODIFIERS.includes(k))) return;
      setShortcut(recordingActionId, combo);
      stopRecording();
    },
    [recordingActionId, setShortcut, stopRecording]
  );

  useEffect(() => {
    if (recordingActionId === null) return;
    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [recordingActionId, handleKeyDown]);

  const hasAnyOverride = shortcuts.some((s) => !sameCombo(s.keys, s.defaultKeys));

  return (
    <div className={styles.app}>
      <p className={styles.intro}>
        View and edit keyboard shortcuts. Click Edit and press the keys you want to use.
      </p>
      {hasAnyOverride && (
        <div className={styles.topBar}>
          <button
            type="button"
            className={styles.restoreAllBtn}
            onClick={resetAllShortcuts}
            aria-label="Restore all shortcuts to defaults"
          >
            Restore all
          </button>
        </div>
      )}
      <div className={styles.list}>
        {shortcuts.map((s) => {
          const isDefault = sameCombo(s.keys, s.defaultKeys);
          return (
            <div
              key={s.actionId}
              className={`${styles.row} ${recordingActionId === s.actionId ? styles.rowRecording : ""}`}
            >
              <span className={styles.label}>{s.label}</span>
              <span className={styles.keys}>
                {recordingActionId === s.actionId ? (
                  <span className={styles.recordingHint}>Press keys…</span>
                ) : (
                  s.keys.map((key) => (
                    <KeyIcon key={`${s.actionId}-${key}`} keyKey={key} className={styles.keyIconWrap} />
                  ))
                )}
              </span>
              <div className={styles.rowActions}>
                <button
                  type="button"
                  className={styles.resetBtn}
                  onClick={() => resetShortcut(s.actionId)}
                  disabled={isDefault}
                  aria-label={`Reset shortcut for ${s.label} to default`}
                >
                  Reset
                </button>
                <button
                  type="button"
                  className={styles.editBtn}
                  onClick={() => startRecording(s.actionId)}
                  disabled={recordingActionId !== null && recordingActionId !== s.actionId}
                  aria-label={`Edit shortcut for ${s.label}`}
                >
                  Edit
                </button>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
