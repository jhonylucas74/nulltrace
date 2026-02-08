import React, { createContext, useContext, useReducer, useCallback, useMemo, useEffect, useRef } from "react";
import { useWindowManager } from "./WindowManagerContext";
import type { WindowType } from "./WindowManagerContext";
import type { GridSlot } from "./WindowManagerContext";
import type { OpenWindowOptions } from "./WindowManagerContext";

export type LayoutPreset = "3x2" | "2x2" | "2x1" | "2+1" | "1+2" | "1x1";

export interface Workspace {
  id: string;
  label: string;
}

interface WorkspaceLayoutState {
  gridModeEnabled: boolean;
  /** Layout preset per workspace (workspace id -> preset). */
  workspaceLayout: Record<string, LayoutPreset>;
  workspaces: Workspace[];
  activeWorkspaceId: string;
}

type WorkspaceLayoutAction =
  | { type: "setGridMode"; payload: boolean }
  | { type: "setLayoutPreset"; payload: { workspaceId: string; preset: LayoutPreset } }
  | { type: "setActiveWorkspace"; payload: string }
  | { type: "addWorkspace"; payload: Workspace }
  | { type: "removeWorkspace"; payload: string }
  | { type: "initWorkspaces"; payload: Workspace[] };

function reducer(state: WorkspaceLayoutState, action: WorkspaceLayoutAction): WorkspaceLayoutState {
  switch (action.type) {
    case "setGridMode":
      return { ...state, gridModeEnabled: action.payload };
    case "setLayoutPreset": {
      const { workspaceId, preset } = action.payload;
      return {
        ...state,
        workspaceLayout: { ...state.workspaceLayout, [workspaceId]: preset },
      };
    }
    case "setActiveWorkspace":
      return { ...state, activeWorkspaceId: action.payload };
    case "addWorkspace": {
      const ws = action.payload;
      return {
        ...state,
        workspaces: [...state.workspaces, ws],
        workspaceLayout: { ...state.workspaceLayout, [ws.id]: "2x2" },
      };
    }
    case "removeWorkspace": {
      const id = action.payload;
      const next = state.workspaces.filter((w) => w.id !== id);
      const nextActive =
        state.activeWorkspaceId === id ? (next[0]?.id ?? state.activeWorkspaceId) : state.activeWorkspaceId;
      const { [id]: _removed, ...nextLayout } = state.workspaceLayout;
      return { ...state, workspaces: next, activeWorkspaceId: nextActive, workspaceLayout: nextLayout };
    }
    case "initWorkspaces":
      return {
        ...state,
        workspaces: action.payload,
        activeWorkspaceId: action.payload[0]?.id ?? state.activeWorkspaceId,
      };
    default:
      return state;
  }
}

/** Returns list of (row, col) slots in display order for the preset. */
function getLayoutSlots(preset: LayoutPreset): GridSlot[] {
  switch (preset) {
    case "3x2": {
      const slots: GridSlot[] = [];
      for (let row = 0; row < 2; row++)
        for (let col = 0; col < 3; col++) slots.push({ row, col });
      return slots;
    }
    case "2x2": {
      const slots: GridSlot[] = [];
      for (let row = 0; row < 2; row++)
        for (let col = 0; col < 2; col++) slots.push({ row, col });
      return slots;
    }
    case "2x1":
      return [{ row: 0, col: 0 }, { row: 0, col: 1 }];
    case "2+1":
      return [{ row: 0, col: 0 }, { row: 0, col: 1 }, { row: 1, col: 0 }];
    case "1+2":
      return [{ row: 0, col: 0 }, { row: 1, col: 0 }, { row: 1, col: 1 }];
    case "1x1":
      return [{ row: 0, col: 0 }];
    default:
      return getLayoutSlots("2x2");
  }
}

