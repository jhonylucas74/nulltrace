import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Play, Square } from "lucide-react";
import {
  getChildren,
  getHomePath,
  createFile,
  getFileContent,
  setFileContent,
} from "../lib/fileSystem";
import { useAuth } from "../contexts/AuthContext";
import { useFilePicker } from "../contexts/FilePickerContext";
import { highlightLua, isLuaFile } from "../lib/luaHighlight";
import ContextMenu, { type ContextMenuItem } from "./ContextMenu";
import Modal from "./Modal";
import styles from "./CodeEditor.module.css";

interface FsEntry {
  name: string;
  node_type: string;
  size_bytes: number;
}

const LINE_HEIGHT = 1.5;
const EDITOR_FONT_SIZE = "0.9rem";

function joinPath(base: string, name: string): string {
  const b = base.replace(/\/$/, "");
  return b ? `${b}/${name}` : `/${name}`;
}

export default function CodeEditor() {
  const [rootPath, setRootPath] = useState<string | null>(null);
  const [activeFilePath, setActiveFilePath] = useState<string | null>(null);
  const [expandedFolders, setExpandedFolders] = useState<string[]>([]);
  const [editorContent, setEditorContent] = useState("");
  const [fileMenuOpen, setFileMenuOpen] = useState(false);
  const [editMenuOpen, setEditMenuOpen] = useState(false);
  const [selectionMenuOpen, setSelectionMenuOpen] = useState(false);
  const [viewMenuOpen, setViewMenuOpen] = useState(false);
  const [newFileModalOpen, setNewFileModalOpen] = useState(false);
  const [newFileName, setNewFileName] = useState("");
  const [newFileError, setNewFileError] = useState("");
  const editorRef = useRef<HTMLTextAreaElement | null>(null);
  const gutterRef = useRef<HTMLDivElement | null>(null);
  const highlightRef = useRef<HTMLDivElement | null>(null);
  const menuBarRef = useRef<HTMLDivElement | null>(null);
  const [saveFeedback, setSaveFeedback] = useState(false);
  const [consoleLogs, setConsoleLogs] = useState<Array<{ type: "stdout" | "stderr" | "system"; text: string }>>([]);
  const [consoleInputPending, setConsoleInputPending] = useState(false);
  const [consoleInputValue, setConsoleInputValue] = useState("");
  const consoleEndRef = useRef<HTMLDivElement | null>(null);
  const pendingTabSelectionRef = useRef<number | null>(null);
  const { token } = useAuth();
  const { openFilePicker } = useFilePicker();
  const tauri = typeof window !== "undefined" && (window as unknown as { __TAURI__?: unknown }).__TAURI__;
  const useGrpc = !!token && !!tauri;

  /** VM tree cache: path -> list of entries (when useGrpc). */
  const [treeCache, setTreeCache] = useState<Record<string, FsEntry[]>>({});
  const [treeCacheLoading, setTreeCacheLoading] = useState(false);
  const [loadFileError, setLoadFileError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  /** When set, New File modal uses this as parent (e.g. from sidebar context "New file here"). */
  const [newFileParentOverride, setNewFileParentOverride] = useState<string | null>(null);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; folderPath: string } | null>(null);
  /** Code Run session (VM): terminal stream session_id from code_run_connect. Stop uses terminal_disconnect. */
  const [runSessionId, setRunSessionId] = useState<string | null>(null);
  const runSessionIdRef = useRef<string | null>(null);
  /** Accumulates stdout for the current run so we show one console entry even when backend sends multiple chunks. */
  const runStdoutBufferRef = useRef("");
  /** Single unlisten for terminal-output so cleanup always removes the listener (effect cleanup runs before async listen() completes in Strict Mode). */
  const terminalOutputUnlistenRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (menuBarRef.current && !menuBarRef.current.contains(e.target as Node)) {
        setFileMenuOpen(false);
        setEditMenuOpen(false);
        setSelectionMenuOpen(false);
        setViewMenuOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const toggleExpanded = useCallback((path: string) => {
    setExpandedFolders((prev) =>
      prev.includes(path) ? prev.filter((p) => p !== path) : [...prev, path]
    );
  }, []);

  /** Fetch list_fs for rootPath and expanded folders; update tree cache. */
  const fetchTreeCache = useCallback(async () => {
    if (!token || !tauri || !rootPath) return;
    setTreeCacheLoading(true);
    const pathsToFetch = [rootPath, ...expandedFolders.filter((p) => p !== rootPath)];
    const next: Record<string, FsEntry[]> = {};
    try {
      for (const p of pathsToFetch) {
        const res = await invoke<{ entries: FsEntry[]; error_message: string }>(
          "grpc_list_fs",
          { path: p, token }
        );
        if (!res.error_message) next[p] = res.entries;
      }
      setTreeCache((prev) => ({ ...prev, ...next }));
    } finally {
      setTreeCacheLoading(false);
    }
  }, [token, tauri, rootPath, expandedFolders]);

  useEffect(() => {
    if (useGrpc && rootPath) {
      fetchTreeCache();
    } else {
      setTreeCache({});
    }
  }, [useGrpc, rootPath, fetchTreeCache]);

  /** Children for a path: from VM cache (when useGrpc) or in-memory fileSystem. */
  function getChildrenForPath(path: string): { name: string; type: "folder" | "file" }[] {
    if (useGrpc && rootPath) {
      const entries = treeCache[path] ?? [];
      return entries.map((e) => ({
        name: e.name,
        type: (e.node_type === "directory" ? "folder" : "file") as "folder" | "file",
      })).sort((a, b) => (a.type !== b.type ? (a.type === "folder" ? -1 : 1) : a.name.localeCompare(b.name)));
    }
    return getChildren(path);
  }

  /** Load file content when active file changes (VM: grpc_read_file; else in-memory). */
  useEffect(() => {
    if (activeFilePath === null) {
      setEditorContent("");
      setLoadFileError(null);
      return;
    }
    if (useGrpc && token && tauri) {
      setEditorContent("");
      setLoadFileError(null);
      invoke<{ success: boolean; error_message: string; content: string }>("grpc_read_file", {
        path: activeFilePath,
        token,
      })
        .then((res) => {
          if (res.success) setEditorContent(res.content);
          else setLoadFileError(res.error_message || "Failed to read file");
        })
        .catch((e) => setLoadFileError(e instanceof Error ? e.message : String(e)));
      return;
    }
    setEditorContent(getFileContent(activeFilePath));
  }, [activeFilePath, useGrpc, token, tauri]);

  const handleSave = useCallback(async () => {
    if (!activeFilePath) return;
    if (useGrpc && token && tauri) {
      setSaveError(null);
      try {
        const res = await invoke<{ success: boolean; error_message: string }>("grpc_write_file", {
          path: activeFilePath,
          content: editorContent,
          token,
        });
        if (res.success) {
          setSaveFeedback(true);
          setTimeout(() => setSaveFeedback(false), 1200);
        } else {
          setSaveError(res.error_message || "Failed to save");
        }
      } catch (e) {
        setSaveError(e instanceof Error ? e.message : String(e));
      }
      return;
    }
    setFileContent(activeFilePath, editorContent);
    setSaveFeedback(true);
    const t = setTimeout(() => setSaveFeedback(false), 1200);
    return () => clearTimeout(t);
  }, [activeFilePath, editorContent, useGrpc, token, tauri]);

  function handleOpenFolder(path: string) {
    setRootPath(path);
    setFileMenuOpen(false);
    setExpandedFolders((prev) => [...prev, path]);
  }

  function openFolderPicker() {
    setFileMenuOpen(false);
    openFilePicker({
      mode: "folder",
      initialPath: rootPath ?? getHomePath(),
      onSelect: handleOpenFolder,
    });
  }

  function openNewFileModal() {
    setFileMenuOpen(false);
    setNewFileParentOverride(null);
    setNewFileName("");
    setNewFileError("");
    setNewFileModalOpen(true);
  }

  function openNewFileModalInFolder(folderPath: string) {
    setContextMenu(null);
    setNewFileParentOverride(folderPath);
    setNewFileName("");
    setNewFileError("");
    setNewFileModalOpen(true);
  }

  async function handleNewFileCreate() {
    const name = newFileName.trim();
    if (!name) {
      setNewFileError("Enter a file name.");
      return;
    }
    const parent = newFileParentOverride ?? rootPath ?? getHomePath();
    if (!rootPath && !newFileParentOverride) {
      setRootPath(parent);
      setExpandedFolders((prev) => [...prev, parent]);
    }
    const newPath = joinPath(parent, name);

    if (useGrpc && token && tauri) {
      setNewFileError("");
      try {
        const res = await invoke<{ success: boolean; error_message: string }>("grpc_write_file", {
          path: newPath,
          content: "",
          token,
        });
        if (!res.success) {
          setNewFileError(res.error_message || "Failed to create file");
          return;
        }
        setTreeCache((prev) => {
          const list = prev[parent] ?? [];
          if (list.some((e) => e.name === name)) return prev;
          return { ...prev, [parent]: [...list, { name, node_type: "file", size_bytes: 0 }] };
        });
        setActiveFilePath(newPath);
        setEditorContent("");
        setNewFileModalOpen(false);
        setNewFileName("");
        setNewFileError("");
        setNewFileParentOverride(null);
      } catch (e) {
        setNewFileError(e instanceof Error ? e.message : String(e));
      }
      return;
    }

    const created = createFile(parent, name);
    if (!created) {
      setNewFileError("A file or folder with that name already exists.");
      return;
    }
    setActiveFilePath(newPath);
    setEditorContent("");
    setFileContent(newPath, "");
    setNewFileModalOpen(false);
    setNewFileName("");
    setNewFileError("");
    setNewFileParentOverride(null);
  }

  function handleEditorChange(e: React.ChangeEvent<HTMLTextAreaElement>) {
    const value = e.target.value;
    setEditorContent(value);
    if (activeFilePath && !useGrpc) setFileContent(activeFilePath, value);
  }

  /** Insert tab at cursor; prevent default so focus does not leave the textarea. */
  function handleEditorKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key !== "Tab") return;
    e.preventDefault();
    const ta = editorRef.current;
    if (!ta) return;
    const start = ta.selectionStart;
    const end = ta.selectionEnd;
    const tab = "  "; /* 2 spaces per Tab */
    const before = editorContent.slice(0, start);
    const after = editorContent.slice(end);
    const nextContent = before + tab + after;
    pendingTabSelectionRef.current = start + tab.length;
    setEditorContent(nextContent);
    if (activeFilePath && !useGrpc) setFileContent(activeFilePath, nextContent);
  }

  useEffect(() => {
    runSessionIdRef.current = runSessionId;
  }, [runSessionId]);

  useEffect(() => {
    return () => {
      const sid = runSessionIdRef.current;
      if (sid && tauri) invoke("terminal_disconnect", { sessionId: sid }).catch(() => {});
    };
  }, [tauri]);

  /** Terminal-output events for Code Run (session from code_run_connect). Same event as Terminal app. */
  useEffect(() => {
    if (!tauri) return;
    terminalOutputUnlistenRef.current?.();
    terminalOutputUnlistenRef.current = null;
    (async () => {
      try {
        const u = await listen<{ sessionId?: string; type: string; data?: string }>("terminal-output", (event) => {
          const sid = runSessionIdRef.current;
          if (event.payload?.sessionId !== sid) return;
          const type_ = event.payload?.type ?? "";
          if (type_ === "stdout") {
            const data = event.payload?.data ?? "";
            runStdoutBufferRef.current += data;
            const accumulated = runStdoutBufferRef.current;
            setConsoleLogs((prev) => {
              const last = prev[prev.length - 1];
              if (last?.type === "stdout") {
                return [...prev.slice(0, -1), { type: "stdout" as const, text: accumulated }];
              }
              return [...prev, { type: "stdout", text: accumulated }];
            });
          } else if (type_ === "closed") {
            runStdoutBufferRef.current = "";
            setRunSessionId(null);
            setConsoleLogs((prev) => [...prev, { type: "system", text: "Done." }]);
          } else if (type_ === "error") {
            runStdoutBufferRef.current = "";
            setRunSessionId(null);
            setConsoleLogs((prev) => [...prev, { type: "system", text: `Error: ${event.payload?.data ?? "unknown"}` }]);
          }
        });
        terminalOutputUnlistenRef.current?.();
        terminalOutputUnlistenRef.current = u;
      } catch {
        // ignore
      }
    })();
    return () => {
      terminalOutputUnlistenRef.current?.();
      terminalOutputUnlistenRef.current = null;
    };
  }, [tauri]);

  useEffect(() => {
    if (pendingTabSelectionRef.current === null) return;
    const pos = pendingTabSelectionRef.current;
    pendingTabSelectionRef.current = null;
    const ta = editorRef.current;
    if (ta) {
      ta.focus();
      ta.setSelectionRange(pos, pos);
    }
  }, [editorContent]);

  function handleTextareaScroll() {
    const top = editorRef.current?.scrollTop ?? 0;
    if (gutterRef.current) gutterRef.current.scrollTop = top;
    if (highlightRef.current) highlightRef.current.scrollTop = top;
  }

  const lineCount = Math.max(1, editorContent.split("\n").length);

  /** Run script: VM (code_run_connect + terminal stream) when useGrpc, else simulate print/io.read for console. Saves the file automatically before running. Clears the console on each run. */
  const runScript = useCallback(async () => {
    if (!activeFilePath) return;
    setConsoleLogs([]);
    runStdoutBufferRef.current = "";
    const name = activeFilePath.split("/").pop() ?? activeFilePath;

    if (useGrpc && token && tauri) {
      await handleSave();
      setConsoleLogs((prev) => [...prev, { type: "system", text: `> Running ${name}...` }]);
      try {
        const sessionId = await invoke<string>("code_run_connect", { token, path: activeFilePath });
        setRunSessionId(sessionId);
      } catch (e) {
        setConsoleLogs((prev) => [...prev, { type: "system", text: `Run failed: ${e instanceof Error ? e.message : String(e)}` }]);
      }
      return;
    }

    setConsoleLogs((prev) => [...prev, { type: "system", text: `> Running ${name}...` }]);
    const content = editorContent;
    const printDouble = /print\s*\(\s*"((?:[^"\\]|\\.)*)"\s*\)/g;
    const printSingle = /print\s*\(\s*'((?:[^'\\]|\\.)*)'\s*\)/g;
    let m: RegExpExecArray | null;
    const printed: string[] = [];
    while ((m = printDouble.exec(content)) !== null) printed.push(m[1].replace(/\\n/g, "\n").replace(/\\t/g, "\t"));
    while ((m = printSingle.exec(content)) !== null) printed.push(m[1].replace(/\\n/g, "\n").replace(/\\t/g, "\t"));
    printed.forEach((line) =>
      setConsoleLogs((prev) => [...prev, { type: "stdout", text: line }])
    );
    const hasRead = /\bio\s*\.\s*read\s*\(/.test(content) || /\bio\.read\s*\(/.test(content);
    if (hasRead) setConsoleInputPending(true);
    else setConsoleLogs((prev) => [...prev, { type: "system", text: "Done." }]);
  }, [activeFilePath, editorContent, useGrpc, token, tauri, handleSave]);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === "n") {
        e.preventDefault();
        openNewFileModal();
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "s") {
        e.preventDefault();
        handleSave();
      }
      if (e.key === "F5") {
        e.preventDefault();
        runScript();
      }
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [handleSave, runScript]);

  const stopRun = useCallback(() => {
    const sid = runSessionId;
    if (!sid || !tauri) return;
    invoke("terminal_disconnect", { sessionId: sid }).catch(() => {});
    setRunSessionId(null);
  }, [runSessionId, tauri]);

  useEffect(() => {
    consoleEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [consoleLogs, consoleInputPending]);

  const submitConsoleInput = useCallback(() => {
    const value = consoleInputValue.trim();
    const line = value || "";
    if (runSessionId && tauri) {
      const toSend = line.endsWith("\n") ? line : line + "\n";
      invoke("terminal_send_stdin", { sessionId: runSessionId, data: toSend }).catch(() => {});
      setConsoleLogs((prev) => [...prev, { type: "stdout", text: line || "(empty input)" }]);
      setConsoleInputValue("");
      setConsoleInputPending(false);
      return;
    }
    setConsoleLogs((prev) => [...prev, { type: "stdout", text: value || "(empty input)" }]);
    setConsoleInputValue("");
    setConsoleInputPending(false);
    setConsoleLogs((prev) => [...prev, { type: "system", text: "Done." }]);
  }, [consoleInputValue, runSessionId, tauri]);

  function renderTree(path: string, depth: number): React.ReactNode {
    const children = getChildrenForPath(path);
    if (children.length === 0) return null;
    return (
      <>
        {children.map((node) => {
          const nodePath = joinPath(path, node.name);
          if (node.type === "folder") {
            const isExpanded = expandedFolders.includes(nodePath);
            return (
              <div key={nodePath} className={styles.treeFolder}>
                <button
                  type="button"
                  className={styles.treeRow}
                  style={{ paddingLeft: `${0.75 + depth * 0.75}rem` }}
                  onClick={() => toggleExpanded(nodePath)}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    setContextMenu({ x: e.clientX, y: e.clientY, folderPath: nodePath });
                  }}
                  data-type="folder"
                >
                  <span className={styles.treeChevron}>{isExpanded ? "▼" : "▶"}</span>
                  <FolderIcon />
                  <span className={styles.treeLabel}>{node.name}</span>
                </button>
                {isExpanded && (
                  <div className={styles.treeChildren}>{renderTree(nodePath, depth + 1)}</div>
                )}
              </div>
            );
          }
          return (
            <button
              key={nodePath}
              type="button"
              className={`${styles.treeRow} ${activeFilePath === nodePath ? styles.treeRowActive : ""}`}
              style={{ paddingLeft: `${0.75 + depth * 0.75}rem` }}
              onClick={() => setActiveFilePath(nodePath)}
              data-type="file"
            >
              <span className={styles.treeChevron} />
              <FileIcon />
              <span className={styles.treeLabel}>{node.name}</span>
            </button>
          );
        })}
      </>
    );
  }

  const showWelcome = rootPath === null;
  const useLuaHighlight = isLuaFile(activeFilePath);

  return (
    <div className={styles.app}>
      <div className={styles.menuBar} ref={menuBarRef}>
        <div className={styles.menuWrap}>
          <button
            type="button"
            className={styles.menuItem}
            onClick={() => {
              setFileMenuOpen((o) => !o);
              setEditMenuOpen(false);
              setSelectionMenuOpen(false);
              setViewMenuOpen(false);
            }}
          >
            File
          </button>
          {fileMenuOpen && (
            <div className={styles.menuDropdown}>
              <button type="button" className={styles.menuDropdownItem} onClick={openNewFileModal}>
                <span className={styles.menuItemLabel}>New File</span>
                <span className={styles.menuItemShortcut}>Ctrl+N</span>
              </button>
              <button type="button" className={styles.menuDropdownItem} onClick={openFolderPicker}>
                <span className={styles.menuItemLabel}>Open Folder…</span>
              </button>
              <div className={styles.menuDropdownSep} />
              <button
                type="button"
                className={styles.menuDropdownItem}
                onClick={() => { handleSave(); setFileMenuOpen(false); }}
                disabled={!activeFilePath}
              >
                <span className={styles.menuItemLabel}>Save</span>
                <span className={styles.menuItemShortcut}>Ctrl+S</span>
              </button>
            </div>
          )}
        </div>
        <div className={styles.menuWrap}>
          <button
            type="button"
            className={styles.menuItem}
            onClick={() => {
              setFileMenuOpen(false);
              setEditMenuOpen((o) => !o);
              setSelectionMenuOpen(false);
              setViewMenuOpen(false);
            }}
          >
            Edit
          </button>
          {editMenuOpen && (
            <div className={styles.menuDropdown}>
              <span className={styles.menuDropdownMuted}>Coming soon</span>
            </div>
          )}
        </div>
        <div className={styles.menuWrap}>
          <button
            type="button"
            className={styles.menuItem}
            onClick={() => {
              setFileMenuOpen(false);
              setEditMenuOpen(false);
              setSelectionMenuOpen((o) => !o);
              setViewMenuOpen(false);
            }}
          >
            Selection
          </button>
          {selectionMenuOpen && (
            <div className={styles.menuDropdown}>
              <span className={styles.menuDropdownMuted}>Coming soon</span>
            </div>
          )}
        </div>
        <div className={styles.menuWrap}>
          <button
            type="button"
            className={styles.menuItem}
            onClick={() => {
              setFileMenuOpen(false);
              setEditMenuOpen(false);
              setSelectionMenuOpen(false);
              setViewMenuOpen((o) => !o);
            }}
          >
            View
          </button>
          {viewMenuOpen && (
            <div className={styles.menuDropdown}>
              <span className={styles.menuDropdownMuted}>Coming soon</span>
            </div>
          )}
        </div>
      </div>

      <div className={styles.body}>
        <aside className={styles.sidebar} onClick={() => setContextMenu(null)}>
          {rootPath !== null && (
            <div className={styles.sidebarProjectHeader} title={rootPath}>
              <span className={styles.sidebarProjectLabel}>Project</span>
              <span className={styles.sidebarProjectPath}>{rootPath}</span>
            </div>
          )}
          <div className={styles.tree}>
            {rootPath !== null ? (
              renderTree(rootPath, 0)
            ) : (
              <div className={styles.sidebarHint}>Open a folder from File menu</div>
            )}
          </div>
        </aside>
        {contextMenu && (
          <ContextMenu
            x={contextMenu.x}
            y={contextMenu.y}
            items={[
              {
                label: "New file here",
                onClick: () => openNewFileModalInFolder(contextMenu.folderPath),
              },
            ]}
            onClose={() => setContextMenu(null)}
          />
        )}

        <div className={styles.editorArea}>
          {showWelcome ? (
            <div className={styles.welcome}>
              <div className={styles.welcomeIcon}>
                <CodeLogoIcon />
              </div>
              <h1 className={styles.welcomeTitle}>Luau Editor</h1>
              <p className={styles.welcomeDesc}>
                Welcome. This editor supports only the <strong>Lua / Luau</strong> programming language.
                Open a folder to browse files and start coding.
              </p>
              <button
                type="button"
                className={styles.welcomeBtn}
                onClick={openFolderPicker}
              >
                Open Folder
              </button>
            </div>
          ) : activeFilePath ? (
            <>
              <div className={styles.editorBar}>
                <button
                  type="button"
                  className={styles.runBtn}
                  onClick={() => runScript()}
                  title="Run script"
                  aria-label="Run script"
                  disabled={!!runSessionId}
                >
                  <Play size={14} />
                  Run
                </button>
                {runSessionId !== null && (
                  <button
                    type="button"
                    className={styles.stopBtn}
                    onClick={stopRun}
                    title="Stop script"
                    aria-label="Stop script"
                  >
                    <Square size={14} />
                    Stop
                  </button>
                )}
                <span className={styles.editorBarPath}>{activeFilePath}</span>
                {saveFeedback && <span className={styles.savedBadge}>Saved</span>}
                {loadFileError && (
                  <span className={styles.editorBarError} title={loadFileError}>
                    Load error
                  </span>
                )}
                {saveError && (
                  <span className={styles.editorBarError} title={saveError}>
                    Save error
                  </span>
                )}
              </div>
              <div className={styles.editorAndConsole}>
                <div className={styles.editorWithGutter}>
                  <div
                    ref={gutterRef}
                    className={styles.gutter}
                    style={{ lineHeight: LINE_HEIGHT, fontSize: EDITOR_FONT_SIZE }}
                  >
                    {Array.from({ length: lineCount }, (_, i) => (
                      <div key={i} className={styles.gutterLine}>
                        {i + 1}
                      </div>
                    ))}
                  </div>
                  <div className={styles.editorScrollWrap}>
                    {useLuaHighlight && (
                      <div
                        ref={highlightRef}
                        className={styles.highlightLayer}
                        style={{ lineHeight: LINE_HEIGHT, fontSize: EDITOR_FONT_SIZE }}
                        aria-hidden
                        dangerouslySetInnerHTML={{ __html: highlightLua(editorContent) }}
                      />
                    )}
                    <textarea
                      ref={editorRef}
                      className={`${styles.textarea} ${useLuaHighlight ? styles.luaHighlight : ""}`}
                      value={editorContent}
                      onChange={handleEditorChange}
                      onKeyDown={handleEditorKeyDown}
                      onScroll={handleTextareaScroll}
                      spellCheck={false}
                      style={{ lineHeight: LINE_HEIGHT, fontSize: EDITOR_FONT_SIZE }}
                    />
                  </div>
                </div>
                <div className={styles.consolePanel}>
                  <div className={styles.consoleHeader}>Console</div>
                  <div className={styles.consoleOutput}>
                    {consoleLogs.length === 0 ? (
                      <div className={styles.consoleEmpty}>Output and input requests will appear here.</div>
                    ) : (
                      consoleLogs.map((entry, i) => (
                        <div
                          key={i}
                          className={
                            entry.type === "stderr"
                              ? styles.consoleLine_stderr
                              : entry.type === "system"
                                ? styles.consoleLine_system
                                : styles.consoleLine_stdout
                          }
                          data-type={entry.type}
                        >
                          {entry.text}
                        </div>
                      ))
                    )}
                    <div ref={consoleEndRef} />
                  </div>
                  {consoleInputPending && (
                    <div className={styles.consoleInputRow}>
                      <span className={styles.consoleInputPrompt}>&gt;</span>
                      <input
                        type="text"
                        className={styles.consoleInput}
                        value={consoleInputValue}
                        onChange={(e) => setConsoleInputValue(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") submitConsoleInput();
                        }}
                        placeholder="Enter value for io.read()..."
                        aria-label="Script input"
                      />
                      <button type="button" className={styles.consoleSubmitBtn} onClick={submitConsoleInput}>
                        Submit
                      </button>
                    </div>
                  )}
                </div>
              </div>
            </>
          ) : (
            <div className={styles.placeholder}>Select a file from the sidebar or create a new one (File → New File)</div>
          )}
        </div>
      </div>

      <Modal
        open={newFileModalOpen}
        onClose={() => setNewFileModalOpen(false)}
        title="New File"
        primaryButton={{ label: "Create", onClick: handleNewFileCreate }}
        secondaryButton={{ label: "Cancel", onClick: () => setNewFileModalOpen(false) }}
      >
        <div className={styles.modalContent}>
          <label className={styles.modalLabel} htmlFor="new-file-name">
            File name
          </label>
          <input
            id="new-file-name"
            type="text"
            className={styles.modalInput}
            value={newFileName}
            onChange={(e) => {
              setNewFileName(e.target.value);
              setNewFileError("");
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                handleNewFileCreate();
              }
            }}
            placeholder="e.g. script.luau"
            autoFocus
          />
          {newFileError && <p className={styles.modalError}>{newFileError}</p>}
        </div>
      </Modal>
    </div>
  );
}

function CodeLogoIcon() {
  return (
    <svg width="80" height="80" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
      <polyline points="16 18 22 12 16 6" />
      <polyline points="8 6 2 12 8 18" />
    </svg>
  );
}

function FolderIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
      <polyline points="2 9 2 5 8 5 10 9 22 9" />
    </svg>
  );
}

function FileIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
      <polyline points="14 2 14 8 20 8" />
      <line x1="16" y1="13" x2="8" y2="13" />
      <line x1="16" y1="17" x2="8" y2="17" />
      <polyline points="10 9 9 9 8 9" />
    </svg>
  );
}
