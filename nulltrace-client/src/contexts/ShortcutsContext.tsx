import React, {
  createContext,
  useContext,
  useState,
  useCallback,
  useEffect,
  useRef,
} from "react";
import { useAuth } from "./AuthContext";
import { useGrpc } from "./GrpcContext";

const STORAGE_KEY = "nulltrace-shortcuts";

export type ShortcutActionId =
  | "appLauncher"
  | "toggleGrid"
  | "nextWorkspace"
  | "prevWorkspace"
  | "nextWorkspaceAlt"
  | "prevWorkspaceAlt"
  | "goToWorkspace1"
  | "goToWorkspace2"
  | "goToWorkspace3"
  | "goToWorkspace4"
  | "goToWorkspace5"
  | "goToWorkspace6"
  | "goToWorkspace7"
  | "goToWorkspace8"
  | "goToWorkspace9";

export interface ShortcutAction {
  actionId: ShortcutActionId;
  label: string;
  defaultKeys: string[];
}

export const SHORTCUT_ACTIONS: ShortcutAction[] = [
  { actionId: "appLauncher", label: "Open App Launcher", defaultKeys: ["Alt", " "] },
  { actionId: "toggleGrid", label: "Toggle Grid Layout", defaultKeys: ["Alt", "g"] },
  { actionId: "nextWorkspace", label: "Next workspace", defaultKeys: ["Meta", "ArrowRight"] },
  { actionId: "prevWorkspace", label: "Previous workspace", defaultKeys: ["Meta", "ArrowLeft"] },
  { actionId: "nextWorkspaceAlt", label: "Next workspace (Alt+arrows)", defaultKeys: ["Alt", "ArrowRight"] },
  { actionId: "prevWorkspaceAlt", label: "Previous workspace (Alt+arrows)", defaultKeys: ["Alt", "ArrowLeft"] },
  { actionId: "goToWorkspace1", label: "Go to workspace 1", defaultKeys: ["Alt", "1"] },
  { actionId: "goToWorkspace2", label: "Go to workspace 2", defaultKeys: ["Alt", "2"] },
  { actionId: "goToWorkspace3", label: "Go to workspace 3", defaultKeys: ["Alt", "3"] },
  { actionId: "goToWorkspace4", label: "Go to workspace 4", defaultKeys: ["Alt", "4"] },
  { actionId: "goToWorkspace5", label: "Go to workspace 5", defaultKeys: ["Alt", "5"] },
  { actionId: "goToWorkspace6", label: "Go to workspace 6", defaultKeys: ["Alt", "6"] },
  { actionId: "goToWorkspace7", label: "Go to workspace 7", defaultKeys: ["Alt", "7"] },
  { actionId: "goToWorkspace8", label: "Go to workspace 8", defaultKeys: ["Alt", "8"] },
  { actionId: "goToWorkspace9", label: "Go to workspace 9", defaultKeys: ["Alt", "9"] },
];

function loadOverrides(): Partial<Record<ShortcutActionId, string[]>> {
  if (typeof window === "undefined") return {};
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Record<string, string[]>;
    const out: Partial<Record<ShortcutActionId, string[]>> = {};
    for (const id of SHORTCUT_ACTIONS.map((a) => a.actionId)) {
      if (Array.isArray(parsed[id])) out[id as ShortcutActionId] = parsed[id];
    }
    return out;
  } catch {
    return {};
  }
}

/** Parse server shortcuts_overrides JSON into overrides map; return null if invalid or empty. */
function parseServerShortcuts(raw: string | undefined): Partial<Record<ShortcutActionId, string[]>> | null {
  if (!raw || !raw.trim()) return null;
  try {
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) return null;
    const out: Partial<Record<ShortcutActionId, string[]>> = {};
    const validIds = new Set(SHORTCUT_ACTIONS.map((a) => a.actionId));
    for (const id of Object.keys(parsed)) {
      if (!validIds.has(id as ShortcutActionId)) continue;
      const val = parsed[id];
      if (!Array.isArray(val) || !val.every((x) => typeof x === "string")) continue;
      out[id as ShortcutActionId] = val as string[];
    }
    return Object.keys(out).length > 0 ? out : null;
  } catch {
    return null;
  }
}

function saveOverrides(overrides: Partial<Record<ShortcutActionId, string[]>>) {
  if (typeof window === "undefined") return;
  localStorage.setItem(STORAGE_KEY, JSON.stringify(overrides));
}

/** Normalize key combo for comparison: sort modifiers, then key. */
function normalizeCombo(keys: string[]): string[] {
  const modOrder = ["Meta", "Control", "Alt", "Shift"];
  const mods = keys.filter((k) => modOrder.includes(k)).sort((a, b) => modOrder.indexOf(a) - modOrder.indexOf(b));
  const rest = keys.filter((k) => !modOrder.includes(k));
  return [...mods, ...rest];
}

function comboKey(keys: string[]): string {
  return normalizeCombo(keys).join("+");
}

