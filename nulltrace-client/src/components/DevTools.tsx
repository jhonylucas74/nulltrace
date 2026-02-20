import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Trash2, X } from "lucide-react";
import { useDevTools, type NetworkEntry, type StorageEntry } from "../contexts/DevToolsContext";
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

export default function DevTools() {
  const [activeTab, setActiveTab] = useState<DevTab>("network");
  const { inspectedTabId, inspectedUrl, ntmlSource, network, consoleLog, clearNetwork, clearConsole } = useDevTools();
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
      const entries = await invoke<StorageEntry[]>("browser_storage_get_all", { origin });
      setStorageEntries(entries);
    } catch {
      setStorageEntries([]);
    }
  }, [origin]);

  useEffect(() => {
    if (activeTab === "storage") loadStorage();
  }, [activeTab, origin, loadStorage]);

  // Re-load storage when inspected tab changes
  useEffect(() => {
    if (activeTab === "storage") loadStorage();
  }, [inspectedTabId]);

  // Auto-scroll console
  useEffect(() => {
    if (activeTab === "console") {
      consoleEndRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [consoleLog, activeTab]);

  const handleDeleteStorage = async (key: string) => {
    try {
      await invoke("browser_storage_delete", { origin, key });
      await loadStorage();
    } catch {}
  };

  const handleClearStorage = async () => {
    try {
      await invoke("browser_storage_clear", { origin });
      await loadStorage();
    } catch {}
  };

  const handleEditSave = async (key: string) => {
    try {
      await invoke("browser_storage_set", { origin, key, value: editingValue });
      setEditingKey(null);
      setEditingValue("");
      await loadStorage();
    } catch {}
  };

  const TABS: { id: DevTab; label: string }[] = [
    { id: "sources", label: "Sources" },
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
          <button type="button" className={styles.clearBtn} onClick={clearNetwork} title="Clear network log">
            <Trash2 size={14} />
          </button>
        )}
        {activeTab === "console" && (
          <button type="button" className={styles.clearBtn} onClick={clearConsole} title="Clear console">
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
          <ConsolePanel entries={consoleLog} endRef={consoleEndRef} />
        )}
      </div>
    </div>
  );
}

// ─── Sources ─────────────────────────────────────────────────────────────────

function SourcesPanel({ source }: { source: string | null }) {
  if (!source) {
    return (
      <div className={styles.emptyState}>
        No NTML source available for this page.
      </div>
    );
  }
  return (
    <div className={styles.sourcesWrap}>
      <pre className={styles.sourceCode}>{highlightNtml(source)}</pre>
    </div>
  );
}

function highlightNtml(yaml: string): React.ReactNode {
  const lines = yaml.split("\n");
  return lines.map((line, i) => {
    const colonIdx = line.indexOf(":");
    if (colonIdx > 0 && !line.trimStart().startsWith("#")) {
      const key = line.slice(0, colonIdx + 1);
      const rest = line.slice(colonIdx + 1);
      return (
        <span key={i}>
          <span className={styles.yamlKey}>{key}</span>
          <span className={styles.yamlValue}>{rest}</span>
          {"\n"}
        </span>
      );
    }
    if (line.trimStart().startsWith("#")) {
      return <span key={i} className={styles.yamlComment}>{line}{"\n"}</span>;
    }
    return <span key={i}>{line}{"\n"}</span>;
  });
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
          {selected.response && (
            <pre className={styles.networkDetailBody}>{selected.response.slice(0, 4000)}{selected.response.length > 4000 ? "\n…" : ""}</pre>
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

function ConsolePanel({
  entries,
  endRef,
}: {
  entries: import("../contexts/DevToolsContext").ConsoleEntry[];
  endRef: { current: HTMLDivElement | null };
}) {
  if (entries.length === 0) {
    return <div className={styles.emptyState}>No console output yet. Use <code>print()</code> in your Lua scripts.</div>;
  }
  return (
    <div className={styles.consoleWrap}>
      {entries.map((e) => (
        <div key={e.id} className={`${styles.consoleLine} ${e.level === "error" ? styles.consoleError : ""}`}>
          <span className={styles.consoleTime}>{formatTime(e.timestamp)}</span>
          <span className={styles.consoleMsg}>{e.message}</span>
        </div>
      ))}
      <div ref={endRef} />
    </div>
  );
}
