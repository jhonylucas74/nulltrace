import styles from "./KeyIcon.module.css";

/** Base path for Kenney Keyboard & Mouse Vector SVGs (public folder). Use literal path so the browser encodes it when requesting. */
const KENNEY_VECTOR_BASE = `${(import.meta.env.BASE_URL || "/").replace(/\/$/, "")}/Keyboard & Mouse/Vector`;

/** Map KeyboardEvent key to Kenney asset filename (without .svg). */
function keyToAssetName(key: string): string {
  const k = key.length === 1 ? key.toLowerCase() : key;
  const map: Record<string, string> = {
    Meta: "keyboard_win",
    Control: "keyboard_ctrl",
    Alt: "keyboard_option",
    Shift: "keyboard_shift",
    " ": "keyboard_space_icon",
    Escape: "keyboard_escape",
    Tab: "keyboard_tab",
    Enter: "keyboard_return",
    Backspace: "keyboard_backspace_icon",
    ArrowUp: "keyboard_arrow_up",
    ArrowDown: "keyboard_arrow_down",
    ArrowLeft: "keyboard_arrow_left",
    ArrowRight: "keyboard_arrow_right",
    F1: "keyboard_f1",
    F2: "keyboard_f2",
    F3: "keyboard_f3",
    F4: "keyboard_f4",
    F5: "keyboard_f5",
    F6: "keyboard_f6",
    F7: "keyboard_f7",
    F8: "keyboard_f8",
    F9: "keyboard_f9",
    F10: "keyboard_f10",
    F11: "keyboard_f11",
    F12: "keyboard_f12",
    Home: "keyboard_home",
    End: "keyboard_end",
    Insert: "keyboard_insert",
    Delete: "keyboard_delete",
    PageUp: "keyboard_page_up",
    PageDown: "keyboard_page_down",
  };
  if (map[k] !== undefined) return map[k];
  if (/^[a-z]$/.test(k)) return `keyboard_${k}`;
  if (/^[0-9]$/.test(k)) return `keyboard_${k}`;
  const special: Record<string, string> = {
    ",": "keyboard_comma",
    ".": "keyboard_period",
    "/": "keyboard_slash_forward",
    ";": "keyboard_semicolon",
    "'": "keyboard_apostrophe",
    "[": "keyboard_bracket_open",
    "]": "keyboard_bracket_close",
    "-": "keyboard_minus",
    "=": "keyboard_equals",
    "`": "keyboard_tilde",
  };
  if (special[k] !== undefined) return special[k];
  return "keyboard_outline";
}

interface KeyIconProps {
  keyKey: string;
  className?: string;
}

export default function KeyIcon({ keyKey, className }: KeyIconProps) {
  const name = keyToAssetName(keyKey);
  const displayLabel = keyKey === " " ? "Space" : keyKey;
  const src = `${KENNEY_VECTOR_BASE}/${name}.svg`;
  return (
    <img
      src={src}
      alt={displayLabel}
      className={`${styles.keyIcon} ${className ?? ""}`}
      title={displayLabel}
    />
  );
}
