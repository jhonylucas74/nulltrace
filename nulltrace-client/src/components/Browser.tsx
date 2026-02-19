import { useState, useCallback, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Plus, X } from "lucide-react";
import {
  getPageHtml,
  getPageTitle,
  DEFAULT_BROWSER_URL,
  BROWSER_HISTORY_URL,
  CONNECTION_ERROR_HTML,
  NTML_PROCESSING_ERROR_HTML,
  httpErrorHtml,
} from "../lib/browserPages";
import {
  isVmUrl,
  normalizeVmUrl,
  parseHttpResponse,
  resolveScriptUrl,
  getBaseHost,
} from "../lib/browserVm";
import { renderLucideIconToSvg } from "../lib/lucideNtmlIcons";
import { useAuth } from "../contexts/AuthContext";
import styles from "./Browser.module.css";

export interface HistoryEntry {
  url: string;
  title: string;
  timestamp: number;
}

export interface BrowserTab {
  id: string;
  url: string;
  title: string;
  content: string | null;
  loading: boolean;
  error: boolean;
  errorStatus?: number;
  contentType: "html" | "text" | null;
}

function now() {
  return Date.now();
}

function generateTabId() {
  return `tab-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

export default function Browser() {
  const { token } = useAuth();
  const tauri = typeof window !== "undefined" && (window as unknown as { __TAURI__?: unknown }).__TAURI__;

  const [tabs, setTabs] = useState<BrowserTab[]>(() => {
    const isVm = isVmUrl(DEFAULT_BROWSER_URL);
    return [
      {
        id: generateTabId(),
        url: DEFAULT_BROWSER_URL,
        title: getPageTitle(DEFAULT_BROWSER_URL),
        content: isVm ? null : getPageHtml(DEFAULT_BROWSER_URL),
        loading: isVm,
        error: false,
        contentType: isVm ? null : ("html" as const),
      },
    ];
  });
  const [activeTabId, setActiveTabId] = useState<string>(tabs[0]?.id ?? "");
  const [history, setHistory] = useState<HistoryEntry[]>([
    { url: DEFAULT_BROWSER_URL, title: getPageTitle(DEFAULT_BROWSER_URL), timestamp: now() },
  ]);
  const [historyIndex, setHistoryIndex] = useState(0);
  const [favorites, setFavorites] = useState<Omit<HistoryEntry, "timestamp">[]>([]);
  const [addressBarValue, setAddressBarValue] = useState(DEFAULT_BROWSER_URL);
  const addressInputRef = useRef<HTMLInputElement>(null);
  const iframeRef = useRef<HTMLIFrameElement>(null);

  const activeTab = tabs.find((t) => t.id === activeTabId) ?? tabs[0];
  const showHistoryPage = activeTab?.url === BROWSER_HISTORY_URL;

  // Keep address bar in sync when active tab changes.
  useEffect(() => {
    if (activeTab) setAddressBarValue(activeTab.url);
  }, [activeTab?.id, activeTab?.url]);

  // Resolve [data-lucide] placeholders in NTML-rendered iframe to Lucide SVG icons.
  useEffect(() => {
    if (activeTab?.contentType !== "html" || !activeTab?.content) return;
    const timer = window.setTimeout(() => {
      const iframe = iframeRef.current;
      if (!iframe?.contentDocument) return;
      const doc = iframe.contentDocument;
      const placeholders = doc.querySelectorAll<HTMLElement>("[data-lucide]");
      placeholders.forEach((el) => {
        const name = el.getAttribute("data-lucide");
        const size = Math.max(1, parseInt(el.getAttribute("data-size") ?? "24", 10));
        const className = el.getAttribute("class") ?? undefined;
        if (!name) return;
        const svgString = renderLucideIconToSvg(name, size, className ?? undefined);
        if (!svgString) return;
        const wrap = doc.createElement("div");
        wrap.innerHTML = svgString;
        const svg = wrap.firstElementChild;
        if (svg) {
          svg.setAttribute("style", el.getAttribute("style") ?? "");
          el.parentNode?.replaceChild(svg, el);
        }
      });
    }, 0);
    return () => window.clearTimeout(timer);
  }, [activeTab?.id, activeTab?.content, activeTab?.contentType]);

  const fetchVmUrl = useCallback(
    async (tabId: string, url: string) => {
      if (!tauri || !token) {
        setTabs((prev) =>
          prev.map((t) =>
            t.id === tabId
              ? {
                  ...t,
                  loading: false,
                  error: true,
                  content: CONNECTION_ERROR_HTML,
                  contentType: "html" as const,
                }
              : t
          )
        );
        return;
      }
      const curlUrl = normalizeVmUrl(url);
      try {
        const res = await invoke<{ stdout: string; exit_code: number }>("grpc_run_process", {
          binName: "curl",
          args: [curlUrl],
          token,
        });
        const parsed = parseHttpResponse(res.stdout);
        console.log("[Browser VM fetch]", { url: curlUrl, exitCode: res.exit_code, parsed });
        if (!parsed) {
          setTabs((prev) =>
            prev.map((t) =>
              t.id === tabId
                ? {
                    ...t,
                    loading: false,
                    error: true,
                    content: CONNECTION_ERROR_HTML,
                    contentType: "html" as const,
                  }
                : t
            )
          );
          return;
        }
        if (parsed.status >= 400) {
          setTabs((prev) =>
            prev.map((t) =>
              t.id === tabId
                ? {
                    ...t,
                    loading: false,
                    error: true,
                    errorStatus: parsed.status,
                    content: httpErrorHtml(parsed.status),
                    contentType: "html" as const,
                  }
                : t
            )
          );
          return;
        }
        const ct = parsed.contentType?.toLowerCase() ?? "";
        if (ct.includes("text/plain")) {
          setTabs((prev) =>
            prev.map((t) =>
              t.id === tabId
                ? {
                    ...t,
                    loading: false,
                    error: false,
                    content: parsed.body,
                    contentType: "text" as const,
                  }
                : t
            )
          );
        } else if (ct.includes("application/x-ntml")) {
          try {
            const resources = await invoke<{
              scripts: { src: string }[];
              imports: { src: string; alias: string }[];
            }>("ntml_get_head_resources", { yaml: parsed.body });

            const baseUrl = url;
            const scriptSources: { src: string; content: string }[] = [];
            const importContents: { alias: string; content: string }[] = [];

            for (const s of resources.scripts) {
              const scriptUrl = resolveScriptUrl(baseUrl, s.src);
              try {
                const scriptRes = await invoke<{ stdout: string; exit_code: number }>(
                  "grpc_run_process",
                  { binName: "curl", args: [scriptUrl], token }
                );
                const scriptParsed = parseHttpResponse(scriptRes.stdout);
                if (scriptParsed && scriptParsed.status === 200) {
                  scriptSources.push({ src: s.src, content: scriptParsed.body });
                }
              } catch {
                // Skip failed script fetch
              }
            }

            for (const imp of resources.imports) {
              const importUrl = resolveScriptUrl(baseUrl, imp.src);
              try {
                const importRes = await invoke<{ stdout: string; exit_code: number }>(
                  "grpc_run_process",
                  { binName: "curl", args: [importUrl], token }
                );
                const importParsed = parseHttpResponse(importRes.stdout);
                if (importParsed && importParsed.status === 200) {
                  importContents.push({ alias: imp.alias, content: importParsed.body });
                }
              } catch {
                // Skip failed import fetch
              }
            }

            const baseUrlForImages =
              url.startsWith("http") ? new URL(url).origin : `http://${getBaseHost(baseUrl)}`;
            await invoke("ntml_create_tab_state", {
              tabId,
              baseUrl: baseUrlForImages,
              scriptSources,
              componentYaml: parsed.body,
              imports: importContents,
            });

            const html = await invoke<string>("ntml_to_html", {
              yaml: parsed.body,
              imports: importContents,
              baseUrl: baseUrlForImages,
            });
            setTabs((prev) =>
              prev.map((t) =>
                t.id === tabId
                  ? {
                      ...t,
                      loading: false,
                      error: false,
                      content: html,
                      contentType: "html" as const,
                    }
                  : t
              )
            );
          } catch (err) {
            console.error("[Browser NTML processing error]", err);
            setTabs((prev) =>
              prev.map((t) =>
                t.id === tabId
                  ? {
                      ...t,
                      loading: false,
                      error: true,
                      content: NTML_PROCESSING_ERROR_HTML,
                      contentType: "html" as const,
                    }
                  : t
              )
            );
          }
        } else {
          // Default: treat as text for safety
          setTabs((prev) =>
            prev.map((t) =>
              t.id === tabId
                ? {
                    ...t,
                    loading: false,
                    error: false,
                    content: parsed.body,
                    contentType: "text" as const,
                  }
                : t
            )
          );
        }
      } catch (err) {
        console.error("[Browser VM fetch error]", err);
        setTabs((prev) =>
          prev.map((t) =>
            t.id === tabId
              ? {
                  ...t,
                  loading: false,
                  error: true,
                  content: CONNECTION_ERROR_HTML,
                  contentType: "html" as const,
                }
              : t
          )
        );
      }
    },
    [tauri, token]
  );

  // Fetch initial VM URL on mount (e.g. ntml.org).
  useEffect(() => {
    const first = tabs[0];
    if (first?.loading && isVmUrl(first.url) && tauri && token) {
      fetchVmUrl(first.id, first.url);
    }
  }, [tauri, token, fetchVmUrl]);

  const navigateTo = useCallback(
    (url: string, pushHistory = true) => {
      const u = url.trim() || DEFAULT_BROWSER_URL;
      if (!activeTabId) return;

      if (pushHistory) {
        setHistory((prev) => {
          const trimmed = prev.slice(0, historyIndex + 1);
          return [...trimmed, { url: u, title: getPageTitle(u), timestamp: now() }];
        });
        setHistoryIndex((prev) => prev + 1);
      }

      if (isVmUrl(u)) {
        setTabs((prev) =>
          prev.map((t) =>
            t.id === activeTabId
              ? {
                  ...t,
                  url: u,
                  title: u,
                  content: null,
                  loading: true,
                  error: false,
                  contentType: null,
                }
              : t
          )
        );
        fetchVmUrl(activeTabId, u);
      } else {
        setTabs((prev) =>
          prev.map((t) =>
            t.id === activeTabId
              ? {
                  ...t,
                  url: u,
                  title: getPageTitle(u),
                  content: getPageHtml(u),
                  loading: false,
                  error: false,
                  contentType: "html",
                }
              : t
          )
        );
      }
    },
    [activeTabId, historyIndex, fetchVmUrl]
  );

  // Listen for NTML button actions and Link navigation from iframe (postMessage).
  useEffect(() => {
    if (!tauri) return;
    const handler = (e: MessageEvent) => {
      const d = e.data;
      if (d?.type === "ntml-action" && typeof d.action === "string") {
        invoke<{ html: string }>("ntml_run_handler", {
          tabId: activeTabId,
          action: d.action,
        })
          .then((res) => {
            setTabs((prev) =>
              prev.map((t) =>
                t.id === activeTabId ? { ...t, content: res.html } : t
              )
            );
          })
          .catch(() => {});
      } else if (d?.type === "ntml-navigate" && typeof d.url === "string") {
        const url = d.url.trim() || DEFAULT_BROWSER_URL;
        const target = d.target === "new" ? "new" : "same";
        if (target === "new") {
          const newTab: BrowserTab = {
            id: generateTabId(),
            url,
            title: getPageTitle(url),
            content: isVmUrl(url) ? null : getPageHtml(url),
            loading: isVmUrl(url),
            error: false,
            contentType: isVmUrl(url) ? null : ("html" as const),
          };
          setTabs((prev) => [...prev, newTab]);
          setActiveTabId(newTab.id);
          if (isVmUrl(url) && token) {
            fetchVmUrl(newTab.id, url);
          }
        } else {
          navigateTo(url);
        }
      }
    };
    window.addEventListener("message", handler);
    return () => window.removeEventListener("message", handler);
  }, [tauri, activeTabId, navigateTo, fetchVmUrl, token]);

  const goBack = useCallback(() => {
    if (historyIndex <= 0) return;
    const next = historyIndex - 1;
    setHistoryIndex(next);
    navigateTo(history[next].url, false);
  }, [historyIndex, history, navigateTo]);

  const goForward = useCallback(() => {
    if (historyIndex >= history.length - 1) return;
    const next = historyIndex + 1;
    setHistoryIndex(next);
    navigateTo(history[next].url, false);
  }, [historyIndex, history, navigateTo]);

  const handleAddressSubmit = useCallback(() => {
    navigateTo(addressBarValue);
    addressInputRef.current?.blur();
  }, [addressBarValue, navigateTo]);

  const toggleFavorite = useCallback(() => {
    const entry = { url: activeTab?.url ?? "", title: getPageTitle(activeTab?.url ?? "") };
    setFavorites((prev) => {
      const exists = prev.some((f) => f.url === entry.url);
      if (exists) return prev.filter((f) => f.url !== entry.url);
      return [...prev, entry];
    });
  }, [activeTab?.url]);

  const isFavorite = favorites.some((f) => f.url === (activeTab?.url ?? ""));

  const goToHistoryEntry = useCallback(
    (index: number) => {
      if (index < 0 || index >= history.length) return;
      setHistoryIndex(index);
      navigateTo(history[index].url, false);
    },
    [history, navigateTo]
  );

  const addTab = useCallback(() => {
    const newTab: BrowserTab = {
      id: generateTabId(),
      url: DEFAULT_BROWSER_URL,
      title: getPageTitle(DEFAULT_BROWSER_URL),
      content: getPageHtml(DEFAULT_BROWSER_URL),
      loading: false,
      error: false,
      contentType: "html",
    };
    setTabs((prev) => [...prev, newTab]);
    setActiveTabId(newTab.id);
  }, []);

  const closeTab = useCallback((tabId: string) => {
    invoke("ntml_close_tab", { tabId }).catch(() => {});
    setTabs((prev) => {
      const idx = prev.findIndex((t) => t.id === tabId);
      if (idx < 0) return prev;
      const next = prev.filter((t) => t.id !== tabId);
      if (next.length === 0) {
        const newTab: BrowserTab = {
          id: generateTabId(),
          url: DEFAULT_BROWSER_URL,
          title: getPageTitle(DEFAULT_BROWSER_URL),
          content: getPageHtml(DEFAULT_BROWSER_URL),
          loading: false,
          error: false,
          contentType: "html",
        };
        setActiveTabId(newTab.id);
        return [newTab];
      }
      if (prev[idx].id === activeTabId) {
        const newActiveIdx = Math.min(idx, next.length - 1);
        setActiveTabId(next[newActiveIdx].id);
      }
      return next;
    });
  }, [activeTabId]);

  const canBack = historyIndex > 0;
  const canForward = historyIndex < history.length - 1;

  return (
    <div className={styles.app}>
      <div className={styles.toolbarWrap}>
        <div className={styles.tabBar}>
          {tabs.map((tab) => (
            <div
              key={tab.id}
              role="tab"
              tabIndex={0}
              className={`${styles.tab} ${tab.id === activeTabId ? styles.tabActive : ""}`}
              onClick={() => setActiveTabId(tab.id)}
              onKeyDown={(e) => e.key === "Enter" && setActiveTabId(tab.id)}
              title={tab.url}
            >
              <span className={styles.tabTitle}>{tab.title || "New Tab"}</span>
              <button
                type="button"
                className={styles.tabClose}
                onClick={(e) => {
                  e.stopPropagation();
                  closeTab(tab.id);
                }}
                aria-label="Close tab"
              >
                <X size={14} />
              </button>
            </div>
          ))}
          <button
            type="button"
            className={styles.tabAdd}
            onClick={addTab}
            aria-label="New tab"
          >
            <Plus size={18} />
          </button>
        </div>
        <div className={styles.toolbar}>
          <div className={styles.navButtons}>
            <button
              type="button"
              className={styles.navBtn}
              onClick={goBack}
              disabled={!canBack}
              aria-label="Back"
              title="Back"
            >
              <BackIcon />
            </button>
            <button
              type="button"
              className={styles.navBtn}
              onClick={goForward}
              disabled={!canForward}
              aria-label="Forward"
              title="Forward"
            >
              <ForwardIcon />
            </button>
          </div>
          <div className={styles.addressWrap}>
            <input
              ref={addressInputRef}
              type="text"
              className={styles.addressBar}
              value={addressBarValue}
              onChange={(e) => setAddressBarValue(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleAddressSubmit()}
              placeholder="Search or enter address"
              aria-label="Address"
            />
            <button
              type="button"
              className={styles.goBtn}
              onClick={handleAddressSubmit}
              aria-label="Go"
            >
              Go
            </button>
          </div>
          <button
            type="button"
            className={`${styles.toolbarBtn} ${isFavorite ? styles.favoriteActive : ""}`}
            onClick={toggleFavorite}
            aria-label={isFavorite ? "Remove from favorites" : "Add to favorites"}
            title={isFavorite ? "Remove from favorites" : "Add to favorites"}
          >
            <StarIcon filled={isFavorite} />
          </button>
          <button
            type="button"
            className={styles.toolbarBtn}
            onClick={() => navigateTo(BROWSER_HISTORY_URL)}
            aria-label="History"
            title="History"
          >
            <HistoryIcon />
          </button>
        </div>
      </div>

      <div className={styles.content}>
        {showHistoryPage ? (
          <HistoryPageView
            history={history}
            historyIndex={historyIndex}
            onSelectEntry={goToHistoryEntry}
            onGoBack={goBack}
            canGoBack={canBack}
            onGoHome={() => navigateTo(DEFAULT_BROWSER_URL, false)}
          />
        ) : activeTab?.contentType === "text" ? (
          <pre className={styles.textContent} style={{ margin: 0 }}>
            {activeTab.content ?? ""}
          </pre>
        ) : activeTab?.loading ? (
          <div className={styles.textContent}>Loading...</div>
        ) : (
          <iframe
            ref={iframeRef}
            title="Page content"
            className={styles.iframe}
            sandbox="allow-scripts allow-same-origin"
            srcDoc={activeTab?.content ?? ""}
          />
        )}
      </div>
    </div>
  );
}

