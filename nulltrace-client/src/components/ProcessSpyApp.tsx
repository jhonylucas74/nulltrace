import { useState, useEffect, useRef, useCallback } from "react";
import { List, Send, X } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useAuth } from "../contexts/AuthContext";
import styles from "./ProcessSpyApp.module.css";

interface ProcessEntry {
  pid: number;
  name: string;
  username: string;
  status: string;
  memory_bytes: number;
  /** Full argv used to invoke the process (always shown). */
  args?: string[];
}

interface TabState {
  pid: number;
  name: string;
  stdinBuffer: string;
  stdoutBuffer: string;
  gone?: boolean;
}

export default function ProcessSpyApp() {
  const { token } = useAuth();
  const [connectionId, setConnectionId] = useState<string | null>(null);
  const [processes, setProcesses] = useState<ProcessEntry[]>([]);
  const [tabs, setTabs] = useState<TabState[]>([]);
  const [activePid, setActivePid] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [stdinInput, setStdinInput] = useState("");
  const unlistenRef = useRef<(() => void)[]>([]);

  // Disconnect on unmount
  useEffect(() => {
    return () => {
      if (connectionId) {
        invoke("process_spy_disconnect", { connectionId }).catch(() => {});
      }
    };
  }, [connectionId]);

  // Register listeners first, then connect, so we never miss the first process list from the server
  useEffect(() => {
    if (!token) {
      setError("Log in to use Proc Spy.");
      return;
    }
    setError(null);
    const unlistens: (() => void)[] = [];
    let cancelled = false;

    (async () => {
      // Register all listeners before connecting so the first ProcessListSnapshot is not missed
      const u1 = await listen<{ processes: ProcessEntry[] }>(
        "process-spy-process-list",
        (event) => {
          if (!cancelled) setProcesses(event.payload?.processes ?? []);
        }
      );
      unlistens.push(u1);

      const u2 = await listen<{ pid: number; data: string }>(
        "process-spy-stdout",
        (event) => {
          const { pid, data } = event.payload ?? { pid: 0, data: "" };
          if (!cancelled) {
            setTabs((prev) =>
              prev.map((t) =>
                t.pid === pid ? { ...t, stdoutBuffer: t.stdoutBuffer + data } : t
              )
            );
          }
        }
      );
      unlistens.push(u2);

      const u3 = await listen<{ pid: number; data: string }>(
        "process-spy-stdin",
        (event) => {
          const { pid, data } = event.payload ?? { pid: 0, data: "" };
          if (!cancelled) {
            setTabs((prev) =>
              prev.map((t) =>
                t.pid === pid ? { ...t, stdinBuffer: t.stdinBuffer + data } : t
              )
            );
          }
        }
      );
      unlistens.push(u3);

      const u4 = await listen<{ pid: number }>("process-spy-process-gone", (event) => {
        const pid = event.payload?.pid ?? 0;
        if (!cancelled) {
          setTabs((prev) =>
            prev.map((t) => (t.pid === pid ? { ...t, gone: true } : t))
          );
        }
      });
      unlistens.push(u4);

      const u5 = await listen<{ message: string }>("process-spy-error", (event) => {
        if (!cancelled) setError(event.payload?.message ?? "Process Spy error.");
      });
      unlistens.push(u5);

      const u6 = await listen<{ connectionId?: string }>("process-spy-closed", (event) => {
        const closedId = event.payload?.connectionId;
        if (!cancelled && closedId !== undefined) {
          setConnectionId((current) => {
            if (current === closedId) {
              setError("Disconnected.");
              return null;
            }
            return current;
          });
        }
      });
      unlistens.push(u6);

      unlistenRef.current = unlistens;

      if (cancelled) return;
      try {
        const id = await invoke<string>("process_spy_connect", { token });
        if (!cancelled) setConnectionId(id);
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    })();

    return () => {
      cancelled = true;
      unlistens.forEach((u) => u());
    };
  }, [token]);

  /** Display label for a process: full command line (args) when available, else name. */
  const processLabel = (proc: ProcessEntry) =>
    proc.args && proc.args.length > 0 ? proc.args.join(" ") : proc.name;

  const openTab = useCallback(
    (pid: number, name: string) => {
      if (!connectionId) return;
      setTabs((prev) => {
        if (prev.some((t) => t.pid === pid)) return prev;
        invoke("process_spy_subscribe", { connectionId, pid }).catch((e) =>
          setError(String(e))
        );
        return [...prev, { pid, name, stdinBuffer: "", stdoutBuffer: "" }];
      });
      setActivePid(pid);
    },
    [connectionId]
  );

  const closeTab = useCallback(
    (pid: number) => {
      if (connectionId) {
        invoke("process_spy_unsubscribe", { connectionId, pid }).catch(() => {});
      }
      setTabs((prev) => prev.filter((t) => t.pid !== pid));
    },
    [connectionId]
  );

  // When tabs change, if active tab was closed, switch to first tab or none
  useEffect(() => {
    if (activePid != null && !tabs.some((t) => t.pid === activePid)) {
      setActivePid(tabs.length > 0 ? tabs[0].pid : null);
    }
  }, [tabs, activePid]);

  const sendStdin = useCallback(
    (pid: number) => {
      if (!connectionId || !stdinInput.trim()) return;
      invoke("process_spy_stdin", {
        connectionId,
        pid,
        data: stdinInput.endsWith("\n") ? stdinInput : stdinInput + "\n",
      }).catch((e) => setError(String(e)));
      setStdinInput("");
    },
    [connectionId, stdinInput]
  );

  const activeTab = tabs.find((t) => t.pid === activePid);

  return (
    <div className={styles.appWithSidebar}>
      <aside className={styles.sidebar}>
        <div className={styles.sidebarSection}>
          <List size={14} style={{ verticalAlign: "middle", marginRight: 4 }} />
          Processes
        </div>
        <ul className={styles.processList}>
          {processes.map((proc) => (
            <li
              key={proc.pid}
              className={`${styles.processRow} ${
                activePid === proc.pid ? styles.processRowActive : ""
              }`}
              onClick={() => openTab(proc.pid, processLabel(proc))}
            >
              <span className={styles.processName} title={processLabel(proc)}>
                {processLabel(proc)}
              </span>
              <span className={styles.processPid}>{proc.pid}</span>
            </li>
          ))}
        </ul>
      </aside>
      <main className={styles.main}>
        {error && <div className={styles.errorBanner}>{error}</div>}
        <h2 className={styles.mainTitle}>Proc Spy</h2>
        <p className={styles.mainSubtitle}>
          {connectionId
            ? "Click a process to open a tab and view/inject stdin and stdout."
            : "Connecting…"}
        </p>
        {tabs.length > 0 && (
          <div className={styles.tabBar}>
            {tabs.map((t) => (
              <button
                key={t.pid}
                type="button"
                className={`${styles.tab} ${
                  activePid === t.pid ? styles.tabActive : ""
                }`}
                onClick={() => setActivePid(t.pid)}
              >
                <span>
                  {t.name}
                  {t.gone && (
                    <span className={styles.processGoneBadge}>(ended)</span>
                  )}
                </span>
                <button
                  type="button"
                  className={styles.tabClose}
                  onClick={(e) => {
                    e.stopPropagation();
                    closeTab(t.pid);
                  }}
                  aria-label="Close tab"
                >
                  <X size={14} />
                </button>
              </button>
            ))}
          </div>
        )}
        {activeTab ? (
          <div className={styles.splitContainer}>
            <div className={styles.panel} style={{ flex: "0 0 45%" }}>
              <div className={styles.panelHeader}>Stdin</div>
              <div className={styles.panelContent}>
                {activeTab.stdinBuffer || "\u00a0"}
              </div>
              <div className={styles.stdinInputRow}>
                <input
                  type="text"
                  className={styles.stdinInput}
                  value={stdinInput}
                  onChange={(e) => setStdinInput(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      sendStdin(activeTab.pid);
                    }
                  }}
                  placeholder="Type and press Enter or Send"
                  disabled={activeTab.gone}
                />
                <button
                  type="button"
                  className={styles.sendBtn}
                  onClick={() => sendStdin(activeTab.pid)}
                  disabled={activeTab.gone || !stdinInput.trim()}
                >
                  <Send size={14} />
                  Send
                </button>
              </div>
            </div>
            <div className={styles.panel} style={{ flex: "1 1 auto" }}>
              <div className={styles.panelHeader}>Stdout</div>
              <div className={styles.panelContent}>
                {activeTab.stdoutBuffer || "\u00a0"}
              </div>
            </div>
          </div>
        ) : (
          <div className={styles.emptyState}>
            {connectionId
              ? "Select a process from the list to open a tab."
              : "Connecting to stream…"}
          </div>
        )}
      </main>
    </div>
  );
}
