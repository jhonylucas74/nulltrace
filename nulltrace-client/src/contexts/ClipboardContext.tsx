import React, {
  createContext,
  useContext,
  useState,
  useCallback,
  useMemo,
} from "react";

export interface ClipboardItem {
  path: string;
  type: "file" | "folder";
}

export type ClipboardOperation = "copy" | "cut";

interface ClipboardState {
  items: ClipboardItem[];
  operation: ClipboardOperation;
}

interface ClipboardContextValue {
  items: ClipboardItem[];
  operation: ClipboardOperation;
  setClipboard: (items: ClipboardItem[], operation: ClipboardOperation) => void;
  getClipboard: () => ClipboardState;
  clearClipboard: () => void;
  hasItems: boolean;
}

const ClipboardContext = createContext<ClipboardContextValue | null>(null);

export function ClipboardProvider({ children }: { children: React.ReactNode }) {
  const [state, setState] = useState<ClipboardState>({
    items: [],
    operation: "copy",
  });

  const setClipboard = useCallback(
    (items: ClipboardItem[], operation: ClipboardOperation) => {
      setState({ items, operation });
    },
    []
  );

  const getClipboard = useCallback(() => state, [state]);

  const clearClipboard = useCallback(() => {
    setState({ items: [], operation: "copy" });
  }, []);

  const hasItems = state.items.length > 0;

  const value = useMemo<ClipboardContextValue>(
    () => ({
      items: state.items,
      operation: state.operation,
      setClipboard,
      getClipboard,
      clearClipboard,
      hasItems,
    }),
    [state, setClipboard, getClipboard, clearClipboard, hasItems]
  );

  return (
    <ClipboardContext.Provider value={value}>
      {children}
    </ClipboardContext.Provider>
  );
}

export function useClipboard(): ClipboardContextValue {
  const ctx = useContext(ClipboardContext);
  if (!ctx) throw new Error("useClipboard must be used within ClipboardProvider");
  return ctx;
}
