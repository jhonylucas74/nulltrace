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
  inspectedTabId: string;
  inspectedUrl: string;
  ntmlSource: string | null;
  network: NetworkEntry[];
  consoleLog: ConsoleEntry[];
  setInspectedTab: (tabId: string, url: string) => void;
  pushNetwork: (entry: Omit<NetworkEntry, "id">) => void;
  pushConsole: (messages: string[], level?: "log" | "error") => void;
  setSource: (tabId: string, yaml: string) => void;
  clearNetwork: () => void;
  clearConsole: () => void;
}

const DevToolsContext = createContext<DevToolsContextValue | null>(null);

let _entryCounter = 0;
function genId(prefix: string) {
  return `${prefix}-${Date.now()}-${(_entryCounter++).toString(36)}`;
}

export function DevToolsContextProvider({ children }: { children: React.ReactNode }) {
  const [inspectedTabId, setInspectedTabId] = useState("");
  const [inspectedUrl, setInspectedUrl] = useState("");
  const [ntmlSourceMap, setNtmlSourceMap] = useState<Record<string, string>>({});
  const [network, setNetwork] = useState<NetworkEntry[]>([]);
  const [consoleLog, setConsoleLog] = useState<ConsoleEntry[]>([]);

  const ntmlSource = ntmlSourceMap[inspectedTabId] ?? null;

  const setInspectedTab = useCallback((tabId: string, url: string) => {
    setInspectedTabId(tabId);
    setInspectedUrl(url);
  }, []);

  const pushNetwork = useCallback((entry: Omit<NetworkEntry, "id">) => {
    setNetwork((prev) => [...prev, { ...entry, id: genId("net") }]);
  }, []);

  const pushConsole = useCallback((messages: string[], level: "log" | "error" = "log") => {
    const ts = Date.now();
    const entries: ConsoleEntry[] = messages.map((message) => ({
      id: genId("con"),
      message,
      level,
      timestamp: ts,
    }));
    setConsoleLog((prev) => [...prev, ...entries]);
  }, []);

  const setSource = useCallback((tabId: string, yaml: string) => {
    setNtmlSourceMap((prev) => ({ ...prev, [tabId]: yaml }));
  }, []);

  const clearNetwork = useCallback(() => setNetwork([]), []);
  const clearConsole = useCallback(() => setConsoleLog([]), []);

  return (
    <DevToolsContext.Provider
      value={{
        inspectedTabId,
        inspectedUrl,
        ntmlSource,
        network,
        consoleLog,
        setInspectedTab,
        pushNetwork,
        pushConsole,
        setSource,
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
