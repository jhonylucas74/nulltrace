import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { highlightLua, isLuaFile, highlightSearchInHtml } from "../lib/luaHighlight";
import styles from "./TelescopeModal.module.css";

export type TelescopeMode = "search" | "findReplace" | "findFile";

export interface GrepResult {
  path: string;
  line: number;
  text: string;
}

export interface TelescopeModalProps {
  open: boolean;
  onClose: () => void;
  mode: TelescopeMode;
  rootPath: string | null;
  token: string | null;
  initialFindValue?: string;
  onOpenInEditor: (path: string, line?: number) => void;
}

function parseGrepStdout(stdout: string): GrepResult[] {
  const results: GrepResult[] = [];
  const lines = stdout.split("\n").filter((l) => l.trim());
  for (const line of lines) {
    const firstColon = line.indexOf(":");
    const secondColon = line.indexOf(":", firstColon + 1);
    if (firstColon >= 0 && secondColon >= 0) {
      const path = line.slice(0, firstColon);
      const lineNum = parseInt(line.slice(firstColon + 1, secondColon), 10);
      const text = line.slice(secondColon + 1);
      if (!Number.isNaN(lineNum)) results.push({ path, line: lineNum, text });
    }
  }
  return results;
}

export default function TelescopeModal({
  open,
  onClose,
  mode,
  rootPath,
  token,
  initialFindValue = "",
  onOpenInEditor,
}: TelescopeModalProps) {
  const [searchQuery, setSearchQuery] = useState(initialFindValue);
  const [replaceValue, setReplaceValue] = useState("");
  /** Case-sensitive search: false = insensitive (default), true = sensitive. */
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [grepResults, setGrepResults] = useState<GrepResult[]>([]);
  const [fileList, setFileList] = useState<string[]>([]);
  const [fileListAll, setFileListAll] = useState<string[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [previewContent, setPreviewContent] = useState<string | null>(null);
  const [previewPath, setPreviewPath] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const replaceInputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const tauri = typeof window !== "undefined" && (window as unknown as { __TAURI__?: unknown }).__TAURI__;

  const canSearch = !!(tauri && token && rootPath);
  const isGrepMode = mode === "search" || mode === "findReplace";
  const results = isGrepMode ? grepResults : fileList;
  const selectedResult = results[selectedIndex];

  // Sync initial find value when modal opens
  useEffect(() => {
    if (open) {
      setSearchQuery(initialFindValue);
      setSelectedIndex(0);
      setPreviewContent(null);
      setPreviewPath(null);
      if (mode === "findFile") {
        setFileList([]);
        setFileListAll([]);
        if (tauri && token && rootPath) {
          setLoading(true);
          invoke<{ stdout: string; exit_code: number }>("grpc_run_process", {
            binName: "find",
            args: [rootPath, "-type", "f"],
            token,
          })
            .then((res) => {
              const paths = res.stdout.split("\n").filter((p) => p.trim());
              setFileListAll(paths);
              setFileList(paths);
            })
            .catch(() => {
              setFileListAll([]);
              setFileList([]);
            })
            .finally(() => setLoading(false));
        }
      } else {
        setGrepResults([]);
      }
      setTimeout(() => searchInputRef.current?.focus(), 0);
    }
  }, [open, mode, initialFindValue, rootPath, token, tauri]);

  // Load preview when selection changes
  useEffect(() => {
    if (!open || !tauri || !token) return;
    const path = isGrepMode
      ? (selectedResult as GrepResult | undefined)?.path
      : (selectedResult as string | undefined);
    if (!path) {
      setPreviewContent(null);
      setPreviewPath(null);
      return;
    }
    setPreviewPath(path);
    invoke<{ success: boolean; content: string }>("grpc_read_file", { path, token })
      .then((res) => {
        if (res.success) setPreviewContent(res.content);
        else setPreviewContent(null);
      })
      .catch(() => setPreviewContent(null));
  }, [open, selectedIndex, selectedResult, isGrepMode, token, tauri]);

  // Filter file list when search query changes (findFile mode)
  useEffect(() => {
    if (mode !== "findFile" || !open) return;
    const q = searchQuery.trim().toLowerCase();
    if (!q) {
      setFileList(fileListAll);
    } else {
      setFileList(fileListAll.filter((p) => p.toLowerCase().includes(q)));
    }
    setSelectedIndex(0);
  }, [mode, open, searchQuery, fileListAll]);

  // Keyboard: Escape close, Enter run search (grep modes), Arrow keys nav
  useEffect(() => {
    if (!open) return;
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        onClose();
        e.preventDefault();
        return;
      }
      if (e.key === "Enter" && document.activeElement === searchInputRef.current) {
        if (isGrepMode && canSearch && searchQuery.trim()) runGrep();
        e.preventDefault();
        return;
      }
      const target = e.target as Node;
      const inSearchInput = searchInputRef.current?.contains?.(target) ?? target === searchInputRef.current;
      const inReplaceInput = replaceInputRef.current?.contains?.(target) ?? target === replaceInputRef.current;
      if (!inSearchInput && !inReplaceInput) {
        if (e.key === "ArrowDown") {
          setSelectedIndex((i) => Math.min(i + 1, results.length - 1));
          e.preventDefault();
        } else if (e.key === "ArrowUp") {
          setSelectedIndex((i) => Math.max(i - 1, 0));
          e.preventDefault();
        }
      }
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [open, onClose, isGrepMode, canSearch, searchQuery, results.length]);

  const runGrep = useCallback(async () => {
    if (!tauri || !token || !rootPath || !searchQuery.trim()) return;
    setLoading(true);
    setGrepResults([]);
    const args = caseSensitive
      ? [searchQuery.trim(), "-r", rootPath]
      : ["-i", searchQuery.trim(), "-r", rootPath];
    try {
      const res = await invoke<{ stdout: string; exit_code: number }>("grpc_run_process", {
        binName: "grep",
        args,
        token,
      });
      setGrepResults(parseGrepStdout(res.stdout));
      setSelectedIndex(0);
    } catch {
      setGrepResults([]);
    } finally {
      setLoading(false);
    }
  }, [tauri, token, rootPath, searchQuery, caseSensitive]);

  const handleOpenInEditor = useCallback(() => {
    if (!selectedResult) return;
    if (isGrepMode) {
      const r = selectedResult as GrepResult;
      onOpenInEditor(r.path, r.line);
    } else {
      onOpenInEditor(selectedResult as string);
    }
    onClose();
  }, [selectedResult, isGrepMode, onOpenInEditor, onClose]);

  const handleReplaceAllInFile = useCallback(async () => {
    if (mode !== "findReplace" || !previewPath || !searchQuery.trim() || !tauri || !token) return;
    if (!previewContent) return;
    const next = previewContent.split(searchQuery).join(replaceValue);
    try {
      const res = await invoke<{ success: boolean; error_message: string }>("grpc_write_file", {
        path: previewPath,
        content: next,
        token,
      });
      if (res.success) {
        setPreviewContent(next);
        setGrepResults((prev) => prev.filter((r) => r.path !== previewPath));
      }
    } catch {
      // ignore
    }
  }, [mode, previewPath, previewContent, searchQuery, replaceValue, tauri, token]);

  if (!open) return null;

  const modeTitle =
    mode === "search" ? "Search" : mode === "findReplace" ? "Find and Replace" : "Find File";

  const renderPreview = () => {
    if (!previewContent && !loading) {
      return <div className={styles.previewEmpty}>Select a file to preview</div>;
    }
    if (!previewContent) return null;
    const searchTerm = isGrepMode ? searchQuery.trim() : "";
    const lines = previewContent.split("\n");
    const selectedLineNumber =
      isGrepMode && selectedResult
        ? (selectedResult as GrepResult).line
        : null;
    const highlightLine = (line: string, path: string | null) => {
      const isLua = path ? isLuaFile(path) : false;
      const html = isLua ? highlightLua(line) : line.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
      const withSearch = searchTerm
        ? highlightSearchInHtml(html, searchTerm, !caseSensitive)
        : html;
      return withSearch;
    };
    return (
      <div className={styles.previewPane}>
        {lines.map((line, i) => {
          const lineNum = i + 1;
          const isSelectedLine = selectedLineNumber !== null && lineNum === selectedLineNumber;
          return (
            <div
              key={i}
              className={`${styles.previewLine} ${isSelectedLine ? styles.previewLineSelected : ""}`}
            >
              <span
                className={styles.previewContent}
                dangerouslySetInnerHTML={{ __html: highlightLine(line, previewPath) }}
              />
            </div>
          );
        })}
      </div>
    );
  };

  return (
    <div
      className={styles.overlay}
      role="dialog"
      aria-modal="true"
      aria-label={modeTitle}
    >
      <div className={styles.panel}>
        <div className={styles.header}>
          <h2 className={styles.title}>{modeTitle}</h2>
          <input
            ref={searchInputRef}
            type="text"
            className={styles.searchInput}
            placeholder={mode === "findFile" ? "Filter files…" : "Search…"}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            aria-label="Search"
          />
          {isGrepMode && (
            <label className={styles.caseSensitiveLabel}>
              <input
                type="checkbox"
                className={styles.checkbox}
                checked={caseSensitive}
                onChange={(e) => setCaseSensitive(e.target.checked)}
                aria-label="Case sensitive"
              />
              Case sensitive
            </label>
          )}
          {mode === "findReplace" && (
            <div className={styles.replaceRow}>
              <span className={styles.replaceLabel}>Replace:</span>
              <input
                ref={replaceInputRef}
                type="text"
                className={styles.replaceInput}
                placeholder="Replace"
                value={replaceValue}
                onChange={(e) => setReplaceValue(e.target.value)}
                aria-label="Replace"
              />
            </div>
          )}
          <button type="button" className={styles.closeBtn} onClick={onClose} aria-label="Close">
            ×
          </button>
        </div>

        {!canSearch && isGrepMode && (
          <div className={styles.loading}>Open a folder and sign in to search in project.</div>
        )}

        <div className={styles.body}>
          <div className={styles.listPane} ref={listRef}>
            {loading && <div className={styles.loading}>Searching…</div>}
            {!loading && isGrepMode && results.map((r, i) => {
              const item = r as GrepResult;
              return (
                <button
                  key={`${item.path}:${item.line}:${i}`}
                  type="button"
                  className={`${styles.listItem} ${i === selectedIndex ? styles.listItemSelected : ""}`}
                  onClick={() => setSelectedIndex(i)}
                >
                  <span>{item.path}</span>
                  <span className={styles.listItemLine}>:{item.line}</span>
                  <span title={item.text}> {item.text.trim().slice(0, 60)}{item.text.length > 60 ? "…" : ""}</span>
                </button>
              );
            })}
            {!loading && mode === "findFile" && (results as string[]).map((path, i) => (
              <button
                key={path}
                type="button"
                className={`${styles.listItem} ${i === selectedIndex ? styles.listItemSelected : ""}`}
                onClick={() => setSelectedIndex(i)}
              >
                {path}
              </button>
            ))}
          </div>
          <div className={styles.previewPane}>
            {renderPreview()}
          </div>
        </div>

        <div className={styles.actions}>
          <button type="button" className={`${styles.actionBtn} ${styles.actionBtnPrimary}`} onClick={handleOpenInEditor}>
            Open in editor
          </button>
          {mode === "findReplace" && previewPath && (
            <button type="button" className={styles.actionBtn} onClick={handleReplaceAllInFile}>
              Replace all in file
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