/** Row layout: for 2+1, row0 has 2 cols, row1 has 1 col. For 1+2, row0 has 1 col, row1 has 2 cols. */
function getRowColCounts(preset: LayoutPreset): { cols: number }[] {
  switch (preset) {
    case "3x2":
      return [{ cols: 3 }, { cols: 3 }];
    case "2x2":
      return [{ cols: 2 }, { cols: 2 }];
    case "2x1":
      return [{ cols: 2 }];
    case "2+1":
      return [{ cols: 2 }, { cols: 1 }];
    case "1+2":
      return [{ cols: 1 }, { cols: 2 }];
    case "1x1":
      return [{ cols: 1 }];
    default:
      return [{ cols: 2 }, { cols: 2 }];
  }
}

const PADDING = 16;
const GRID_GAP = 16;

export interface WorkspaceArea {
  left: number;
  top: number;
  width: number;
  height: number;
  workspaceViewportTop?: number;
}

/** Workspace-relative padded area (16px from top bar and dock); includes workspaceViewportTop for client-to-workspace conversion. */
export function getWorkspaceArea(): WorkspaceArea {
  if (typeof window === "undefined")
    return { left: PADDING, top: PADDING, width: 800 - 2 * PADDING, height: 600 - 2 * PADDING, workspaceViewportTop: 36 };
  const barHeight = 36;
  const dockBottom = 6;
  const dockHeight = 56;
  const workspaceHeight = window.innerHeight - barHeight - dockBottom - dockHeight;
  return {
    left: PADDING,
    top: PADDING,
    width: window.innerWidth - 2 * PADDING,
    height: Math.max(200 - 2 * PADDING, workspaceHeight - 2 * PADDING),
    workspaceViewportTop: barHeight,
  };
}

/** Returns the slot that contains the given client coordinates, or null. Converts client Y to area coords using workspaceViewportTop when set. */
export function getSlotAtPoint(
  area: WorkspaceArea,
  preset: LayoutPreset,
  clientX: number,
  clientY: number
): GridSlot | null {
  const relX = clientX;
  const relY = area.workspaceViewportTop !== undefined ? clientY - area.workspaceViewportTop : clientY;
  const slots = getLayoutSlots(preset);
  for (const slot of slots) {
    const b = getSlotBounds(preset, slot, area);
    if (
      relX >= b.x &&
      relX < b.x + b.width &&
      relY >= b.y &&
      relY < b.y + b.height
    ) {
      return slot;
    }
  }
  return null;
}

/** Compute cell bounds for a slot (workspace-relative), with GRID_GAP between cells. */
export function getSlotBounds(
  preset: LayoutPreset,
  slot: GridSlot,
  area: { left: number; top: number; width: number; height: number }
): { x: number; y: number; width: number; height: number } {
  const rowLayout = getRowColCounts(preset);
  const rowCount = rowLayout.length;
  const totalVerticalGap = (rowCount - 1) * GRID_GAP;
  const availableHeight = area.height - totalVerticalGap;
  const cellH = availableHeight / rowCount;
  const y = area.top + slot.row * (cellH + GRID_GAP);
  const height = (slot.rowSpan ?? 1) * cellH;

  const colsInRow = rowLayout[slot.row]?.cols ?? 1;
  const totalHorizontalGap = (colsInRow - 1) * GRID_GAP;
  const availableWidth = area.width - totalHorizontalGap;
  const cellW = availableWidth / colsInRow;
  const x = area.left + slot.col * (cellW + GRID_GAP);
  const width = (slot.colSpan ?? 1) * cellW;

  return { x, y, width, height };
}

const DEFAULT_WORKSPACE_IDS = ["ws-1", "ws-2", "ws-3", "ws-4"];

const INITIAL_WORKSPACES: Workspace[] = [
  { id: "ws-1", label: "Workspace 1" },
  { id: "ws-2", label: "Workspace 2" },
  { id: "ws-3", label: "Workspace 3" },
  { id: "ws-4", label: "Workspace 4" },
];

const INITIAL_WORKSPACE_LAYOUT: Record<string, LayoutPreset> = Object.fromEntries(
  INITIAL_WORKSPACES.map((ws) => [ws.id, "2x2"])
);

