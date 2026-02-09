import React, { createContext, useContext, useReducer, useCallback } from "react";

export type WindowType = "terminal" | "explorer" | "browser" | "apps" | "editor" | "theme" | "sound" | "network" | "email" | "wallet" | "pixelart" | "sysinfo" | "shortcuts" | "sysmon" | "nullcloud" | "hackerboard" | "startup" | "wallpaper" | "settings" | "traceroute" | "store" | "minesweeper" | "packet" | "codelab";

export interface WindowPosition {
  x: number;
  y: number;
}

export interface WindowSize {
  width: number;
  height: number;
}

export interface GridSlot {
  row: number;
  col: number;
  rowSpan?: number;
  colSpan?: number;
}

export interface WindowState {
  id: string;
  type: WindowType;
  title: string;
  position: WindowPosition;
  size: WindowSize;
  minimized: boolean;
  maximized: boolean;
  zIndex: number;
  workspaceId: string;
  gridSlot?: GridSlot;
}

export interface OpenWindowOptions {
  title?: string;
  position?: WindowPosition;
  size?: WindowSize;
  workspaceId?: string;
  gridSlot?: GridSlot;
}

interface WindowManagerState {
  windows: WindowState[];
  focusedId: string | null;
  nextZIndex: number;
}

type WindowManagerAction =
  | { type: "open"; payload: { window: WindowState } }
  | { type: "close"; payload: { id: string } }
  | { type: "minimize"; payload: { id: string } }
  | { type: "maximize"; payload: { id: string } }
  | { type: "setFocus"; payload: { id: string } }
  | { type: "move"; payload: { id: string; x: number; y: number } }
  | { type: "resize"; payload: { id: string; width: number; height: number } }
  | { type: "setWindowWorkspace"; payload: { id: string; workspaceId: string } }
  | { type: "setWindowGridSlot"; payload: { id: string; gridSlot: GridSlot | undefined } };

const DEFAULT_SIZE: WindowSize = { width: 640, height: 400 };

/** Browser and Code editor need more space; use larger default size. */
const LARGE_WINDOW_SIZE: WindowSize = { width: 900, height: 600 };

/** Pixel Art: small picker on open; resize to this when user confirms canvas size. */
export const PIXELART_EDITOR_SIZE: WindowSize = { width: 900, height: 600 };
const PIXELART_PICKER_SIZE: WindowSize = { width: 420, height: 320 };

/** Sound Manager and Network are compact. */
const SOUND_WINDOW_SIZE: WindowSize = { width: 380, height: 320 };
const NETWORK_WINDOW_SIZE: WindowSize = { width: 400, height: 280 };

/** Email app: message list + read/compose panel (large UI like pixel editor). */
const EMAIL_WINDOW_SIZE: WindowSize = { width: 900, height: 600 };

/** Wallet app: full finance UI (Overview, Statement, Transfer, Keys, Convert, NFTs). */
const WALLET_WINDOW_SIZE: WindowSize = { width: 560, height: 600 };

/** System info (Nullfetch) app: ASCII art + info block. */
const SYSINFO_WINDOW_SIZE: WindowSize = { width: 640, height: 400 };

/** Shortcuts app: list + edit bindings. */
const SHORTCUTS_WINDOW_SIZE: WindowSize = { width: 560, height: 420 };

/** System Monitor app: resources + processes. */
const SYSMON_WINDOW_SIZE: WindowSize = { width: 520, height: 480 };

/** NullCloud app: machine upgrades and VPS. */
const NULLCLOUD_WINDOW_SIZE: WindowSize = { width: 560, height: 640 };

/** Hackerboard app: feed and rankings. */
const HACKERBOARD_WINDOW_SIZE: WindowSize = { width: 600, height: 700 };

/** Startup settings app: programs at login and grid defaults. */
const STARTUP_WINDOW_SIZE: WindowSize = { width: 520, height: 480 };

/** Background app: wallpaper picker from Pexels. */
const WALLPAPER_WINDOW_SIZE: WindowSize = { width: 560, height: 520 };

/** Settings app: preferences and window config. */
const SETTINGS_WINDOW_SIZE: WindowSize = { width: 560, height: 520 };

/** TraceRoute app: world map and route visualization. */
const TRACEROUTE_WINDOW_SIZE: WindowSize = { width: 880, height: 560 };

/** Store app: discover and install apps. */
const STORE_WINDOW_SIZE: WindowSize = { width: 720, height: 520 };

/** Minesweeper app: classic grid game. */
const MINESWEEPER_WINDOW_SIZE: WindowSize = { width: 400, height: 480 };

