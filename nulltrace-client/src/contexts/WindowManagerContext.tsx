import React, { createContext, useContext, useReducer, useCallback } from "react";

export type WindowType = "terminal" | "explorer" | "browser" | "apps" | "editor" | "theme";

export interface WindowPosition {
  x: number;
  y: number;
}

export interface WindowSize {
  width: number;
  height: number;
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
}

export interface OpenWindowOptions {
  title?: string;
  position?: WindowPosition;
  size?: WindowSize;
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
  | { type: "resize"; payload: { id: string; width: number; height: number } };

const DEFAULT_SIZE: WindowSize = { width: 640, height: 400 };
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
      const position = options?.position ?? { x: 60 + (state.windows.length % 4) * 40, y: 60 + (state.windows.length % 3) * 32 };
      const size = options?.size ?? DEFAULT_SIZE;
      const defaultTitles: Record<WindowType, string> = {
        terminal: "Terminal",
        explorer: "Files",
        browser: "Browser",
        apps: "All Apps",
        editor: "Code",
        theme: "Theme",
      };
  const title = options?.title ?? defaultTitles[type];
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