/** Returns a map of slot key "row,col" to window id for the given workspace (excluding optional window). */
export function getOccupiedSlots(
  windows: { id: string; workspaceId: string; gridSlot?: GridSlot }[],
  workspaceId: string,
  _preset: LayoutPreset,
  excludeWindowId?: string
): Map<string, string> {
  const map = new Map<string, string>();
  for (const w of windows) {
    if (w.workspaceId !== workspaceId || !w.gridSlot || w.id === excludeWindowId) continue;
    const s = w.gridSlot;
    const rowSpan = s.rowSpan ?? 1;
    const colSpan = s.colSpan ?? 1;
    for (let r = 0; r < rowSpan; r++)
      for (let c = 0; c < colSpan; c++) map.set(`${s.row + r},${s.col + c}`, w.id);
  }
  return map;
}

interface WorkspaceLayoutValue {
  gridModeEnabled: boolean;
  layoutPreset: LayoutPreset;
  workspaces: Workspace[];
  activeWorkspaceId: string;
  setGridMode: (enabled: boolean) => void;
  setLayoutPreset: (preset: LayoutPreset) => void;
  setActiveWorkspace: (id: string) => void;
  addWorkspace: () => Workspace;
  moveWindowToWorkspace: (winId: string, workspaceId: string) => void;
  getLayoutForWorkspace: (workspaceId: string) => LayoutPreset;
  getFirstFreeSlot: (workspaceId: string) => { workspaceId: string; slot: GridSlot } | null;
  getSlotAtPoint: (area: { left: number; top: number; width: number; height: number }, clientX: number, clientY: number) => GridSlot | null;
  getOccupiedSlots: (workspaceId: string, excludeWindowId?: string) => Map<string, string>;
  openApp: (type: WindowType, options?: OpenWindowOptions) => string;
}

const WorkspaceLayoutContext = createContext<WorkspaceLayoutValue | null>(null);

let nextWorkspaceNum = 5;

