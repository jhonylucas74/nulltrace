import React, { createContext, useContext, useState, useCallback } from "react";

export interface NetworkEntry {
  id: string;
  origin: "browser" | "lua";
  url: string;
  method: string;
  status: number | null;
  duration: number | null;
  contentType: string | null;
  size: number | null;
  response: string | null;
  timestamp: number;
}

export interface ConsoleEntry {
  id: string;
  message: string;
  level: "log" | "error";
  timestamp: number;
}

export interface StorageEntry {
  key: string;
  value: string;
}

interface DevToolsContextValue {
  networkByTab: Record<string, NetworkEntry[]>;
  consoleByTab: Record<string, ConsoleEntry[]>;
  ntmlSourceMap: Record<string, string>;
  urlByTab: Record<string, string>;
  pushNetwork: (entry: Omit<NetworkEntry, "id">, tabId: string) => void;
  pushConsole: (messages: string[], tabId: string, level?: "log" | "error") => void;
  setSource: (tabId: string, yaml: string) => void;
  setTabUrl: (tabId: string, url: string) => void;
  removeTabData: (tabId: string) => void;
  clearNetwork: (tabId: string) => void;
  clearConsole: (tabId: string) => void;
}

const DevToolsContext = createContext<DevToolsContextValue | null>(null);

let _entryCounter = 0;
function genId(prefix: string) {
  return `${prefix}-${Date.now()}-${(_entryCounter++).toString(36)}`;
}

export function DevToolsContextProvider({ children }: { children: React.ReactNode }) {
  const [networkByTab, setNetworkByTab] = useState<Record<string, NetworkEntry[]>>({});
  const [consoleByTab, setConsoleByTab] = useState<Record<string, ConsoleEntry[]>>({});
  const [ntmlSourceMap, setNtmlSourceMap] = useState<Record<string, string>>({});
  const [urlByTab, setUrlByTabState] = useState<Record<string, string>>({});

  const pushNetwork = useCallback((entry: Omit<NetworkEntry, "id">, tabId: string) => {
    setNetworkByTab((prev) => ({
      ...prev,
      [tabId]: [...(prev[tabId] ?? []), { ...entry, id: genId("net") }],
    }));
  }, []);

  const pushConsole = useCallback((messages: string[], tabId: string, level: "log" | "error" = "log") => {
    const ts = Date.now();
    const entries: ConsoleEntry[] = messages.map((message) => ({
      id: genId("con"),
      message,
      level,
      timestamp: ts,
    }));
    setConsoleByTab((prev) => ({
      ...prev,
      [tabId]: [...(prev[tabId] ?? []), ...entries],
    }));
  }, []);

  const setSource = useCallback((tabId: string, yaml: string) => {
    setNtmlSourceMap((prev) => ({ ...prev, [tabId]: yaml }));
  }, []);

  const setTabUrl = useCallback((tabId: string, url: string) => {
    setUrlByTabState((prev) => ({ ...prev, [tabId]: url }));
  }, []);

  const removeTabData = useCallback((tabId: string) => {
    setNetworkByTab((prev) => { const n = { ...prev }; delete n[tabId]; return n; });
    setConsoleByTab((prev) => { const n = { ...prev }; delete n[tabId]; return n; });
    setNtmlSourceMap((prev) => { const n = { ...prev }; delete n[tabId]; return n; });
    setUrlByTabState((prev) => { const n = { ...prev }; delete n[tabId]; return n; });
  }, []);

  const clearNetwork = useCallback((tabId: string) => {
    setNetworkByTab((prev) => ({ ...prev, [tabId]: [] }));
  }, []);

  const clearConsole = useCallback((tabId: string) => {
    setConsoleByTab((prev) => ({ ...prev, [tabId]: [] }));
  }, []);

  return (
    <DevToolsContext.Provider
      value={{
        networkByTab,
        consoleByTab,
        ntmlSourceMap,
        urlByTab,
        pushNetwork,
        pushConsole,
        setSource,
        setTabUrl,
        removeTabData,
        clearNetwork,
        clearConsole,
      }}
    >
      {children}
    </DevToolsContext.Provider>
  );
}

export function useDevTools(): DevToolsContextValue {
  const ctx = useContext(DevToolsContext);
  if (!ctx) throw new Error("useDevTools must be used within DevToolsContextProvider");
  return ctx;
}