/** Groups history entries by Today, Yesterday, Last 7 days, Older. */
function groupHistoryByDate(entries: HistoryEntry[]): { label: string; entries: { entry: HistoryEntry; index: number }[] }[] {
  const nowDate = new Date();
  const todayStart = new Date(nowDate.getFullYear(), nowDate.getMonth(), nowDate.getDate()).getTime();
  const oneDay = 24 * 60 * 60 * 1000;
  const yesterdayStart = todayStart - oneDay;
  const sevenDaysStart = todayStart - 7 * oneDay;

  const groups: { label: string; entries: { entry: HistoryEntry; index: number }[] }[] = [
    { label: "Today", entries: [] },
    { label: "Yesterday", entries: [] },
    { label: "Last 7 days", entries: [] },
    { label: "Older", entries: [] },
  ];

  entries.forEach((entry, index) => {
    if (entry.url === BROWSER_HISTORY_URL) return;
    const t = entry.timestamp;
    const item = { entry, index };
    if (t >= todayStart) groups[0].entries.push(item);
    else if (t >= yesterdayStart) groups[1].entries.push(item);
    else if (t >= sevenDaysStart) groups[2].entries.push(item);
    else groups[3].entries.push(item);
  });

  return groups.filter((g) => g.entries.length > 0);
}