export function WorkspaceLayoutProvider({ children }: { children: React.ReactNode }) {
  const [state, dispatch] = useReducer(reducer, {
    gridModeEnabled: false,
    workspaceLayout: INITIAL_WORKSPACE_LAYOUT,
    workspaces: INITIAL_WORKSPACES,
    activeWorkspaceId: INITIAL_WORKSPACES[0].id,
  });

  const { windows, open: wmOpen, setWindowWorkspace, move, resize, setWindowGridSlot } = useWindowManager();

  const getLayoutForWorkspace = useCallback(
    (workspaceId: string): LayoutPreset => state.workspaceLayout[workspaceId] ?? "2x2",
    [state.workspaceLayout]
  );

  const setGridMode = useCallback((enabled: boolean) => {
    dispatch({ type: "setGridMode", payload: enabled });
  }, []);

  const setLayoutPreset = useCallback(
    (preset: LayoutPreset) => {
      dispatch({ type: "setLayoutPreset", payload: { workspaceId: state.activeWorkspaceId, preset } });
    },
    [state.activeWorkspaceId]
  );

  const prevWorkspaceLayoutRef = useRef<Record<string, LayoutPreset>>({});
  const prevGridModeRef = useRef(state.gridModeEnabled);

  // Remove empty temporary workspaces (non-default only).
  useEffect(() => {
    for (const ws of state.workspaces) {
      if (DEFAULT_WORKSPACE_IDS.includes(ws.id)) continue;
      const count = windows.filter((w) => w.workspaceId === ws.id).length;
      if (count === 0) {
        dispatch({ type: "removeWorkspace", payload: ws.id });
        break;
      }
    }
  }, [windows, state.workspaces]);

  const setActiveWorkspace = useCallback((id: string) => {
    dispatch({ type: "setActiveWorkspace", payload: id });
  }, []);

  const addWorkspace = useCallback((): Workspace => {
    const n = nextWorkspaceNum++;
    const ws: Workspace = { id: `ws-${n}`, label: `Workspace ${n}` };
    dispatch({ type: "addWorkspace", payload: ws });
    return ws;
  }, []);

  // When grid mode is turned ON, snap all floating windows (no gridSlot) into slots.
  // Prefer each window's current workspace; if no free slot, search other workspaces or add one.
  useEffect(() => {
    const wasOff = !prevGridModeRef.current;
    prevGridModeRef.current = state.gridModeEnabled;
    if (!state.gridModeEnabled || !wasOff) return;

    const floating = windows.filter((w) => !w.gridSlot);
    if (floating.length === 0) return;

    const area = getWorkspaceArea();
    const slotKey = (s: GridSlot) => `${s.row},${s.col}`;
    const occupied: Record<string, Map<string, string>> = {};
    for (const ws of state.workspaces) {
      occupied[ws.id] = new Map();
    }
    for (const win of windows) {
      if (!win.gridSlot) continue;
      const key = slotKey(win.gridSlot);
      if (occupied[win.workspaceId]) occupied[win.workspaceId].set(key, win.id);
    }

    const getFirstFreeIn = (workspaceId: string): GridSlot | null => {
      const preset = state.workspaceLayout[workspaceId] ?? "2x2";
      const slots = getLayoutSlots(preset);
      const occ = occupied[workspaceId];
      if (!occ) return null;
      for (const slot of slots) {
        if (!occ.has(slotKey(slot))) return slot;
      }
      return null;
    };

    const assignToSlot = (win: { id: string; workspaceId: string }, workspaceId: string, slot: GridSlot) => {
      occupied[workspaceId].set(slotKey(slot), win.id);
      const preset = state.workspaceLayout[workspaceId] ?? "2x2";
      const bounds = getSlotBounds(preset, slot, area);
      if (win.workspaceId !== workspaceId) setWindowWorkspace(win.id, workspaceId);
      move(win.id, bounds.x, bounds.y);
      resize(win.id, bounds.width, bounds.height);
      setWindowGridSlot(win.id, slot);
    };

    for (const win of floating) {
      let slot = getFirstFreeIn(win.workspaceId);
      let targetWsId = win.workspaceId;
      if (!slot) {
        for (const ws of state.workspaces) {
          if (ws.id === win.workspaceId) continue;
          slot = getFirstFreeIn(ws.id);
          if (slot) {
            targetWsId = ws.id;
            break;
          }
        }
      }
      if (!slot) {
        const newWs = addWorkspace();
        targetWsId = newWs.id;
        occupied[targetWsId] = new Map();
        slot = getLayoutSlots("2x2")[0] ?? { row: 0, col: 0 };
      }
      assignToSlot(win, targetWsId, slot);
    }
  }, [
    state.gridModeEnabled,
    state.workspaces,
    state.workspaceLayout,
    windows,
    move,
    resize,
    setWindowGridSlot,
    setWindowWorkspace,
    addWorkspace,
  ]);

  // Re-snap grid windows when their workspace's layout changes. If new layout has fewer slots,
  // overflow windows move to the next workspace with a free slot (in order).
  useEffect(() => {
    const currentLayout = state.workspaceLayout;
    const prevLayout = prevWorkspaceLayoutRef.current;
    prevWorkspaceLayoutRef.current = { ...currentLayout };

    const area = getWorkspaceArea();
    const slotKey = (s: GridSlot) => `${s.row},${s.col}`;

    // Build occupied map per workspace (slotKey -> windowId) from current windows.
    const occupied: Record<string, Map<string, string>> = {};
    for (const ws of state.workspaces) {
      occupied[ws.id] = new Map();
    }
    for (const win of windows) {
      if (!win.gridSlot) continue;
      const key = slotKey(win.gridSlot);
      if (occupied[win.workspaceId]) occupied[win.workspaceId].set(key, win.id);
    }

    // Find first free slot in a workspace given current occupied map.
    const getFirstFreeSlotIn = (workspaceId: string): GridSlot | null => {
      const preset = state.workspaceLayout[workspaceId] ?? "2x2";
      const slots = getLayoutSlots(preset);
      const occ = occupied[workspaceId];
      if (!occ) return null;
      for (const slot of slots) {
        if (!occ.has(slotKey(slot))) return slot;
      }
      return null;
    };

    for (const ws of state.workspaces) {
      const oldPreset = prevLayout[ws.id] ?? "2x2";
      const newPreset = currentLayout[ws.id] ?? "2x2";
      if (oldPreset === newPreset) continue;

      const winsInWs = windows
        .filter((w) => w.workspaceId === ws.id && w.gridSlot)
        .map((w) => ({ win: w, oldIndex: getLayoutSlots(oldPreset).findIndex((s) => s.row === w.gridSlot!.row && s.col === w.gridSlot!.col) }))
        .sort((a, b) => (a.oldIndex < 0 ? 0 : a.oldIndex) - (b.oldIndex < 0 ? 0 : b.oldIndex))
        .map((x) => x.win);

      const newSlots = getLayoutSlots(newPreset);
      if (newSlots.length === 0) continue;

      occupied[ws.id] = new Map();

      // Assign first N windows to slots 0..N-1 in this workspace (re-snap in place).
      const keepCount = Math.min(winsInWs.length, newSlots.length);
      for (let i = 0; i < keepCount; i++) {
        const win = winsInWs[i];
        const newSlot = newSlots[i];
        occupied[ws.id].set(slotKey(newSlot), win.id);
        const bounds = getSlotBounds(newPreset, newSlot, area);
        move(win.id, bounds.x, bounds.y);
        resize(win.id, bounds.width, bounds.height);
        setWindowGridSlot(win.id, newSlot);
      }

      // Overflow: move each extra window to the next workspace with a free slot.
      const overflow = winsInWs.slice(keepCount);
      for (const win of overflow) {
        let targetWsId: string | null = null;
        let targetSlot: GridSlot | null = null;
        for (const other of state.workspaces) {
          targetSlot = getFirstFreeSlotIn(other.id);
          if (targetSlot) {
            targetWsId = other.id;
            break;
          }
        }
        if (!targetWsId || !targetSlot) {
          const newWs = addWorkspace();
          targetWsId = newWs.id;
          occupied[targetWsId] = new Map();
          targetSlot = getLayoutSlots(state.workspaceLayout[targetWsId] ?? "2x2")[0] ?? { row: 0, col: 0 };
        }
        occupied[targetWsId].set(slotKey(targetSlot), win.id);
        const targetPreset = state.workspaceLayout[targetWsId] ?? "2x2";
        const bounds = getSlotBounds(targetPreset, targetSlot, area);
        setWindowWorkspace(win.id, targetWsId);
        move(win.id, bounds.x, bounds.y);
        resize(win.id, bounds.width, bounds.height);
        setWindowGridSlot(win.id, targetSlot);
      }
    }
  }, [state.workspaceLayout, state.workspaces, windows, move, resize, setWindowGridSlot, setWindowWorkspace, addWorkspace]);

  const getFirstFreeSlot = useCallback(
    (workspaceId: string): { workspaceId: string; slot: GridSlot } | null => {
      const preset = getLayoutForWorkspace(workspaceId);
      const slots = getLayoutSlots(preset);
      const occupied = getOccupiedSlots(windows, workspaceId, preset);
      for (const slot of slots) {
        const key = `${slot.row},${slot.col}`;
        if (!occupied.has(key)) return { workspaceId, slot };
      }
      return null;
    },
    [windows, getLayoutForWorkspace]
  );

  const moveWindowToWorkspace = useCallback(
    (winId: string, workspaceId: string) => {
      if (state.gridModeEnabled) {
        let slotResult = getFirstFreeSlot(workspaceId);
        let targetWorkspaceId = workspaceId;
        if (!slotResult) {
          const newWs = addWorkspace();
          targetWorkspaceId = newWs.id;
          slotResult = getFirstFreeSlot(targetWorkspaceId);
        }
        if (slotResult) {
          const area = getWorkspaceArea();
          const preset = getLayoutForWorkspace(slotResult.workspaceId);
          const bounds = getSlotBounds(preset, slotResult.slot, area);
          setWindowWorkspace(winId, slotResult.workspaceId);
          move(winId, bounds.x, bounds.y);
          resize(winId, bounds.width, bounds.height);
          setWindowGridSlot(winId, slotResult.slot);
          dispatch({ type: "setActiveWorkspace", payload: slotResult.workspaceId });
          return;
        }
      }
      setWindowWorkspace(winId, workspaceId);
      dispatch({ type: "setActiveWorkspace", payload: workspaceId });
    },
    [
      state.gridModeEnabled,
      setWindowWorkspace,
      getFirstFreeSlot,
      getLayoutForWorkspace,
      addWorkspace,
      move,
      resize,
      setWindowGridSlot,
    ]
  );

  const getSlotAtPointCallback = useCallback(
    (area: { left: number; top: number; width: number; height: number }, clientX: number, clientY: number) =>
      getSlotAtPoint(area, getLayoutForWorkspace(state.activeWorkspaceId), clientX, clientY),
    [state.activeWorkspaceId, getLayoutForWorkspace]
  );

  const getOccupiedSlotsCallback = useCallback(
    (workspaceId: string, excludeWindowId?: string) =>
      getOccupiedSlots(windows, workspaceId, getLayoutForWorkspace(workspaceId), excludeWindowId),
    [windows, getLayoutForWorkspace]
  );

  const openApp = useCallback(
    (type: WindowType, options?: OpenWindowOptions): string => {
      const area = getWorkspaceArea();
      if (state.gridModeEnabled) {
        let slotResult: { workspaceId: string; slot: GridSlot } | null = null;
        for (const ws of state.workspaces) {
          slotResult = getFirstFreeSlot(ws.id);
          if (slotResult) break;
        }
        if (!slotResult) {
          const newWs = addWorkspace();
          slotResult = getFirstFreeSlot(newWs.id);
        }
        if (slotResult) {
          const preset = getLayoutForWorkspace(slotResult.workspaceId);
          const bounds = getSlotBounds(preset, slotResult.slot, area);
          const winId = wmOpen(type, {
            ...options,
            workspaceId: slotResult.workspaceId,
            gridSlot: slotResult.slot,
            position: { x: bounds.x, y: bounds.y },
            size: { width: bounds.width, height: bounds.height },
          });
          if (slotResult.workspaceId !== state.activeWorkspaceId) {
            dispatch({ type: "setActiveWorkspace", payload: slotResult.workspaceId });
          }
          return winId;
        }
      }
      return wmOpen(type, {
        ...options,
        workspaceId: state.activeWorkspaceId,
      });
    },
    [
      state.gridModeEnabled,
      state.workspaces,
      state.activeWorkspaceId,
      getLayoutForWorkspace,
      getFirstFreeSlot,
      addWorkspace,
      wmOpen,
    ]
  );

  const layoutPreset = getLayoutForWorkspace(state.activeWorkspaceId);

  const value: WorkspaceLayoutValue = useMemo(
    () => ({
      gridModeEnabled: state.gridModeEnabled,
      layoutPreset,
      workspaces: state.workspaces,
      activeWorkspaceId: state.activeWorkspaceId,
      setGridMode,
      setLayoutPreset,
      setActiveWorkspace,
      addWorkspace,
      moveWindowToWorkspace,
      getLayoutForWorkspace,
      getFirstFreeSlot,
      getSlotAtPoint: getSlotAtPointCallback,
      getOccupiedSlots: getOccupiedSlotsCallback,
      openApp,
    }),
    [
      state.gridModeEnabled,
      layoutPreset,
      state.workspaces,
      state.activeWorkspaceId,
      setGridMode,
      setLayoutPreset,
      setActiveWorkspace,
      addWorkspace,
      moveWindowToWorkspace,
      getLayoutForWorkspace,
      getFirstFreeSlot,
      getSlotAtPointCallback,
      getOccupiedSlotsCallback,
      openApp,
    ]
  );

  return (
    <WorkspaceLayoutContext.Provider value={value}>
      {children}
    </WorkspaceLayoutContext.Provider>
  );
}

export function useWorkspaceLayout(): WorkspaceLayoutValue {
  const ctx = useContext(WorkspaceLayoutContext);
  if (!ctx) throw new Error("useWorkspaceLayout must be used within WorkspaceLayoutProvider");
  return ctx;
}