/** Packet app: wide for columns. */
const PACKET_WINDOW_SIZE: WindowSize = { width: 800, height: 500 };

/** Codelab app: needs space for split editor layout. */
const CODELAB_WINDOW_SIZE: WindowSize = { width: 960, height: 640 };

export function getDefaultSizeForType(type: WindowType): WindowSize {
  if (type === "browser" || type === "editor") return LARGE_WINDOW_SIZE;
  if (type === "pixelart") return PIXELART_PICKER_SIZE;
  if (type === "sound") return SOUND_WINDOW_SIZE;
  if (type === "network") return NETWORK_WINDOW_SIZE;
  if (type === "email") return EMAIL_WINDOW_SIZE;
  if (type === "wallet") return WALLET_WINDOW_SIZE;
  if (type === "sysinfo") return SYSINFO_WINDOW_SIZE;
  if (type === "shortcuts") return SHORTCUTS_WINDOW_SIZE;
  if (type === "sysmon") return SYSMON_WINDOW_SIZE;
  if (type === "nullcloud") return NULLCLOUD_WINDOW_SIZE;
  if (type === "hackerboard") return HACKERBOARD_WINDOW_SIZE;
  if (type === "startup") return STARTUP_WINDOW_SIZE;
  if (type === "wallpaper") return WALLPAPER_WINDOW_SIZE;
  if (type === "settings") return SETTINGS_WINDOW_SIZE;
  if (type === "traceroute") return TRACEROUTE_WINDOW_SIZE;
  if (type === "store") return STORE_WINDOW_SIZE;
  if (type === "minesweeper") return MINESWEEPER_WINDOW_SIZE;
  if (type === "packet") return PACKET_WINDOW_SIZE;
  if (type === "codelab") return CODELAB_WINDOW_SIZE;
  return DEFAULT_SIZE;
}
const MIN_WIDTH = 320;
const MIN_HEIGHT = 200;

function reducer(state: WindowManagerState, action: WindowManagerAction): WindowManagerState {
  switch (action.type) {
    case "open": {
      const { window: win } = action.payload;
      return {
        ...state,
        windows: [...state.windows, win],
        focusedId: win.id,
        nextZIndex: state.nextZIndex + 1,
      };
    }
    case "close": {
      const { id } = action.payload;
      return {
        ...state,
        windows: state.windows.filter((w) => w.id !== id),
        focusedId: state.focusedId === id ? null : state.focusedId,
      };
    }
    case "minimize": {
      const { id } = action.payload;
      return {
        ...state,
        windows: state.windows.map((w) =>
          w.id === id ? { ...w, minimized: true } : w
        ),
      };
    }
    case "maximize": {
      const { id } = action.payload;
      return {
        ...state,
        windows: state.windows.map((w) =>
          w.id === id ? { ...w, maximized: !w.maximized } : w
        ),
      };
    }
    case "setFocus": {
      const { id } = action.payload;
      const nextZ = state.nextZIndex + 1;
      return {
        ...state,
        focusedId: id,
        nextZIndex: nextZ,
        windows: state.windows.map((w) =>
          w.id === id ? { ...w, zIndex: nextZ, minimized: false } : w
        ),
      };
    }
    case "move": {
      const { id, x, y } = action.payload;
      return {
        ...state,
        windows: state.windows.map((w) =>
          w.id === id ? { ...w, position: { x, y } } : w
        ),
      };
    }
    case "resize": {
      const { id, width, height } = action.payload;
      const w = Math.max(MIN_WIDTH, width);
      const h = Math.max(MIN_HEIGHT, height);
      return {
        ...state,
        windows: state.windows.map((win) =>
          win.id === id ? { ...win, size: { width: w, height: h } } : win
        ),
      };
    }
    case "setWindowWorkspace": {
      const { id, workspaceId } = action.payload;
      return {
        ...state,
        windows: state.windows.map((w) =>
          w.id === id ? { ...w, workspaceId } : w
        ),
      };
    }
    case "setWindowGridSlot": {
      const { id, gridSlot } = action.payload;
      return {
        ...state,
        windows: state.windows.map((w) =>
          w.id === id ? { ...w, gridSlot } : w
        ),
      };
    }
    default:
      return state;
  }
}

interface WindowManagerValue {
  windows: WindowState[];
  focusedId: string | null;
  open: (type: WindowType, options?: OpenWindowOptions) => string;
  close: (id: string) => void;
  minimize: (id: string) => void;
  maximize: (id: string) => void;
  setFocus: (id: string) => void;
  move: (id: string, x: number, y: number) => void;
  resize: (id: string, width: number, height: number) => void;
  setWindowWorkspace: (id: string, workspaceId: string) => void;
  setWindowGridSlot: (id: string, gridSlot: GridSlot | undefined) => void;
  getWindowIdsByType: (type: WindowType) => string[];
}

