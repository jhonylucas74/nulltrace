import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Play, Square } from "lucide-react";
import {
  getChildren,
  getHomePath,
  getParentPath,
  createFile,
  createFolder,
  renamePath as fsRenamePath,
  movePath as fsMovePath,
  getFileContent,
  setFileContent,
} from "../lib/fileSystem";
import { useAuth } from "../contexts/AuthContext";
import { useFilePicker } from "../contexts/FilePickerContext";
import { useClipboard } from "../contexts/ClipboardContext";
import { highlightLua, isLuaFile } from "../lib/luaHighlight";
import { getRecentFolders, addRecentFolder } from "../lib/codeEditorRecentFolders";
import ContextMenu from "./ContextMenu";
import Modal from "./Modal";
import TelescopeModal from "./TelescopeModal";
import styles from "./CodeEditor.module.css";

interface FsEntry {
  name: string;
  node_type: string;
  size_bytes: number;
}

const LINE_HEIGHT = 1.5;
const EDITOR_FONT_SIZE = "0.9rem";
const UNDO_MAX = 50;

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
  const [sidebarVisible, setSidebarVisible] = useState(true);
  const [consoleVisible, setConsoleVisible] = useState(true);
  /** Undo/redo: past and future content stacks; cap at UNDO_MAX. */
  const [undoPast, setUndoPast] = useState<string[]>([]);
  const [undoFuture, setUndoFuture] = useState<string[]>([]);
  const editorContentRef = useRef("");
  const pendingGoToLineRef = useRef<{ path: string; line: number } | null>(null);
  /** Telescope modal: mode and open state; when open, shortcuts (Ctrl+F etc.) are handled by modal. */
  const [telescopeOpen, setTelescopeOpen] = useState(false);
  const [telescopeMode, setTelescopeMode] = useState<"search" | "findReplace" | "findFile">("search");
  /** Initial search query when opening Telescope (e.g. from editor selection). */
  const [telescopeInitialFind, setTelescopeInitialFind] = useState("");
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
  const { token, playerId } = useAuth();
  /** Recent folders for the current user (localStorage), shown on welcome screen. */
  const [recentFolders, setRecentFolders] = useState<string[]>(() => getRecentFolders(playerId));
  const { openFilePicker } = useFilePicker();
  const { setClipboard, getClipboard, clearClipboard, hasItems: clipboardHasItems } = useClipboard();
  const tauri = typeof window !== "undefined" && (window as unknown as { __TAURI__?: unknown }).__TAURI__;
  const useGrpc = !!token && !!tauri;

  /** VM tree cache: path -> list of entries (when useGrpc). */
  const [treeCache, setTreeCache] = useState<Record<string, FsEntry[]>>({});
  const [, setTreeCacheLoading] = useState(false);
  const [loadFileError, setLoadFileError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  /** When set, New File modal uses this as parent (e.g. from sidebar context "New file here"). */
  const [newFileParentOverride, setNewFileParentOverride] = useState<string | null>(null);
  type ContextMenuBackground = { type: "background"; x: number; y: number; folderPath: string };
  type ContextMenuNode = { type: "node"; x: number; y: number; folderPath: string; fullPath: string; nodeType: "file" | "folder"; name: string };
  const [contextMenu, setContextMenu] = useState<ContextMenuBackground | ContextMenuNode | null>(null);
  const [newFolderModalOpen, setNewFolderModalOpen] = useState(false);
  const [newFolderParentOverride, setNewFolderParentOverride] = useState<string | null>(null);
  const [newFolderName, setNewFolderName] = useState("");
  const [newFolderError, setNewFolderError] = useState("");
  const [renameModalOpen, setRenameModalOpen] = useState(false);
  const [renameTargetPath, setRenameTargetPath] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [renameError, setRenameError] = useState("");
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);
  const [deleteTargetPath, setDeleteTargetPath] = useState<string | null>(null);
  const [deleteTargetName, setDeleteTargetName] = useState("");
  const [homePath, setHomePath] = useState<string | null>(null);
  const trashPath = homePath ? `${homePath.replace(/\/$/, "")}/Trash` : null;
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

  /** Fetch tree when we have token, tauri and rootPath so we don't miss a fetch if token loads after user opened a folder. */
  useEffect(() => {
    if (token && tauri && rootPath) {
      fetchTreeCache();
    } else if (!rootPath) {
      setTreeCache({});
    }
  }, [token, tauri, rootPath, fetchTreeCache]);

  useEffect(() => {
    editorContentRef.current = editorContent;
  }, [editorContent]);

  useEffect(() => {
    setRecentFolders(getRecentFolders(playerId));
  }, [playerId]);

  useEffect(() => {
    if (!useGrpc || !tauri || !token) return;
    invoke<{ path: string; error_message?: string }>("grpc_get_home_path", { token })
      .then((res) => {
        if (res.path) setHomePath(res.path);
      })
      .catch(() => {});
  }, [useGrpc, tauri, token]);

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

  /** Apply pending "go to line" (e.g. from search-in-project result click). */
  useEffect(() => {
    const pending = pendingGoToLineRef.current;
    if (!pending || pending.path !== activeFilePath || !editorContent) return;
    const lines = editorContent.split("\n");
    const lineIndex = Math.max(0, Math.min(pending.line - 1, lines.length - 1));
    const offset = lines.slice(0, lineIndex).join("\n").length;
    pendingGoToLineRef.current = null;
    requestAnimationFrame(() => {
      const ta = editorRef.current;
      if (ta) ta.setSelectionRange(offset, offset);
    });
  }, [activeFilePath, editorContent]);

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
    addRecentFolder(playerId, path);
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

  function openNewFolderModalInFolder(folderPath: string) {
    setContextMenu(null);
    setNewFolderParentOverride(folderPath);
    setNewFolderName("");
    setNewFolderError("");
    setNewFolderModalOpen(true);
  }

  async function handleNewFolderCreate() {
    const name = newFolderName.trim();
    if (!name) {
      setNewFolderError("Enter a folder name.");
      return;
    }
    const parent = newFolderParentOverride ?? rootPath ?? getHomePath();
    if (!rootPath && !newFolderParentOverride) {
      setRootPath(parent);
      setExpandedFolders((prev) => [...prev, parent]);
    }
    const newPath = joinPath(parent, name);

    if (useGrpc && token && tauri) {
      setNewFolderError("");
      try {
        const res = await invoke<{ success: boolean; error_message: string }>("grpc_create_folder", {
          path: newPath,
          token,
        });
        if (!res.success) {
          setNewFolderError(res.error_message || "Failed to create folder");
          return;
        }
        setTreeCache((prev) => {
          const list = prev[parent] ?? [];
          if (list.some((e) => e.name === name)) return prev;
          return { ...prev, [parent]: [...list, { name, node_type: "directory", size_bytes: 0 }] };
        });
        setExpandedFolders((prev) => (prev.includes(parent) ? prev : [...prev, parent]));
        setExpandedFolders((prev) => (prev.includes(newPath) ? prev : [...prev, newPath]));
        setNewFolderModalOpen(false);
        setNewFolderName("");
        setNewFolderError("");
        setNewFolderParentOverride(null);
      } catch (e) {
        setNewFolderError(e instanceof Error ? e.message : String(e));
      }
      return;
    }

    const created = createFolder(parent, name);
    if (!created) {
      setNewFolderError("A file or folder with that name already exists.");
      return;
    }
    setExpandedFolders((prev) => (prev.includes(parent) ? prev : [...prev, parent]));
    setExpandedFolders((prev) => (prev.includes(newPath) ? prev : [...prev, newPath]));
    setNewFolderModalOpen(false);
    setNewFolderName("");
    setNewFolderError("");
    setNewFolderParentOverride(null);
  }

  const DRAG_PATH_KEY = "application/x-nulltrace-path";

  const handleDropMove = useCallback(
    async (srcPath: string, destFolderPath: string) => {
      if (srcPath === destFolderPath) return;
      if (destFolderPath.startsWith(srcPath + "/")) return;
      const dest = destFolderPath.replace(/\/+$/, "");
      const base = srcPath.replace(/\/+$/, "").split("/").pop() ?? "";
      const newPath = joinPath(dest, base);

      if (useGrpc && token && tauri) {
        try {
          const res = await invoke<{ success: boolean; error_message: string }>("grpc_move_path", {
            srcPath,
            destPath: dest,
            token,
          });
          if (!res.success) return;
          await fetchTreeCache();
          if (activeFilePath === srcPath) setActiveFilePath(newPath);
        } catch {
          // ignore
        }
        return;
      }

      const ok = fsMovePath(srcPath, destFolderPath);
      if (!ok) return;
      setExpandedFolders((prev) => (prev.includes(destFolderPath) ? prev : [...prev, destFolderPath]));
      if (activeFilePath === srcPath) setActiveFilePath(newPath);
    },
    [useGrpc, token, tauri, fetchTreeCache, activeFilePath]
  );

  const performPaste = useCallback(
    async (destFolder: string) => {
      if (!token || !tauri) return;
      const { items: clipItems, operation } = getClipboard();
      if (clipItems.length === 0) return;
      const dest = destFolder.replace(/\/$/, "");
      for (const item of clipItems) {
        const basename = item.path.split("/").pop() ?? "";
        const destPath = joinPath(dest, basename);
        if (operation === "copy") {
          const res = await invoke<{ success: boolean; error_message: string }>("grpc_copy_path", {
            srcPath: item.path,
            destPath,
            token,
          });
          if (!res.success) continue;
        } else {
          const res = await invoke<{ success: boolean; error_message: string }>("grpc_move_path", {
            srcPath: item.path,
            destPath: dest,
            token,
          });
          if (!res.success) continue;
        }
      }
      if (getClipboard().operation === "cut") clearClipboard();
      await fetchTreeCache();
    },
    [token, tauri, getClipboard, clearClipboard, fetchTreeCache]
  );

  function handleRenameClick(fullPath: string, name: string) {
    setContextMenu(null);
    setRenameTargetPath(fullPath);
    setRenameValue(name);
    setRenameError("");
    setRenameModalOpen(true);
  }

  async function handleRenameSubmit() {
    if (!renameTargetPath) return;
    const newName = renameValue.trim();
    if (!newName) {
      setRenameError("Enter a name.");
      return;
    }
    const parent = getParentPath(renameTargetPath);
    if (parent === null) return;
    const newPath = joinPath(parent, newName);

    if (useGrpc && token && tauri) {
      setRenameError("");
      try {
        const res = await invoke<{ success: boolean; error_message: string }>("grpc_rename_path", {
          path: renameTargetPath,
          newName,
          token,
        });
        if (!res.success) {
          setRenameError(res.error_message || "Failed to rename");
          return;
        }
        setTreeCache((prev) => {
          const list = prev[parent] ?? [];
          const idx = list.findIndex((e) => joinPath(parent, e.name) === renameTargetPath);
          if (idx < 0) return prev;
          const next = [...list];
          next[idx] = { ...next[idx], name: newName };
          return { ...prev, [parent]: next };
        });
        if (activeFilePath === renameTargetPath) setActiveFilePath(newPath);
        setRenameModalOpen(false);
        setRenameTargetPath(null);
        setRenameValue("");
        setRenameError("");
      } catch (e) {
        setRenameError(e instanceof Error ? e.message : String(e));
      }
      return;
    }

    const ok = fsRenamePath(renameTargetPath, newPath);
    if (!ok) {
      setRenameError("Rename failed or name already exists.");
      return;
    }
    if (activeFilePath === renameTargetPath) setActiveFilePath(newPath);
    setRenameModalOpen(false);
    setRenameTargetPath(null);
    setRenameValue("");
    setRenameError("");
  }

  function handleDeleteClick(fullPath: string, name: string) {
    setContextMenu(null);
    setDeleteTargetPath(fullPath);
    setDeleteTargetName(name);
    setDeleteConfirmOpen(true);
  }

  const handleDeleteConfirm = useCallback(async () => {
    if (!deleteTargetPath || !trashPath || !token || !tauri) return;
    const pathToDelete = deleteTargetPath;
    try {
      const res = await invoke<{ success: boolean; error_message: string }>("grpc_move_path", {
        srcPath: pathToDelete,
        destPath: trashPath,
        token,
      });
      if (!res.success) return;
      const parent = getParentPath(pathToDelete);
      if (parent !== null) {
        setTreeCache((prev) => {
          const list = prev[parent] ?? [];
          return { ...prev, [parent]: list.filter((e) => joinPath(parent, e.name) !== pathToDelete) };
        });
      }
      setActiveFilePath((current) => (current === pathToDelete ? null : current));
    } finally {
      setDeleteConfirmOpen(false);
      setDeleteTargetPath(null);
      setDeleteTargetName("");
    }
  }, [deleteTargetPath, trashPath, token, tauri]);

  useEffect(() => {
    if (!deleteConfirmOpen) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Enter") {
        e.preventDefault();
        handleDeleteConfirm();
      }
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [deleteConfirmOpen, handleDeleteConfirm]);

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

  /** Push current content to undo past, clear future, then set new content. Used for all user edits. */
  function pushUndoAndSetContent(newContent: string) {
    setUndoPast((p) => {
      const next = [...p, editorContentRef.current];
      return next.slice(-UNDO_MAX);
    });
    setUndoFuture([]);
    setEditorContent(newContent);
    if (activeFilePath && !useGrpc) setFileContent(activeFilePath, newContent);
  }

  function handleUndo() {
    if (undoPast.length === 0) return;
    const prev = undoPast[undoPast.length - 1];
    setUndoPast((p) => p.slice(0, -1));
    setUndoFuture((f) => [...f, editorContent]);
    setEditorContent(prev);
    if (activeFilePath && !useGrpc) setFileContent(activeFilePath, prev);
  }

  function handleRedo() {
    if (undoFuture.length === 0) return;
    const next = undoFuture[undoFuture.length - 1];
    setUndoFuture((f) => f.slice(0, -1));
    setUndoPast((p) => [...p, editorContent]);
    setEditorContent(next);
    if (activeFilePath && !useGrpc) setFileContent(activeFilePath, next);
  }

  function handleEditorChange(e: React.ChangeEvent<HTMLTextAreaElement>) {
    const value = e.target.value;
    pushUndoAndSetContent(value);
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
    pushUndoAndSetContent(nextContent);
  }

  function handleCut() {
    const ta = editorRef.current;
    if (!ta) return;
    document.execCommand("cut");
    const start = ta.selectionStart;
    const end = ta.selectionEnd;
    const next = editorContent.slice(0, start) + editorContent.slice(end);
    pushUndoAndSetContent(next);
  }

  function handleCopy() {
    document.execCommand("copy");
  }

  async function handlePaste() {
    const ta = editorRef.current;
    if (!ta) return;
    let text = "";
    try {
      text = await navigator.clipboard.readText();
    } catch {
      document.execCommand("paste");
      setTimeout(() => {
        const v = ta.value;
        pushUndoAndSetContent(v);
      }, 0);
      return;
    }
    const start = ta.selectionStart;
    const end = ta.selectionEnd;
    const next = editorContent.slice(0, start) + text + editorContent.slice(end);
    pushUndoAndSetContent(next);
    pendingTabSelectionRef.current = start + text.length;
  }

  function handleSelectAll() {
    const ta = editorRef.current;
    if (!ta) return;
    ta.focus();
    ta.setSelectionRange(0, editorContent.length);
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
      const editorFocused = editorRef.current && document.activeElement === editorRef.current;
      const modalOpen = telescopeOpen;

      if ((e.ctrlKey || e.metaKey) && e.key === "z") {
        if (editorFocused && !modalOpen) {
          e.preventDefault();
          if (e.shiftKey) handleRedo();
          else handleUndo();
        }
        return;
      }
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
      if (!modalOpen && (e.ctrlKey || e.metaKey)) {
        if (e.key === "f") {
          e.preventDefault();
          const sel = editorRef.current
            ? editorContentRef.current.slice(editorRef.current.selectionStart, editorRef.current.selectionEnd)
            : "";
          setTelescopeInitialFind(sel);
          setTelescopeMode("search");
          setTelescopeOpen(true);
        } else if (e.key === "h") {
          e.preventDefault();
          const sel = editorRef.current
            ? editorContentRef.current.slice(editorRef.current.selectionStart, editorRef.current.selectionEnd)
            : "";
          setTelescopeInitialFind(sel);
          setTelescopeMode("findReplace");
          setTelescopeOpen(true);
        } else if (e.key === "p") {
          e.preventDefault();
          setTelescopeInitialFind("");
          setTelescopeMode("findFile");
          setTelescopeOpen(true);
        }
      }
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [handleSave, runScript, telescopeOpen, handleUndo, handleRedo]);

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
                  draggable
                  onDragStart={(e) => {
                    e.dataTransfer.setData(DRAG_PATH_KEY, nodePath);
                    e.dataTransfer.effectAllowed = "move";
                  }}
                  onDragOver={(e) => {
                    e.preventDefault();
                    e.dataTransfer.dropEffect = "move";
                  }}
                  onDrop={(e) => {
                    e.preventDefault();
                    const src = e.dataTransfer.getData(DRAG_PATH_KEY);
                    if (src) handleDropMove(src, nodePath);
                  }}
                  onClick={() => toggleExpanded(nodePath)}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    setContextMenu({
                      type: "node",
                      x: e.clientX,
                      y: e.clientY,
                      folderPath: path,
                      fullPath: nodePath,
                      nodeType: "folder",
                      name: node.name,
                    });
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
              draggable
              onDragStart={(e) => {
                e.dataTransfer.setData(DRAG_PATH_KEY, nodePath);
                e.dataTransfer.effectAllowed = "move";
              }}
              onClick={() => setActiveFilePath(nodePath)}
              onContextMenu={(e) => {
                e.preventDefault();
                e.stopPropagation();
                setContextMenu({
                  type: "node",
                  x: e.clientX,
                  y: e.clientY,
                  folderPath: path,
                  fullPath: nodePath,
                  nodeType: "file",
                  name: node.name,
                });
              }}
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
              <button
                type="button"
                className={styles.menuDropdownItem}
                onClick={() => {
                  setFileMenuOpen(false);
                  openNewFolderModalInFolder(rootPath ?? getHomePath());
                }}
              >
                <span className={styles.menuItemLabel}>New Folder</span>
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
              <button type="button" className={styles.menuDropdownItem} onClick={() => { handleUndo(); setEditMenuOpen(false); }} disabled={undoPast.length === 0}>
                <span className={styles.menuItemLabel}>Undo</span>
                <span className={styles.menuItemShortcut}>Ctrl+Z</span>
              </button>
              <button type="button" className={styles.menuDropdownItem} onClick={() => { handleRedo(); setEditMenuOpen(false); }} disabled={undoFuture.length === 0}>
                <span className={styles.menuItemLabel}>Redo</span>
                <span className={styles.menuItemShortcut}>Ctrl+Shift+Z</span>
              </button>
              <div className={styles.menuDropdownSep} />
              <button type="button" className={styles.menuDropdownItem} onClick={() => { handleCut(); setEditMenuOpen(false); }}>
                <span className={styles.menuItemLabel}>Cut</span>
                <span className={styles.menuItemShortcut}>Ctrl+X</span>
              </button>
              <button type="button" className={styles.menuDropdownItem} onClick={() => { handleCopy(); setEditMenuOpen(false); }}>
                <span className={styles.menuItemLabel}>Copy</span>
                <span className={styles.menuItemShortcut}>Ctrl+C</span>
              </button>
              <button type="button" className={styles.menuDropdownItem} onClick={() => { handlePaste(); setEditMenuOpen(false); }}>
                <span className={styles.menuItemLabel}>Paste</span>
                <span className={styles.menuItemShortcut}>Ctrl+V</span>
              </button>
              <div className={styles.menuDropdownSep} />
              <button type="button" className={styles.menuDropdownItem} onClick={() => { setTelescopeInitialFind(editorRef.current ? editorContent.slice(editorRef.current.selectionStart, editorRef.current.selectionEnd) : ""); setTelescopeMode("search"); setTelescopeOpen(true); setEditMenuOpen(false); }}>
                <span className={styles.menuItemLabel}>Find</span>
                <span className={styles.menuItemShortcut}>Ctrl+F</span>
              </button>
              <button type="button" className={styles.menuDropdownItem} onClick={() => { setTelescopeInitialFind(editorRef.current ? editorContent.slice(editorRef.current.selectionStart, editorRef.current.selectionEnd) : ""); setTelescopeMode("findReplace"); setTelescopeOpen(true); setEditMenuOpen(false); }}>
                <span className={styles.menuItemLabel}>Find and Replace</span>
                <span className={styles.menuItemShortcut}>Ctrl+H</span>
              </button>
              <button type="button" className={styles.menuDropdownItem} onClick={() => { setTelescopeInitialFind(""); setTelescopeMode("findFile"); setTelescopeOpen(true); setEditMenuOpen(false); }}>
                <span className={styles.menuItemLabel}>Find File</span>
                <span className={styles.menuItemShortcut}>Ctrl+P</span>
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
              setEditMenuOpen(false);
              setSelectionMenuOpen((o) => !o);
              setViewMenuOpen(false);
            }}
          >
            Selection
          </button>
          {selectionMenuOpen && (
            <div className={styles.menuDropdown}>
              <button type="button" className={styles.menuDropdownItem} onClick={() => { handleSelectAll(); setSelectionMenuOpen(false); }}>
                <span className={styles.menuItemLabel}>Select All</span>
                <span className={styles.menuItemShortcut}>Ctrl+A</span>
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
              setEditMenuOpen(false);
              setSelectionMenuOpen(false);
              setViewMenuOpen((o) => !o);
            }}
          >
            View
          </button>
          {viewMenuOpen && (
            <div className={styles.menuDropdown}>
              <button
                type="button"
                className={styles.menuDropdownItem}
                onClick={() => { setConsoleVisible((v) => !v); setViewMenuOpen(false); }}
              >
                <span className={styles.menuItemLabel}>Toggle Console</span>
              </button>
              <button
                type="button"
                className={styles.menuDropdownItem}
                onClick={() => { setSidebarVisible((v) => !v); setViewMenuOpen(false); }}
              >
                <span className={styles.menuItemLabel}>Toggle Sidebar</span>
              </button>
            </div>
          )}
        </div>
      </div>

      <TelescopeModal
        open={telescopeOpen}
        onClose={() => setTelescopeOpen(false)}
        mode={telescopeMode}
        rootPath={rootPath}
        token={token}
        initialFindValue={telescopeInitialFind}
        onOpenInEditor={(path, line) => {
          setActiveFilePath(path);
          if (line != null) pendingGoToLineRef.current = { path, line };
          setTelescopeOpen(false);
        }}
      />

      <div className={styles.body}>
        {sidebarVisible && (
        <aside className={styles.sidebar} onClick={() => setContextMenu(null)}>
          {rootPath !== null && (
            <div className={styles.sidebarProjectHeader} title={rootPath}>
              <span className={styles.sidebarProjectLabel}>Project</span>
              <span className={styles.sidebarProjectPath}>{rootPath}</span>
            </div>
          )}
          <div
            className={styles.tree}
            onContextMenu={
              rootPath !== null
                ? (e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    if ((e.target as HTMLElement).closest?.("button[data-type]")) return;
                    setContextMenu({ type: "background", x: e.clientX, y: e.clientY, folderPath: rootPath });
                  }
                : undefined
            }
            onDragOver={
              rootPath !== null
                ? (e) => {
                    e.preventDefault();
                    e.dataTransfer.dropEffect = "move";
                  }
                : undefined
            }
            onDrop={
              rootPath !== null
                ? (e) => {
                    e.preventDefault();
                    const src = e.dataTransfer.getData(DRAG_PATH_KEY);
                    if (src) handleDropMove(src, rootPath);
                  }
                : undefined
            }
          >
            {rootPath !== null ? (
              renderTree(rootPath, 0)
            ) : (
              <div className={styles.sidebarHint}>Open a folder from File menu</div>
            )}
          </div>
        </aside>
        )}
        {contextMenu && (
          <ContextMenu
            x={contextMenu.x}
            y={contextMenu.y}
            items={(() => {
              const items: { label: string; onClick: () => void; disabled?: boolean }[] = [];
              if (contextMenu.type === "background") {
                items.push(
                  { label: "New File", onClick: () => openNewFileModalInFolder(contextMenu.folderPath) },
                  { label: "New Folder", onClick: () => openNewFolderModalInFolder(contextMenu.folderPath) }
                );
                if (clipboardHasItems && useGrpc) {
                  items.push({ label: "Paste", onClick: () => performPaste(contextMenu.folderPath) });
                }
              } else {
                if (contextMenu.nodeType === "folder") {
                  items.push(
                    { label: "New File", onClick: () => openNewFileModalInFolder(contextMenu.fullPath) },
                    { label: "New Folder", onClick: () => openNewFolderModalInFolder(contextMenu.fullPath) }
                  );
                }
                items.push(
                  { label: "Copy", onClick: () => setClipboard([{ path: contextMenu.fullPath, type: contextMenu.nodeType }], "copy") },
                  { label: "Cut", onClick: () => setClipboard([{ path: contextMenu.fullPath, type: contextMenu.nodeType }], "cut") }
                );
                if (clipboardHasItems && useGrpc) {
                  items.push({ label: "Paste", onClick: () => performPaste(contextMenu.folderPath) });
                }
                items.push({ label: "Rename", onClick: () => handleRenameClick(contextMenu.fullPath, contextMenu.name) });
                if (trashPath && contextMenu.fullPath !== trashPath && useGrpc) {
                  items.push({ label: "Delete", onClick: () => handleDeleteClick(contextMenu.fullPath, contextMenu.name) });
                }
              }
              return items;
            })()}
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
              {recentFolders.length > 0 && (
                <div className={styles.welcomeRecent}>
                  <span className={styles.welcomeRecentLabel}>Reopen folder:</span>
                  <div className={styles.welcomeRecentList}>
                    {recentFolders.map((path) => (
                      <button
                        key={path}
                        type="button"
                        className={styles.welcomeRecentBtn}
                        onClick={() => handleOpenFolder(path)}
                        title={path}
                      >
                        {path.split("/").filter(Boolean).pop() || path || "/"}
                      </button>
                    ))}
                  </div>
                </div>
              )}
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
                {consoleVisible && (
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
                )}
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

      <Modal
        open={newFolderModalOpen}
        onClose={() => setNewFolderModalOpen(false)}
        title="New Folder"
        primaryButton={{ label: "Create", onClick: handleNewFolderCreate }}
        secondaryButton={{ label: "Cancel", onClick: () => setNewFolderModalOpen(false) }}
      >
        <div className={styles.modalContent}>
          <label className={styles.modalLabel} htmlFor="new-folder-name">
            Folder name
          </label>
          <input
            id="new-folder-name"
            type="text"
            className={styles.modalInput}
            value={newFolderName}
            onChange={(e) => {
              setNewFolderName(e.target.value);
              setNewFolderError("");
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                handleNewFolderCreate();
              }
            }}
            placeholder="e.g. src"
            autoFocus
          />
          {newFolderError && <p className={styles.modalError}>{newFolderError}</p>}
        </div>
      </Modal>

      <Modal
        open={renameModalOpen}
        onClose={() => setRenameModalOpen(false)}
        title="Rename"
        primaryButton={{ label: "Rename", onClick: handleRenameSubmit }}
        secondaryButton={{ label: "Cancel", onClick: () => setRenameModalOpen(false) }}
      >
        <div className={styles.modalContent}>
          <label className={styles.modalLabel} htmlFor="rename-input">
            Name
          </label>
          <input
            id="rename-input"
            type="text"
            className={styles.modalInput}
            value={renameValue}
            onChange={(e) => {
              setRenameValue(e.target.value);
              setRenameError("");
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                handleRenameSubmit();
              }
            }}
            autoFocus
          />
          {renameError && <p className={styles.modalError}>{renameError}</p>}
        </div>
      </Modal>

      <Modal
        open={deleteConfirmOpen}
        onClose={() => setDeleteConfirmOpen(false)}
        title="Delete"
        primaryButton={{ label: "Delete", onClick: handleDeleteConfirm }}
        secondaryButton={{ label: "Cancel", onClick: () => setDeleteConfirmOpen(false) }}
      >
        <div className={styles.modalContent}>
          <p className={styles.modalLabel}>
            Delete {deleteTargetName}? It will be moved to Trash.
          </p>
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