function HistoryPageView({
  history,
  historyIndex,
  onSelectEntry,
  onGoBack,
  canGoBack,
  onGoHome,
}: {
  history: HistoryEntry[];
  historyIndex: number;
  onSelectEntry: (index: number) => void;
  onGoBack: () => void;
  canGoBack: boolean;
  onGoHome: () => void;
}) {
  const groups = groupHistoryByDate(history);

  return (
    <div className={styles.historyPage}>
      <div className={styles.historyPageInner}>
        <div className={styles.historyPageActions}>
          {canGoBack ? (
            <button type="button" className={styles.historyPageBack} onClick={onGoBack}>
              ← Back
            </button>
          ) : (
            <button type="button" className={styles.historyPageBack} onClick={onGoHome}>
              Go to home
            </button>
          )}
        </div>
        <h1 className={styles.historyPageTitle}>History</h1>
        <p className={styles.historyPageSubtitle}>Browse your session history. Click an item to open it.</p>
        {groups.length === 0 ? (
          <p className={styles.historyPageEmpty}>No history yet.</p>
        ) : (
          groups.map((group) => (
            <section key={group.label} className={styles.historyPageSection}>
              <h2 className={styles.historyPageSectionTitle}>{group.label}</h2>
              <ul className={styles.historyPageList}>
                {group.entries.map(({ entry, index }) => (
                  <li key={`${entry.url}-${index}`}>
                    <button
                      type="button"
                      className={`${styles.historyPageItem} ${index === historyIndex ? styles.historyPageItemActive : ""}`}
                      onClick={() => onSelectEntry(index)}
                    >
                      <span className={styles.historyPageItemTitle}>{entry.title}</span>
                      <span className={styles.historyPageItemUrl}>{entry.url}</span>
                    </button>
                  </li>
                ))}
              </ul>
            </section>
          ))
        )}
      </div>
    </div>
  );
}

function BackIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M19 12H5M12 19l-7-7 7-7" />
    </svg>
  );
}

function ForwardIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M5 12h14M12 5l7 7-7 7" />
    </svg>
  );
}

function StarIcon({ filled }: { filled: boolean }) {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill={filled ? "currentColor" : "none"} stroke="currentColor" strokeWidth="2">
      <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
    </svg>
  );
}

function HistoryIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <circle cx="12" cy="12" r="10" />
      <polyline points="12 6 12 12 16 14" />
    </svg>
  );
}