const WindowManagerContext = createContext<WindowManagerValue | null>(null);

let nextId = 1;

export function WindowManagerProvider({ children }: { children: React.ReactNode }) {
  const [state, dispatch] = useReducer(reducer, {
    windows: [],
    focusedId: null,
    nextZIndex: 1,
  });

  const open = useCallback(
    (type: WindowType, options?: OpenWindowOptions): string => {
      const id = `win-${nextId++}`;
      const size = options?.size ?? getDefaultSizeForType(type);
      const position =
        options?.position ??
        (typeof window !== "undefined"
          ? (() => {
            const dockBottom = 6;
            const dockHeight = 56;
            const safeBottom = dockBottom + dockHeight;
            const availableHeight = window.innerHeight - safeBottom;
            const centerX = (window.innerWidth - size.width) / 2;
            const centerY = (availableHeight - size.height) / 2;
            const cascadeOffset = 28;
            const n = state.windows.length;
            const y = centerY + n * cascadeOffset;
            const yClamped = Math.max(0, Math.min(y, availableHeight - size.height));
            return {
              x: Math.max(0, centerX),
              y: yClamped,
            };
          })()
          : { x: 60, y: 60 });
      const defaultTitles: Record<WindowType, string> = {
        terminal: "Terminal",
        explorer: "Files",
        browser: "Browser",
        apps: "All Apps",
        editor: "Code",
        theme: "Theme",
        sound: "Sound",
        network: "Network",
        email: "Mail",
        wallet: "Wallet",
        pixelart: "Pixel Art",
        sysinfo: "Nullfetch",
        shortcuts: "Shortcuts",
        sysmon: "System Monitor",
        nullcloud: "NullCloud",
        hackerboard: "Hackerboard",
        startup: "Startup",
        wallpaper: "Background",
        settings: "Settings",
        traceroute: "TraceRoute",
        store: "Store",
        minesweeper: "Minesweeper",
        packet: "Packet",
        codelab: "Codelab",
      };
      const title = options?.title ?? defaultTitles[type];
      const workspaceId = options?.workspaceId ?? "";
      const gridSlot = options?.gridSlot;
      dispatch({
        type: "open",
        payload: {
          window: {
            id,
            type,
            title,
            position,
            size,
            minimized: false,
            maximized: false,
            zIndex: state.nextZIndex,
            workspaceId,
            gridSlot,
          },
        },
      });
      return id;
    },
    [state.windows.length, state.nextZIndex]
  );

  const close = useCallback((id: string) => {
    dispatch({ type: "close", payload: { id } });
  }, []);

  const minimize = useCallback((id: string) => {
    dispatch({ type: "minimize", payload: { id } });
  }, []);

  const maximize = useCallback((id: string) => {
    dispatch({ type: "maximize", payload: { id } });
  }, []);

  const setFocus = useCallback((id: string) => {
    dispatch({ type: "setFocus", payload: { id } });
  }, []);

  const move = useCallback((id: string, x: number, y: number) => {
    dispatch({ type: "move", payload: { id, x, y } });
  }, []);

  const resize = useCallback((id: string, width: number, height: number) => {
    dispatch({
      type: "resize",
      payload: {
        id,
        width: Math.max(MIN_WIDTH, width),
        height: Math.max(MIN_HEIGHT, height),
      },
    });
  }, []);

  const setWindowWorkspace = useCallback((id: string, workspaceId: string) => {
    dispatch({ type: "setWindowWorkspace", payload: { id, workspaceId } });
  }, []);

  const setWindowGridSlot = useCallback((id: string, gridSlot: GridSlot | undefined) => {
    dispatch({ type: "setWindowGridSlot", payload: { id, gridSlot } });
  }, []);

  const getWindowIdsByType = useCallback(
    (type: WindowType) => state.windows.filter((w) => w.type === type).map((w) => w.id),
    [state.windows]
  );

  const value: WindowManagerValue = {
    windows: state.windows,
    focusedId: state.focusedId,
    open,
    close,
    minimize,
    maximize,
    setFocus,
    move,
    resize,
    setWindowWorkspace,
    setWindowGridSlot,
    getWindowIdsByType,
  };

  return (
    <WindowManagerContext.Provider value={value}>
      {children}
    </WindowManagerContext.Provider>
  );
}

export function useWindowManager(): WindowManagerValue {
  const ctx = useContext(WindowManagerContext);
  if (!ctx) throw new Error("useWindowManager must be used within WindowManagerProvider");
  return ctx;
}
