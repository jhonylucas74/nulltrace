import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Trash2, X } from "lucide-react";
import { useDevTools, type NetworkEntry, type StorageEntry } from "../contexts/DevToolsContext";
import { useAuth } from "../contexts/AuthContext";
import styles from "./DevTools.module.css";

type DevTab = "sources" | "network" | "storage" | "console";

function getOrigin(url: string): string {
  if (!url || url.startsWith("browser://")) return "";
  const u = url.replace(/^https?:\/\//, "");
  const idx = u.indexOf("/");
  return idx >= 0 ? u.slice(0, idx) : u;
}

function formatDuration(ms: number | null): string {
  if (ms === null) return "—";
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(2)}s`;
}

function formatSize(bytes: number | null): string {
  if (bytes === null) return "—";
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
}

function formatTime(ts: number): string {
  return new Date(ts).toLocaleTimeString("en-US", { hour12: false });
}

function statusClass(status: number | null): string {
  if (status === null) return "";
  if (status >= 500) return styles.statusError;
  if (status >= 400) return styles.statusWarn;
  if (status >= 200 && status < 300) return styles.statusOk;
  return "";
}

export default function DevTools({ tabId }: { tabId: string }) {
  const [activeTab, setActiveTab] = useState<DevTab>("network");
  const { playerId } = useAuth();
  const {
    networkByTab,
    consoleByTab,
    ntmlSourceMap,
    urlByTab,
    clearNetwork,
    clearConsole,
    pushConsole,
  } = useDevTools();

  const network = networkByTab[tabId] ?? [];
  const consoleLog = consoleByTab[tabId] ?? [];
  const ntmlSource = ntmlSourceMap[tabId] ?? null;
  const inspectedUrl = urlByTab[tabId] ?? "";

  const [selectedNetworkEntry, setSelectedNetworkEntry] = useState<NetworkEntry | null>(null);
  const consoleEndRef = useRef<HTMLDivElement | null>(null);

  // Storage state
  const origin = getOrigin(inspectedUrl);
  const [storageEntries, setStorageEntries] = useState<StorageEntry[]>([]);
  const [editingKey, setEditingKey] = useState<string | null>(null);
  const [editingValue, setEditingValue] = useState("");

  const loadStorage = useCallback(async () => {
    if (!origin) { setStorageEntries([]); return; }
    try {
      const entries = await invoke<StorageEntry[]>("browser_storage_get_all", { origin, userId: playerId ?? "" });
      setStorageEntries(entries);
    } catch {
      setStorageEntries([]);
    }
  }, [origin, playerId]);

  useEffect(() => {
    if (activeTab === "storage") loadStorage();
  }, [activeTab, origin, loadStorage]);

  // Re-load storage when inspected tab changes
  useEffect(() => {
    if (activeTab === "storage") loadStorage();
  }, [tabId]);

  // Auto-scroll console
  useEffect(() => {
    if (activeTab === "console") {
      consoleEndRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [consoleLog, activeTab]);

  const handleDeleteStorage = async (key: string) => {
    try {
      await invoke("browser_storage_delete", { origin, key, userId: playerId ?? "" });
      await loadStorage();
    } catch {}
  };

  const handleClearStorage = async () => {
    try {
      await invoke("browser_storage_clear", { origin, userId: playerId ?? "" });
      await loadStorage();
    } catch {}
  };

  const handleEditSave = async (key: string) => {
    try {
      await invoke("browser_storage_set", { origin, key, value: editingValue, userId: playerId ?? "" });
      setEditingKey(null);
      setEditingValue("");
      await loadStorage();
    } catch {}
  };

  const TABS: { id: DevTab; label: string }[] = [
    { id: "sources", label: "Source" },
    { id: "network", label: "Network" },
    { id: "storage", label: "Storage" },
    { id: "console", label: "Console" },
  ];

  return (
    <div className={styles.root}>
      {/* Tab bar */}
      <div className={styles.tabBar}>
        {TABS.map((t) => (
          <button
            key={t.id}
            type="button"
            className={`${styles.tabBtn} ${activeTab === t.id ? styles.tabBtnActive : ""}`}
            onClick={() => setActiveTab(t.id)}
          >
            {t.label}
            {t.id === "console" && consoleLog.length > 0 && (
              <span className={styles.badge}>{consoleLog.length}</span>
            )}
            {t.id === "network" && network.length > 0 && (
              <span className={styles.badge}>{network.length}</span>
            )}
          </button>
        ))}
        <div className={styles.tabBarSpacer} />
        {(activeTab === "network") && (
          <button type="button" className={styles.clearBtn} onClick={() => clearNetwork(tabId)} title="Clear network log">
            <Trash2 size={14} />
          </button>
        )}
        {activeTab === "console" && (
          <button type="button" className={styles.clearBtn} onClick={() => clearConsole(tabId)} title="Clear console">
            <Trash2 size={14} />
          </button>
        )}
        {activeTab === "storage" && origin && (
          <button type="button" className={styles.clearBtn} onClick={handleClearStorage} title="Clear all storage">
            <Trash2 size={14} />
          </button>
        )}
      </div>

      {/* Content */}
      <div className={styles.content}>
        {activeTab === "sources" && <SourcesPanel source={ntmlSource} />}
        {activeTab === "network" && (
          <NetworkPanel
            entries={network}
            selected={selectedNetworkEntry}
            onSelect={setSelectedNetworkEntry}
          />
        )}
        {activeTab === "storage" && (
          <StoragePanel
            origin={origin}
            entries={storageEntries}
            editingKey={editingKey}
            editingValue={editingValue}
            onEdit={(key, val) => { setEditingKey(key); setEditingValue(val); }}
            onEditChange={setEditingValue}
            onEditSave={handleEditSave}
            onEditCancel={() => { setEditingKey(null); setEditingValue(""); }}
            onDelete={handleDeleteStorage}
          />
        )}
        {activeTab === "console" && (
          <ConsolePanel
            entries={consoleLog}
            endRef={consoleEndRef}
            tabId={tabId}
            pushConsole={pushConsole}
          />
        )}
      </div>
    </div>
  );
}

// ─── Source ──────────────────────────────────────────────────────────────────

function SourcesPanel({ source }: { source: string | null }) {
  if (!source) {
    return (
      <div className={styles.emptyState}>
        No NTML source available for this page.
      </div>
    );
  }
  const lines = source.split("\n");
  const lineCount = lines.length;
  const numWidth = String(lineCount).length;
  return (
    <div className={styles.sourcesWrap}>
      <pre className={styles.sourceCode}>
        {lines.map((line, i) => (
          <span key={i} className={styles.sourceLine}>
            <span className={styles.sourceLineNum} style={{ minWidth: `${numWidth + 1}ch` }}>
              {i + 1}
            </span>
            <span className={styles.sourceLineBody}>{highlightNtmlLine(line)}</span>
            {"\n"}
          </span>
        ))}
      </pre>
    </div>
  );
}

/** Classifies a plain YAML value string into a CSS class. */
function valueClass(raw: string): string | null {
  const v = raw.trim();
  if (!v) return null;
  if (v === "true" || v === "false" || v === "null" || v === "~") return styles.yamlBool;
  if (v === "|" || v === ">" || v === "|-" || v === ">-" || v === "|+" || v === ">+") return styles.yamlBlock;
  if (/^-?(?:0x[\da-fA-F]+|\d+\.?\d*(?:[eE][+-]?\d+)?)$/.test(v)) return styles.yamlNumber;
  if ((v.startsWith('"') && v.endsWith('"')) || (v.startsWith("'") && v.endsWith("'"))) return styles.yamlString;
  if (v.startsWith("*") || v.startsWith("&")) return styles.yamlAnchor;
  return null;
}

/** Renders a value string (after the colon or after "- ") with token coloring. */
function renderValue(raw: string): React.ReactNode {
  if (!raw) return null;
  // Split leading space from actual content
  const match = raw.match(/^(\s*)(.*?)(\s*)$/s);
  if (!match) return raw;
  const [, pre, content, post] = match;
  const cls = valueClass(content);
  if (cls) return <>{pre}<span className={cls}>{content}</span>{post}</>;
  return raw;
}

/** Tokenizes one line of NTML/YAML into highlighted React nodes. */
function highlightNtmlLine(line: string): React.ReactNode {
  // Empty line
  if (!line.trim()) return line;

  const trimmed = line.trimStart();
  const indentLen = line.length - trimmed.length;
  const indent = line.slice(0, indentLen);

  // Comment
  if (trimmed.startsWith("#")) {
    return <span className={styles.yamlComment}>{line}</span>;
  }

  // List item: "- " or bare "-"
  const listMatch = trimmed.match(/^(-\s+)(.*)/);
  if (listMatch) {
    const [, dash, rest] = listMatch;
    // List item may itself be a key: value mapping
    const innerColon = rest.indexOf(":");
    if (innerColon > 0) {
      const key = rest.slice(0, innerColon);
      const afterColon = rest.slice(innerColon + 1);
      return (
        <>
          {indent}
          <span className={styles.yamlListMarker}>{dash}</span>
          <span className={styles.yamlKey}>{key}</span>
          <span className={styles.yamlColon}>:</span>
          {renderValue(afterColon)}
        </>
      );
    }
    return (
      <>
        {indent}
        <span className={styles.yamlListMarker}>{dash}</span>
        {renderValue(rest)}
      </>
    );
  }

  // Bare list marker with no value
  if (trimmed === "-") {
    return <>{indent}<span className={styles.yamlListMarker}>-</span></>;
  }

  // Key: value  (colon must be followed by space or end-of-line)
  const colonMatch = trimmed.match(/^([^:#\s][^:]*?):\s*(.*)/s);
  if (colonMatch) {
    const [, key, rest] = colonMatch;
    return (
      <>
        {indent}
        <span className={styles.yamlKey}>{key}</span>
        <span className={styles.yamlColon}>:</span>
        {rest ? <>{" "}{renderValue(rest)}</> : null}
      </>
    );
  }

  return line;
}

// ─── Network ─────────────────────────────────────────────────────────────────

function NetworkPanel({
  entries,
  selected,
  onSelect,
}: {
  entries: NetworkEntry[];
  selected: NetworkEntry | null;
  onSelect: (e: NetworkEntry | null) => void;
}) {
  if (entries.length === 0) {
    return <div className={styles.emptyState}>No network requests recorded yet.</div>;
  }

  return (
    <div className={styles.networkWrap}>
      <div className={styles.networkList}>
        <div className={styles.networkHeader}>
          <span className={styles.netColOrigin}>Origin</span>
          <span className={styles.netColMethod}>Method</span>
          <span className={styles.netColStatus}>Status</span>
          <span className={styles.netColUrl}>URL</span>
          <span className={styles.netColType}>Type</span>
          <span className={styles.netColSize}>Size</span>
          <span className={styles.netColTime}>Time</span>
        </div>
        {entries.map((e) => (
          <button
            key={e.id}
            type="button"
            className={`${styles.networkRow} ${selected?.id === e.id ? styles.networkRowSelected : ""}`}
            onClick={() => onSelect(selected?.id === e.id ? null : e)}
          >
            <span className={`${styles.netColOrigin} ${e.origin === "lua" ? styles.originLua : styles.originBrowser}`}>
              {e.origin}
            </span>
            <span className={styles.netColMethod}>{e.method}</span>
            <span className={`${styles.netColStatus} ${statusClass(e.status)}`}>
              {e.status ?? "—"}
            </span>
            <span className={styles.netColUrl} title={e.url}>{e.url}</span>
            <span className={styles.netColType}>{e.contentType ?? "—"}</span>
            <span className={styles.netColSize}>{formatSize(e.size)}</span>
            <span className={styles.netColTime}>{formatDuration(e.duration)}</span>
          </button>
        ))}
      </div>
      {selected && (
        <div className={styles.networkDetail}>
          <div className={styles.networkDetailHeader}>
            <span>{selected.method} {selected.url}</span>
            <button type="button" className={styles.detailClose} onClick={() => onSelect(null)}>
              <X size={14} />
            </button>
          </div>
          <div className={styles.networkDetailMeta}>
            <span>Status: <strong className={statusClass(selected.status)}>{selected.status ?? "—"}</strong></span>
            <span>Duration: <strong>{formatDuration(selected.duration)}</strong></span>
            <span>Size: <strong>{formatSize(selected.size)}</strong></span>
            <span>Time: <strong>{formatTime(selected.timestamp)}</strong></span>
            <span>Content-Type: <strong>{selected.contentType ?? "—"}</strong></span>
          </div>
          {selected.requestHeaders && (
            <div className={styles.networkDetailSection}>
              <div className={styles.networkDetailLabel}>Request headers</div>
              <pre className={styles.networkDetailBody}>{selected.requestHeaders}</pre>
            </div>
          )}
          {selected.requestBody && (
            <div className={styles.networkDetailSection}>
              <div className={styles.networkDetailLabel}>Request body</div>
              <pre className={styles.networkDetailBody}>{selected.requestBody.slice(0, 4000)}{selected.requestBody.length > 4000 ? "\n…" : ""}</pre>
            </div>
          )}
          {selected.responseHeaders && (
            <div className={styles.networkDetailSection}>
              <div className={styles.networkDetailLabel}>Response headers</div>
              <pre className={styles.networkDetailBody}>{selected.responseHeaders}</pre>
            </div>
          )}
          {selected.response && (
            <div className={styles.networkDetailSection}>
              <div className={styles.networkDetailLabel}>Response</div>
              <pre className={styles.networkDetailBody}>{selected.response.slice(0, 4000)}{selected.response.length > 4000 ? "\n…" : ""}</pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ─── Storage ─────────────────────────────────────────────────────────────────

function StoragePanel({
  origin,
  entries,
  editingKey,
  editingValue,
  onEdit,
  onEditChange,
  onEditSave,
  onEditCancel,
  onDelete,
}: {
  origin: string;
  entries: StorageEntry[];
  editingKey: string | null;
  editingValue: string;
  onEdit: (key: string, val: string) => void;
  onEditChange: (val: string) => void;
  onEditSave: (key: string) => void;
  onEditCancel: () => void;
  onDelete: (key: string) => void;
}) {
  if (!origin) {
    return <div className={styles.emptyState}>Storage is not available for this page.</div>;
  }
  return (
    <div className={styles.storageWrap}>
      <div className={styles.storageOriginLabel}>Origin: <strong>{origin}</strong></div>
      {entries.length === 0 ? (
        <div className={styles.emptyState}>No storage entries for this origin.</div>
      ) : (
        <table className={styles.storageTable}>
          <thead>
            <tr>
              <th className={styles.storageThKey}>Key</th>
              <th className={styles.storageThVal}>Value</th>
              <th className={styles.storageThAct} />
            </tr>
          </thead>
          <tbody>
            {entries.map((entry) => (
              <tr key={entry.key} className={styles.storageRow}>
                <td className={styles.storageTdKey}>{entry.key}</td>
                <td className={styles.storageTdVal}>
                  {editingKey === entry.key ? (
                    <div className={styles.storageEditWrap}>
                      <input
                        className={styles.storageEditInput}
                        value={editingValue}
                        onChange={(e) => onEditChange(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") onEditSave(entry.key);
                          if (e.key === "Escape") onEditCancel();
                        }}
                        autoFocus
                      />
                      <button type="button" className={styles.storageEditSave} onClick={() => onEditSave(entry.key)}>Save</button>
                      <button type="button" className={styles.storageEditCancel} onClick={onEditCancel}>Cancel</button>
                    </div>
                  ) : (
                    <span
                      className={styles.storageTdValText}
                      onClick={() => onEdit(entry.key, entry.value)}
                      title="Click to edit"
                    >
                      {entry.value}
                    </span>
                  )}
                </td>
                <td className={styles.storageTdAct}>
                  <button
                    type="button"
                    className={styles.storageDeleteBtn}
                    onClick={() => onDelete(entry.key)}
                    aria-label="Delete entry"
                  >
                    <X size={12} />
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

// ─── Console ─────────────────────────────────────────────────────────────────

import React from "react";

interface ConsolePanelProps {
  entries: import("../contexts/DevToolsContext").ConsoleEntry[];
  endRef: { current: HTMLDivElement | null };
  tabId: string;
  pushConsole: (messages: string[], tabId: string, level?: "log" | "error") => void;
}

function ConsolePanel({ entries, endRef, tabId, pushConsole }: ConsolePanelProps) {
  const [input, setInput] = useState("");
  const [cmdHistory, setCmdHistory] = useState<string[]>([]);
  const [historyIdx, setHistoryIdx] = useState(-1);
  const [running, setRunning] = useState(false);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const runCode = useCallback(async () => {
    const code = input.trim();
    if (!code || running) return;

    // Add to history
    setCmdHistory((prev) => [code, ...prev].slice(0, 100));
    setHistoryIdx(-1);
    setInput("");
    setRunning(true);

    try {
      const result = await invoke<{ output: string[]; error: string | null }>("ntml_eval_lua", {
        tabId,
        code,
      });

      // Console output streams via devtools:console; only push errors here
      if (result.error) {
        pushConsole([result.error], tabId, "error");
      }
    } catch (err) {
      pushConsole([String(err)], tabId, "error");
    } finally {
      setRunning(false);
      setTimeout(() => inputRef.current?.focus(), 0);
    }
  }, [input, running, tabId, pushConsole]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        runCode();
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        const nextIdx = Math.min(historyIdx + 1, cmdHistory.length - 1);
        setHistoryIdx(nextIdx);
        if (cmdHistory[nextIdx] !== undefined) setInput(cmdHistory[nextIdx]);
        return;
      }
      if (e.key === "ArrowDown") {
        e.preventDefault();
        const nextIdx = historyIdx - 1;
        if (nextIdx < 0) {
          setHistoryIdx(-1);
          setInput("");
        } else {
          setHistoryIdx(nextIdx);
          if (cmdHistory[nextIdx] !== undefined) setInput(cmdHistory[nextIdx]);
        }
        return;
      }
    },
    [runCode, historyIdx, cmdHistory]
  );

  // Auto-resize textarea
  const handleInputChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setInput(e.target.value);
    setHistoryIdx(-1);
    const el = e.target;
    el.style.height = "auto";
    el.style.height = `${Math.min(el.scrollHeight, 120)}px`;
  }, []);

  return (
    <div className={styles.consolePanelWrap}>
      {/* Log area */}
      <div className={styles.consoleWrap}>
        {entries.length === 0 && (
          <div className={styles.emptyState}>
            Use <code>print()</code> in Lua scripts or type code below.
          </div>
        )}
        {entries.map((e) => (
          <div key={e.id} className={`${styles.consoleLine} ${e.level === "error" ? styles.consoleError : ""}`}>
            <span className={styles.consoleTime}>{formatTime(e.timestamp)}</span>
            <span className={styles.consoleMsg}>{e.message}</span>
          </div>
        ))}
        <div ref={endRef} />
      </div>

      {/* REPL input */}
      <div className={styles.consoleRepl}>
        <div className={styles.consoleReplPrompt}>
          <span className={styles.consoleReplArrow}>{running ? "…" : ">"}</span>
          <textarea
            ref={inputRef}
            className={styles.consoleReplInput}
            value={input}
            onChange={handleInputChange}
            onKeyDown={handleKeyDown}
            placeholder="Enter Lua code… (Enter to run, Shift+Enter for newline)"
            rows={1}
            spellCheck={false}
            autoComplete="off"
            autoCorrect="off"
            autoCapitalize="off"
          />
        </div>
      </div>
    </div>
  );
}
