import { useState, useEffect, useCallback, useRef } from "react";
import { createPortal } from "react-dom";
import { useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { Trash2 } from "lucide-react";
import { useAuth } from "../contexts/AuthContext";
import { useClipboard } from "../contexts/ClipboardContext";
import ContextMenu, { type ContextMenuItem } from "./ContextMenu";
import Modal from "./Modal";
import styles from "./Explorer.module.css";

function getParentPath(path: string, homePath: string): string | null {
  const normalized = path.replace(/\/+/g, "/").replace(/\/$/, "") || "";
  if (!normalized) return null;
  if (normalized === homePath || normalized === homePath.trimEnd()) return null;
  const parts = normalized.split("/");
  parts.pop();
  const parent = parts.length === 0 ? "/" : "/" + parts.join("/");
  return parent;
}

function pathToBreadcrumb(path: string, homePath: string): string[] {
  const normalized = path.replace(/\/+/g, "/").replace(/^\//, "").replace(/\/$/, "") || "";
  if (!normalized) return [homePath.split("/").pop() || "Home"];
  return normalized.split("/");
}

interface FsEntry {
  name: string;
  node_type: string;
  size_bytes: number;
}

export default function Explorer() {
  const { playerId, token, logout } = useAuth();
  const navigate = useNavigate();
  const { setClipboard, getClipboard, clearClipboard, hasItems } = useClipboard();
  const [homePath, setHomePath] = useState<string | null>(null);
  const [currentPath, setCurrentPath] = useState<string>("/home/user");
  const [entries, setEntries] = useState<FsEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());
  const lastClickedIndex = useRef<number | null>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const [contextMenu, setContextMenu] = useState<
    | { x: number; y: number; entry: FsEntry; fullPath: string }
    | { x: number; y: number; type: "background" }
    | null
  >(null);
  const [renameModal, setRenameModal] = useState<{ path: string; currentName: string } | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [renameLoading, setRenameLoading] = useState(false);
  const [conflict, setConflict] = useState<{
    item: { path: string; type: "file" | "folder" };
    destName: string;
    existingType: string;
    canReplace: boolean;
    resolve: (choice: "replace" | "skip" | "cancel", replaceAll?: boolean) => void;
  } | null>(null);
  const [replaceAll, setReplaceAll] = useState(false);
  const replaceAllCheckRef = useRef<HTMLInputElement>(null);
  const [createFileModal, setCreateFileModal] = useState(false);
  const [createFileName, setCreateFileName] = useState("");
  const [createFileLoading, setCreateFileLoading] = useState(false);
  const [emptyTrashConfirmOpen, setEmptyTrashConfirmOpen] = useState(false);
  const [emptyTrashLoading, setEmptyTrashLoading] = useState(false);

  useEffect(() => {
    listRef.current?.focus();
  }, [currentPath]);

  const tauri = typeof window !== "undefined" && (window as unknown as { __TAURI__?: unknown }).__TAURI__;

  const fetchHomePath = useCallback(async () => {
    if (!playerId || !token || !tauri) return;
    try {
      const res = await invoke<{ home_path: string; error_message: string }>(
        "grpc_get_home_path",
        { token }
      );
      if (res.error_message) {
        if (res.error_message === "UNAUTHENTICATED") {
          logout();
          navigate("/login");
          return;
        }
        setError(res.error_message);
      } else {
        setHomePath(res.home_path);
        setCurrentPath(res.home_path);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [playerId, token, tauri, logout, navigate]);

  const fetchEntries = useCallback(async () => {
    if (!playerId || !token || !tauri || !currentPath) return;
    setLoading(true);
    setError(null);
    try {
      const res = await invoke<{ entries: FsEntry[]; error_message: string }>(
        "grpc_list_fs",
        { path: currentPath, token }
      );
      if (res.error_message) {
        if (res.error_message === "UNAUTHENTICATED") {
          logout();
          navigate("/login");
          return;
        }
        setError(res.error_message);
        setEntries([]);
      } else {
        setEntries(res.entries);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setEntries([]);
    } finally {
      setLoading(false);
    }
  }, [playerId, token, currentPath, tauri, logout, navigate]);

  useEffect(() => {
    fetchHomePath();
  }, [fetchHomePath]);

  useEffect(() => {
    if (homePath && currentPath) {
      fetchEntries();
    } else {
      setEntries([]);
    }
  }, [homePath, currentPath, fetchEntries]);

  useEffect(() => {
    if (renameModal) setRenameValue(renameModal.currentName);
  }, [renameModal]);

  useEffect(() => {
    if (createFileModal) setCreateFileName("");
  }, [createFileModal]);

  const parentPath = homePath ? getParentPath(currentPath, homePath) : null;
  const breadcrumb = homePath ? pathToBreadcrumb(currentPath, homePath) : [];
  const trashPath = homePath ? `${homePath.replace(/\/$/, "")}/Trash` : null;

  const places = homePath
    ? [
        { label: "Home", path: homePath },
        { label: "Documents", path: `${homePath}/Documents` },
        { label: "Downloads", path: `${homePath}/Downloads` },
        { label: "Images", path: `${homePath}/Images` },
        ...(trashPath ? [{ label: "Trash", path: trashPath }] : []),
      ]
    : [];

  function handleBack() {
    if (parentPath !== null) setCurrentPath(parentPath);
  }

  function handlePlace(path: string) {
    setCurrentPath(path);
  }

  function handleOpen(entry: FsEntry) {
    if (entry.node_type === "directory") {
      const newPath = currentPath.replace(/\/$/, "") + "/" + entry.name;
      setCurrentPath(newPath);
    }
  }

  function handleContextMenu(entry: FsEntry, fullPath: string, e: React.MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY, entry, fullPath });
  }

  function handleRenameClick(entry: FsEntry, fullPath: string) {
    setContextMenu(null);
    setRenameModal({ path: fullPath, currentName: entry.name });
    setRenameValue(entry.name);
  }

  async function handleRenameSubmit() {
    if (!renameModal || !playerId || !token || !tauri || !renameValue.trim()) return;
    const trimmed = renameValue.trim();
    if (trimmed === renameModal.currentName) {
      setRenameModal(null);
      return;
    }
    setRenameLoading(true);
    setError(null);
    try {
      const res = await invoke<{ success: boolean; error_message: string }>(
        "grpc_rename_path",
        { path: renameModal.path, newName: trimmed, token }
      );
      if (!res.success) {
        if (res.error_message === "UNAUTHENTICATED") {
          logout();
          navigate("/login");
          return;
        }
        setError(res.error_message);
        setRenameLoading(false);
        return;
      }
      await fetchEntries();
      setRenameModal(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setRenameLoading(false);
    }
  }

  async function handleDeleteClick(fullPath: string) {
    setContextMenu(null);
    if (!playerId || !token || !tauri || !trashPath) return;
    setError(null);
    try {
      const res = await invoke<{ success: boolean; error_message: string }>(
        "grpc_move_path",
        { srcPath: fullPath, destPath: trashPath, token }
      );
      if (!res.success) {
        if (res.error_message === "UNAUTHENTICATED") {
          logout();
          navigate("/login");
          return;
        }
        setError(res.error_message);
        return;
      }
      await fetchEntries();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  async function handleEmptyTrashConfirm() {
    if (!playerId || !token || !tauri) return;
    setEmptyTrashLoading(true);
    setError(null);
    try {
      const res = await invoke<{ success: boolean; error_message: string }>(
        "grpc_empty_trash",
        { token }
      );
      if (!res.success) {
        if (res.error_message === "UNAUTHENTICATED") {
          logout();
          navigate("/login");
          return;
        }
        setError(res.error_message);
        setEmptyTrashLoading(false);
        return;
      }
      setEmptyTrashConfirmOpen(false);
      await fetchEntries();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setEmptyTrashLoading(false);
    }
  }

  async function handleCreateFileSubmit() {
    if (!playerId || !token || !tauri || !currentPath) return;
    const trimmed = createFileName.trim();
    if (!trimmed) return;
    if (trimmed.includes("/")) {
      setError("File name cannot contain /");
      return;
    }
    setCreateFileLoading(true);
    setError(null);
    const path = currentPath.replace(/\/$/, "") + "/" + trimmed;
    try {
      const res = await invoke<{ success: boolean; error_message: string }>(
        "grpc_write_file",
        { path, content: "", token }
      );
      if (!res.success) {
        if (res.error_message === "UNAUTHENTICATED") {
          logout();
          navigate("/login");
          return;
        }
        setError(res.error_message);
        setCreateFileLoading(false);
        return;
      }
      await fetchEntries();
      setCreateFileModal(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setCreateFileLoading(false);
    }
  }

  function handleBreadcrumbClick(index: number) {
    if (index === 0 && homePath) {
      setCurrentPath(homePath);
      return;
    }
    const segs = breadcrumb.slice(0, index + 1);
    setCurrentPath("/" + segs.join("/"));
  }

  const DRAG_PATH_KEY = "application/x-nulltrace-path";

  const handleDropMove = useCallback(
    async (srcPath: string, destFolderPath: string) => {
      if (srcPath === destFolderPath) return;
      if (destFolderPath.startsWith(srcPath + "/")) return;
      if (!token || !tauri) return;
      const dest = destFolderPath.replace(/\/$/, "");
      try {
        const res = await invoke<{ success: boolean; error_message: string }>("grpc_move_path", {
          srcPath,
          destPath: dest,
          token,
        });
        if (!res.success) {
          if (res.error_message === "UNAUTHENTICATED") {
            logout();
            navigate("/login");
            return;
          }
          setError(res.error_message);
          return;
        }
        await fetchEntries();
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    },
    [token, tauri, fetchEntries, logout, navigate]
  );

  const performPaste = useCallback(
    async (items: { path: string; type: "file" | "folder" }[], operation: "copy" | "cut") => {
      if (!playerId || !tauri) return;
      const destFolder = currentPath.replace(/\/$/, "");
      for (const item of items) {
        const basename = item.path.split("/").pop() ?? "";
        const existing = entries.find((e) => e.name === basename);
        if (existing) {
          const canReplace = item.type === "file" && existing.node_type === "file";
          if (!canReplace) {
            const choice = await new Promise<"skip" | "cancel">((resolve) => {
              setConflict({
                item,
                destName: basename,
                existingType: existing.node_type,
                canReplace: false,
                resolve: (c) => {
                  setConflict(null);
                  resolve(c as "skip" | "cancel");
                },
              });
            });
            if (choice === "cancel") break;
            continue;
          }
          if (!replaceAll) {
            const result = await new Promise<{
              choice: "replace" | "skip" | "cancel";
              replaceAll: boolean;
            }>((resolve) => {
              setConflict({
                item,
                destName: basename,
                existingType: existing.node_type,
                canReplace: true,
                resolve: (c, r) => {
                  setConflict(null);
                  resolve({ choice: c, replaceAll: r ?? false });
                },
              });
            });
            if (result.choice === "cancel") break;
            if (result.choice === "skip") continue;
            if (result.replaceAll) setReplaceAll(true);
          }
        }
        if (operation === "copy") {
          const destPath = `${destFolder}/${basename}`;
          const res = await invoke<{ success: boolean; error_message: string }>(
            "grpc_copy_path",
            { srcPath: item.path, destPath, token }
          );
          if (!res.success) {
            if (res.error_message === "UNAUTHENTICATED") {
              logout();
              navigate("/login");
              return;
            }
            setError(res.error_message);
          }
        } else {
          const res = await invoke<{ success: boolean; error_message: string }>(
            "grpc_move_path",
            { srcPath: item.path, destPath: destFolder, token }
          );
          if (!res.success) {
            if (res.error_message === "UNAUTHENTICATED") {
              logout();
              navigate("/login");
              return;
            }
            setError(res.error_message);
          }
        }
      }
      if (operation === "cut") clearClipboard();
      setReplaceAll(false);
      await fetchEntries();
    },
    [playerId, token, currentPath, tauri, entries, fetchEntries, clearClipboard, logout, navigate]
  );

  function handleRowClick(entry: FsEntry, index: number, e: React.MouseEvent) {
    const fullPath = currentPath.replace(/\/$/, "") + "/" + entry.name;
    if (e.ctrlKey) {
      setSelectedPaths((prev) => {
        const next = new Set(prev);
        if (next.has(fullPath)) next.delete(fullPath);
        else next.add(fullPath);
        return next;
      });
    } else if (e.shiftKey) {
      const start = lastClickedIndex.current ?? index;
      const [lo, hi] = [Math.min(start, index), Math.max(start, index)];
      setSelectedPaths((prev) => {
        const next = new Set(prev);
        for (let i = lo; i <= hi; i++) {
          const ent = entries[i];
          if (ent) next.add(currentPath.replace(/\/$/, "") + "/" + ent.name);
        }
        return next;
      });
    } else {
      setSelectedPaths(new Set([fullPath]));
    }
    lastClickedIndex.current = index;
  }

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.ctrlKey || e.metaKey) {
        if (e.key === "c") {
          e.preventDefault();
          e.stopPropagation();
          if (selectedPaths.size > 0) {
            const items = Array.from(selectedPaths).map((path) => {
              const ent = entries.find((x) => path.endsWith("/" + x.name));
              return {
                path,
                type: (ent?.node_type === "directory" ? "folder" : "file") as "file" | "folder",
              };
            });
            setClipboard(items, "copy");
          }
        } else if (e.key === "x") {
          e.preventDefault();
          e.stopPropagation();
          if (selectedPaths.size > 0) {
            const items = Array.from(selectedPaths).map((path) => {
              const ent = entries.find((x) => path.endsWith("/" + x.name));
              return {
                path,
                type: (ent?.node_type === "directory" ? "folder" : "file") as "file" | "folder",
              };
            });
            setClipboard(items, "cut");
          }
        } else if (e.key === "v") {
          e.preventDefault();
          e.stopPropagation();
          if (!hasItems || !playerId || !tauri) return;
          const { items, operation } = getClipboard();
          if (items.length === 0) return;
          performPaste(items, operation);
        }
      }
    },
    [selectedPaths, entries, hasItems, getClipboard, setClipboard, performPaste]
  );

  if (!playerId || !tauri) {
    return (
      <div className={styles.app}>
        <div className={styles.empty}>Log in with the desktop app to view files.</div>
      </div>
    );
  }

  if (!homePath) {
    return (
      <div className={styles.app}>
        <div className={styles.empty}>{error ?? "Loading…"}</div>
      </div>
    );
  }

  return (
    <div className={styles.app}>
      <aside className={styles.sidebar} onClick={() => setContextMenu(null)}>
        <div className={styles.sidebarSection}>Places</div>
        {places.map(({ label, path }) => (
          <button
            key={path}
            type="button"
            className={`${styles.place} ${currentPath === path ? styles.placeActive : ""}`}
            onClick={() => handlePlace(path)}
          >
            <span className={styles.placeIcon}>
              {path === trashPath ? (
                <Trash2 size={20} />
              ) : path === homePath ? (
                <HomeIcon />
              ) : (
                <FolderIcon />
              )}
            </span>
            <span className={styles.placeLabel}>{label}</span>
          </button>
        ))}
      </aside>
      <div className={styles.main}>
        <div className={styles.toolbar} onClick={() => setContextMenu(null)}>
          <button
            type="button"
            className={styles.backBtn}
            onClick={handleBack}
            disabled={parentPath === null}
            title="Back"
          >
            <BackIcon />
          </button>
          <div className={styles.breadcrumb}>
            {breadcrumb.map((seg, i) => (
              <span key={i}>
                {i > 0 && <span className={styles.breadcrumbSep}>/</span>}
                <button
                  type="button"
                  className={styles.breadcrumbPart}
                  onClick={() => handleBreadcrumbClick(i)}
                >
                  {seg || "Home"}
                </button>
              </span>
            ))}
          </div>
          {currentPath === trashPath && (
            <button
              type="button"
              className={styles.emptyTrashBtn}
              onClick={() => setEmptyTrashConfirmOpen(true)}
              title="Empty trash"
            >
              Empty trash
            </button>
          )}
        </div>
        <div
          ref={listRef}
          className={styles.list}
          tabIndex={0}
          onClick={() => setContextMenu(null)}
          onKeyDown={handleKeyDown}
          onContextMenu={(e) => {
            e.preventDefault();
            setContextMenu({ x: e.clientX, y: e.clientY, type: "background" });
          }}
          onDragOver={(e) => {
            e.preventDefault();
            e.dataTransfer.dropEffect = "move";
          }}
          onDrop={(e) => {
            e.preventDefault();
            const src = e.dataTransfer.getData(DRAG_PATH_KEY);
            if (src) handleDropMove(src, currentPath.replace(/\/$/, ""));
          }}
          role="list"
        >
          {loading ? (
            <div className={styles.empty}>Loading…</div>
          ) : error ? (
            <div className={styles.empty}>{error}</div>
          ) : entries.length === 0 ? (
            <div className={styles.empty}>This folder is empty</div>
          ) : (
            entries.map((entry, index) => {
              const fullPath = currentPath.replace(/\/$/, "") + "/" + entry.name;
              const isSelected = selectedPaths.has(fullPath);
              const isDirectory = entry.node_type === "directory";
              return (
                <div
                  key={entry.name}
                  role="button"
                  tabIndex={-1}
                  className={`${styles.row} ${isSelected ? styles.rowSelected : ""}`}
                  data-type={entry.node_type}
                  draggable
                  onDragStart={(e) => {
                    e.dataTransfer.setData(DRAG_PATH_KEY, fullPath);
                    e.dataTransfer.effectAllowed = "move";
                  }}
                  onDragOver={isDirectory ? (e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    e.dataTransfer.dropEffect = "move";
                  } : undefined}
                  onDrop={isDirectory ? (e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    const src = e.dataTransfer.getData(DRAG_PATH_KEY);
                    if (src) handleDropMove(src, fullPath);
                  } : undefined}
                  onClick={(e) => handleRowClick(entry, index, e)}
                  onDoubleClick={() => handleOpen(entry)}
                  onContextMenu={(e) => handleContextMenu(entry, fullPath, e)}
                >
                  <span className={styles.rowIcon}>
                    {isDirectory ? <FolderIcon /> : <FileIcon />}
                  </span>
                  <span className={styles.rowName}>{entry.name}</span>
                </div>
              );
            })
          )}
        </div>
      </div>

      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={buildContextMenuItems(contextMenu)}
          onClose={() => setContextMenu(null)}
        />
      )}

      {createPortal(
        <Modal
          open={!!renameModal}
          onClose={() => !renameLoading && setRenameModal(null)}
          title="Rename"
          primaryButton={{
            label: renameLoading ? "Renaming…" : "Rename",
            onClick: handleRenameSubmit,
            disabled: renameLoading,
          }}
          secondaryButton={{
            label: "Cancel",
            onClick: () => setRenameModal(null),
            disabled: renameLoading,
          }}
        >
          {renameModal && (
            <label className={styles.renameLabel}>
              New name
              <input
                type="text"
                className={styles.renameInput}
                value={renameValue}
                onChange={(e) => setRenameValue(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleRenameSubmit();
                  if (e.key === "Escape") setRenameModal(null);
                }}
                disabled={renameLoading}
                autoFocus
              />
            </label>
          )}
        </Modal>,
        document.body
      )}

      {createPortal(
        <Modal
          open={createFileModal}
          onClose={() => !createFileLoading && setCreateFileModal(false)}
          title="New file"
          primaryButton={{
            label: createFileLoading ? "Creating…" : "Create",
            onClick: handleCreateFileSubmit,
            disabled: createFileLoading || !createFileName.trim(),
          }}
          secondaryButton={{
            label: "Cancel",
            onClick: () => setCreateFileModal(false),
            disabled: createFileLoading,
          }}
        >
          <label className={styles.renameLabel}>
            File name
            <input
              type="text"
              className={styles.renameInput}
              value={createFileName}
              onChange={(e) => setCreateFileName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleCreateFileSubmit();
                if (e.key === "Escape") setCreateFileModal(false);
              }}
              disabled={createFileLoading}
              autoFocus
              placeholder="e.g. notes.txt or script.lua"
            />
          </label>
        </Modal>,
        document.body
      )}

      {conflict && (
        <Modal
          open
          onClose={() => conflict.resolve("cancel")}
          title={
            conflict.canReplace
              ? `Replace "${conflict.destName}"?`
              : `Cannot replace "${conflict.destName}"`
          }
          primaryButton={
            conflict.canReplace
              ? {
                  label: "Replace",
                  onClick: () =>
                    conflict.resolve("replace", replaceAllCheckRef.current?.checked ?? false),
                }
              : { label: "Skip", onClick: () => conflict.resolve("skip") }
          }
          secondaryButton={{ label: "Cancel", onClick: () => conflict.resolve("cancel") }}
          tertiaryButton={
            conflict.canReplace ? { label: "Skip", onClick: () => conflict.resolve("skip") } : undefined
          }
        >
          <p className={styles.conflictMessage}>
            {conflict.canReplace
              ? `A file with this name already exists. Replace it?`
              : `A folder with this name already exists. Skip it?`}
          </p>
          {conflict.canReplace && (
            <label className={styles.replaceAllLabel}>
              <input ref={replaceAllCheckRef} type="checkbox" /> Replace all
            </label>
          )}
        </Modal>
      )}

      {createPortal(
        <Modal
          open={emptyTrashConfirmOpen}
          onClose={() => !emptyTrashLoading && setEmptyTrashConfirmOpen(false)}
          title="Empty trash"
          primaryButton={{
            label: emptyTrashLoading ? "Emptying…" : "Empty trash",
            onClick: handleEmptyTrashConfirm,
            disabled: emptyTrashLoading,
          }}
          secondaryButton={{
            label: "Cancel",
            onClick: () => setEmptyTrashConfirmOpen(false),
            disabled: emptyTrashLoading,
          }}
        >
          <p className={styles.conflictMessage}>
            Permanently delete all items in Trash? This cannot be undone.
          </p>
        </Modal>,
        document.body
      )}
    </div>
  );

  function buildContextMenuItems(ctx: NonNullable<typeof contextMenu>): ContextMenuItem[] {
    const items: ContextMenuItem[] = [];
    if ("fullPath" in ctx && ctx.entry && ctx.entry.name) {
      items.push(
        { label: "Copy", onClick: () => setClipboard([{ path: ctx.fullPath, type: ctx.entry.node_type === "directory" ? "folder" : "file" }], "copy") },
        { label: "Cut", onClick: () => setClipboard([{ path: ctx.fullPath, type: ctx.entry.node_type === "directory" ? "folder" : "file" }], "cut") },
      );
      if (hasItems) {
        items.push({
          label: "Paste",
          onClick: () => {
            const { items: clipItems, operation } = getClipboard();
            if (clipItems.length > 0) performPaste(clipItems, operation);
          },
        });
      }
      items.push({ label: "Rename", onClick: () => handleRenameClick(ctx.entry, ctx.fullPath) });
      if (trashPath && ctx.fullPath !== trashPath) {
        items.push({ label: "Delete", onClick: () => handleDeleteClick(ctx.fullPath) });
      }
    } else if ("type" in ctx && ctx.type === "background") {
      items.push({
        label: "Create file",
        onClick: () => {
          setContextMenu(null);
          setCreateFileModal(true);
        },
      });
      if (hasItems) {
        items.push({
          label: "Paste",
          onClick: () => {
            const { items: clipItems, operation } = getClipboard();
            if (clipItems.length > 0) performPaste(clipItems, operation);
          },
        });
      }
    }
    return items;
  }
}

function FolderIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
      <polyline points="2 9 2 5 8 5 10 9 22 9" />
    </svg>
  );
}

function FileIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
      <polyline points="14 2 14 8 20 8" />
      <line x1="16" y1="13" x2="8" y2="13" />
      <line x1="16" y1="17" x2="8" y2="17" />
      <polyline points="10 9 9 9 8 9" />
    </svg>
  );
}

function HomeIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
      <polyline points="9 22 9 12 15 12 15 22" />
    </svg>
  );
}

function BackIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <line x1="19" y1="12" x2="5" y2="12" />
      <polyline points="12 19 5 12 12 5" />
    </svg>
  );
}
