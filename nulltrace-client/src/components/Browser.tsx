import { useState, useCallback, useRef, useEffect } from "react";
import {
  getPageHtml,
  getPageTitle,
  DEFAULT_BROWSER_URL,
  BROWSER_HISTORY_URL,
} from "../lib/browserPages";
import styles from "./Browser.module.css";

export interface HistoryEntry {
  url: string;
  title: string;
  timestamp: number;
}

function now() {
  return Date.now();
}

export default function Browser() {
  const [currentUrl, setCurrentUrl] = useState(DEFAULT_BROWSER_URL);
  const [history, setHistory] = useState<HistoryEntry[]>([
    { url: DEFAULT_BROWSER_URL, title: getPageTitle(DEFAULT_BROWSER_URL), timestamp: now() },
  ]);
  const [historyIndex, setHistoryIndex] = useState(0);
  const [favorites, setFavorites] = useState<Omit<HistoryEntry, "timestamp">[]>([]);
  const [addressBarValue, setAddressBarValue] = useState(DEFAULT_BROWSER_URL);
  const addressInputRef = useRef<HTMLInputElement>(null);
  const showHistoryPage = currentUrl === BROWSER_HISTORY_URL;

  // Keep address bar in sync when navigating (back/forward or programmatic).
  useEffect(() => {
    setAddressBarValue(currentUrl);
  }, [currentUrl]);

  const navigateTo = useCallback((url: string, pushHistory = true) => {
    const u = url.trim() || DEFAULT_BROWSER_URL;
    setCurrentUrl(u);
    if (pushHistory) {
      setHistory((prev) => {
        const trimmed = prev.slice(0, historyIndex + 1);
        return [...trimmed, { url: u, title: getPageTitle(u), timestamp: now() }];
      });
      setHistoryIndex((prev) => prev + 1);
    }
  }, [historyIndex]);

  const goBack = useCallback(() => {
    if (historyIndex <= 0) return;
    const next = historyIndex - 1;
    setHistoryIndex(next);
    setCurrentUrl(history[next].url);
  }, [historyIndex, history]);

  const goForward = useCallback(() => {
    if (historyIndex >= history.length - 1) return;
    const next = historyIndex + 1;
    setHistoryIndex(next);
    setCurrentUrl(history[next].url);
  }, [historyIndex, history]);

  const handleAddressSubmit = useCallback(() => {
    navigateTo(addressBarValue);
    addressInputRef.current?.blur();
  }, [addressBarValue, navigateTo]);

  const toggleFavorite = useCallback(() => {
    const entry = { url: currentUrl, title: getPageTitle(currentUrl) };
    setFavorites((prev) => {
      const exists = prev.some((f) => f.url === currentUrl);
      if (exists) return prev.filter((f) => f.url !== currentUrl);
      return [...prev, entry];
    });
  }, [currentUrl]);

  const isFavorite = favorites.some((f) => f.url === currentUrl);

  const goToHistoryEntry = useCallback((index: number) => {
    if (index < 0 || index >= history.length) return;
    setHistoryIndex(index);
    setCurrentUrl(history[index].url);
  }, [history]);

  const canBack = historyIndex > 0;
  const canForward = historyIndex < history.length - 1;

  return (
    <div className={styles.app}>
      <div className={styles.toolbarWrap}>
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
            onGoHome={() => { setHistoryIndex(0); setCurrentUrl(DEFAULT_BROWSER_URL); }}
          />
        ) : (
          <iframe
            title="Page content"
            className={styles.iframe}
            srcDoc={getPageHtml(currentUrl)}
          />
        )}
      </div>
    </div>
  );
}

/** Groups history entries by Today, Yesterday, Last 7 days, Older. */
function groupHistoryByDate(entries: HistoryEntry[]): { label: string; entries: { entry: HistoryEntry; index: number }[] }[] {
  const now = new Date();
  const todayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
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
