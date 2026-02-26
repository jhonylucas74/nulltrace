import React, { createContext, useContext, useState, useCallback, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";

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
  requestBody?: string | null;
  requestHeaders?: string | null;
  responseHeaders?: string | null;
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

  // Real-time console: listen for Lua print() events from backend.
  // Cancelled flag + unlisten on setup completion prevents duplicate listeners (React Strict Mode).
  useEffect(() => {
    const tauri = (window as unknown as { __TAURI__?: unknown }).__TAURI__;
    if (!tauri) return;
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    const setup = async () => {
      const fn = await listen<{ tab_id: string; message: string; level?: "log" | "error" }>("devtools:console", (event) => {
        if (cancelled) return;
        const { tab_id, message, level } = event.payload;
        const entryLevel = level === "error" ? ("error" as const) : ("log" as const);
        setConsoleByTab((prev) => ({
          ...prev,
          [tab_id]: [
            ...(prev[tab_id] ?? []),
            { id: genId("con"), message, level: entryLevel, timestamp: Date.now() },
          ],
        }));
      });
      if (cancelled) {
        fn();
        return;
      }
      unlisten = fn;
    };
    setup();
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  // Real-time network: listen for Lua http.get/post events from backend.
  // Cancelled flag + unlisten on setup completion prevents duplicate listeners (React Strict Mode).
  useEffect(() => {
    const tauri = (window as unknown as { __TAURI__?: unknown }).__TAURI__;
    if (!tauri) return;
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    const setup = async () => {
      const fn = await listen<{
        tab_id: string;
        entry: {
          origin: string;
          url: string;
          method: string;
          status?: number | null;
          duration_ms?: number | null;
          content_type?: string | null;
          size?: number | null;
          response?: string | null;
          request_body?: string | null;
          request_headers?: string | null;
          response_headers?: string | null;
          timestamp: number;
        };
      }>("devtools:network", (event) => {
        if (cancelled) return;
        const { tab_id, entry } = event.payload;
        setNetworkByTab((prev) => ({
          ...prev,
          [tab_id]: [
            ...(prev[tab_id] ?? []),
            {
              id: genId("net"),
              origin: (entry.origin as "browser" | "lua") || "lua",
              url: entry.url,
              method: entry.method,
              status: entry.status ?? null,
              duration: entry.duration_ms ?? null,
              contentType: entry.content_type ?? null,
              size: entry.size ?? null,
              response: entry.response ?? null,
              requestBody: entry.request_body ?? null,
              requestHeaders: entry.request_headers ?? null,
              responseHeaders: entry.response_headers ?? null,
              timestamp: entry.timestamp,
            },
          ],
        }));
      });
      if (cancelled) {
        fn();
        return;
      }
      unlisten = fn;
    };
    setup();
    return () => {
      cancelled = true;
      unlisten?.();
    };
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