interface ShortcutsContextValue {
  getShortcuts: () => { actionId: ShortcutActionId; label: string; keys: string[]; defaultKeys: string[] }[];
  setShortcut: (actionId: ShortcutActionId, keys: string[]) => void;
  resetShortcut: (actionId: ShortcutActionId) => void;
  resetAllShortcuts: () => void;
  getKeysForAction: (actionId: ShortcutActionId) => string[];
  registerActionHandler: (actionId: ShortcutActionId, callback: () => void) => () => void;
  isRecording: boolean;
  recordingActionId: ShortcutActionId | null;
  startRecording: (actionId: ShortcutActionId) => void;
  stopRecording: () => void;
}

const ShortcutsContext = createContext<ShortcutsContextValue | null>(null);

export function ShortcutsProvider({ children }: { children: React.ReactNode }) {
  const [overrides, setOverrides] = useState<Partial<Record<ShortcutActionId, string[]>>>(() => loadOverrides());
  const [recordingActionId, setRecordingActionId] = useState<ShortcutActionId | null>(null);
  const handlersRef = useRef<Partial<Record<ShortcutActionId, () => void>>>({});
  const { token } = useAuth();
  const { getPlayerProfile, setShortcuts } = useGrpc();
  const isRecording = recordingActionId !== null;
  const startRecording = useCallback((actionId: ShortcutActionId) => setRecordingActionId(actionId), []);
  const stopRecording = useCallback(() => setRecordingActionId(null), []);

  // When we have a token, fetch profile and apply server shortcuts_overrides (if valid and non-empty).
  useEffect(() => {
    if (!token) return;
    getPlayerProfile(token)
      .then((profile) => {
        const serverOverrides = parseServerShortcuts(profile.shortcuts_overrides);
        if (serverOverrides) {
          setOverrides(serverOverrides);
          saveOverrides(serverOverrides);
        }
      })
      .catch(() => {
        // Keep current theme (localStorage or default)
      });
  }, [token, getPlayerProfile]);

  const getKeysForAction = useCallback((actionId: ShortcutActionId): string[] => {
    if (overrides[actionId]) return overrides[actionId]!;
    const action = SHORTCUT_ACTIONS.find((a) => a.actionId === actionId);
    return action?.defaultKeys ?? [];
  }, [overrides]);

  const setShortcut = useCallback(
    (actionId: ShortcutActionId, keys: string[]) => {
      setOverrides((prev) => {
        const next = { ...prev, [actionId]: keys };
        saveOverrides(next);
        if (token) {
          setShortcuts(token, JSON.stringify(next)).catch(() => {});
        }
        return next;
      });
    },
    [token, setShortcuts]
  );

  const resetShortcut = useCallback(
    (actionId: ShortcutActionId) => {
      setOverrides((prev) => {
        const next = { ...prev };
        delete next[actionId];
        saveOverrides(next);
        if (token) {
          setShortcuts(token, JSON.stringify(next)).catch(() => {});
        }
        return next;
      });
    },
    [token, setShortcuts]
  );

  const resetAllShortcuts = useCallback(() => {
    setOverrides({});
    saveOverrides({});
    if (token) {
      setShortcuts(token, "{}").catch(() => {});
    }
  }, [token, setShortcuts]);

  const getShortcuts = useCallback(() => {
    return SHORTCUT_ACTIONS.map((a) => ({
      actionId: a.actionId,
      label: a.label,
      keys: getKeysForAction(a.actionId),
      defaultKeys: a.defaultKeys,
    }));
  }, [getKeysForAction]);

  const registerActionHandler = useCallback((actionId: ShortcutActionId, callback: () => void) => {
    handlersRef.current[actionId] = callback;
    return () => {
      delete handlersRef.current[actionId];
    };
  }, []);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (isRecording) return;
      const target = document.activeElement;
      const tag = target?.tagName?.toLowerCase();
      if (tag === "input" || tag === "textarea" || (target as HTMLElement)?.getAttribute?.("contenteditable") === "true") {
        return;
      }
      const mods: string[] = [];
      if (e.metaKey) mods.push("Meta");
      if (e.ctrlKey) mods.push("Control");
      if (e.altKey) mods.push("Alt");
      if (e.shiftKey) mods.push("Shift");
      const key = e.key === " " ? " " : e.key;
      const combo = normalizeCombo([...mods, key]);
      const comboStr = comboKey(combo);
      for (const action of SHORTCUT_ACTIONS) {
        const bound = getKeysForAction(action.actionId);
        if (bound.length && comboKey(bound) === comboStr) {
          e.preventDefault();
          handlersRef.current[action.actionId]?.();
          return;
        }
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isRecording, getKeysForAction]);

  const value: ShortcutsContextValue = {
    getShortcuts,
    setShortcut,
    resetShortcut,
    resetAllShortcuts,
    getKeysForAction,
    registerActionHandler,
    isRecording,
    recordingActionId,
    startRecording,
    stopRecording,
  };

  return (
    <ShortcutsContext.Provider value={value}>
      {children}
    </ShortcutsContext.Provider>
  );
}

export function useShortcuts(): ShortcutsContextValue {
  const ctx = useContext(ShortcutsContext);
  if (!ctx) throw new Error("useShortcuts must be used within ShortcutsProvider");
  return ctx;
}
