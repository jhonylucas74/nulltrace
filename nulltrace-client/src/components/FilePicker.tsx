import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAuth } from "../contexts/AuthContext";
import { getChildren, getParentPath, getHomePath } from "../lib/fileSystem";
import type { FileSystemNode } from "../lib/fileSystem";
import type { FilePickerMode } from "../contexts/FilePickerContext";
import styles from "./FilePicker.module.css";

function joinPath(base: string, name: string): string {
  const b = base.replace(/\/$/, "");
  return b ? `${b}/${name}` : `/${name}`;
}

export interface FilePickerProps {
  open: boolean;
  mode: FilePickerMode;
  initialPath: string;
  onSelect: (path: string) => void;
  onCancel: () => void;
}

interface FsEntry {
  name: string;
  node_type: string;
  size_bytes: number;
}

export default function FilePicker({
  open,
  mode,
  initialPath,
  onSelect,
  onCancel,
}: FilePickerProps) {
  const { token } = useAuth();
  const [currentPath, setCurrentPath] = useState(initialPath || getHomePath());
  const [homePath, setHomePath] = useState<string | null>(null);
  const [grpcEntries, setGrpcEntries] = useState<FsEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const tauri = typeof window !== "undefined" && (window as unknown as { __TAURI__?: unknown }).__TAURI__;
  const useGrpc = !!token && !!tauri;

  const fetchHomePath = useCallback(async () => {
    if (!token || !tauri) return;
    try {
      const res = await invoke<{ home_path: string; error_message: string }>(
        "grpc_get_home_path",
        { token }
      );
      if (!res.error_message) setHomePath(res.home_path);
    } catch {
      // Ignore
    }
  }, [token, tauri]);

  const fetchEntries = useCallback(async () => {
    if (!token || !tauri || !currentPath) return;
    setLoading(true);
    setError(null);
    try {
      const res = await invoke<{ entries: FsEntry[]; error_message: string }>(
        "grpc_list_fs",
        { path: currentPath, token }
      );
      if (res.error_message) {
        setError(res.error_message);
        setGrpcEntries([]);
      } else {
        setGrpcEntries(res.entries);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setGrpcEntries([]);
    } finally {
      setLoading(false);
    }
  }, [token, currentPath, tauri]);

  useEffect(() => {
    if (open) {
      setCurrentPath(initialPath || getHomePath());
      if (useGrpc) {
        fetchHomePath();
      }
    }
  }, [open, initialPath, useGrpc, fetchHomePath]);

  useEffect(() => {
    if (open && useGrpc && homePath) {
      setCurrentPath((p) => (p === getHomePath() ? homePath : p));
    }
  }, [open, useGrpc, homePath]);

  useEffect(() => {
    if (open && useGrpc && currentPath) {
      fetchEntries();
    }
  }, [open, useGrpc, currentPath, fetchEntries]);

  const parentPath = useGrpc && homePath
    ? (() => {
        const norm = currentPath.replace(/\/+/g, "/").replace(/\/$/, "");
        const homeNorm = homePath.replace(/\/+/g, "/").replace(/\/$/, "");
        if (!norm || norm === homeNorm) return null;
        if (!norm.startsWith(homeNorm + "/")) return null;
        const parts = norm.split("/");
        parts.pop();
        return parts.length === 0 ? "/" : "/" + parts.join("/");
      })()
    : getParentPath(currentPath);

  const children: FileSystemNode[] = useGrpc
    ? grpcEntries.map((e) => ({
        name: e.name,
        type: (e.node_type === "directory" ? "folder" : "file") as "folder" | "file",
      }))
    : getChildren(currentPath);

  function handleSelectNode(node: FileSystemNode) {
    const path = joinPath(currentPath, node.name);
    if (node.type === "folder") {
      setCurrentPath(path);
      return;
    }
    if (node.type === "file" && mode === "file") {
      onSelect(path);
    }
  }

  function handleConfirmFolder() {
    if (mode === "folder") {
      onSelect(currentPath);
    }
  }

  function handleDoubleClick(node: FileSystemNode) {
    const path = joinPath(currentPath, node.name);
    if (node.type === "folder") {
      setCurrentPath(path);
    } else if (node.type === "file" && mode === "file") {
      onSelect(path);
    }
  }

  function handleGoUp() {
    if (parentPath !== null) setCurrentPath(parentPath);
  }

  function handleBreadcrumbRoot() {
    setCurrentPath(useGrpc && homePath ? homePath : "/");
  }

  if (!open) return null;

  const title = mode === "folder" ? "Select Folder" : "Open File";

  return (
    <div className={styles.overlay} role="dialog" aria-modal="true" aria-label={title}>
      <div className={styles.panel}>
        <div className={styles.header}>
          <h2 className={styles.title}>{title}</h2>
          <div className={styles.pathBar}>
            <button
              type="button"
              className={styles.pathBtn}
              onClick={handleGoUp}
              disabled={parentPath === null}
              title="Go up"
            >
              ↑ Up
            </button>
            <span className={styles.pathText} title={currentPath}>
              {currentPath || (useGrpc && homePath ? homePath : "/")}
            </span>
          </div>
        </div>
        <div className={styles.breadcrumb}>
          <button
            type="button"
            className={styles.breadcrumbItem}
            onClick={handleBreadcrumbRoot}
          >
            {useGrpc && homePath ? homePath.split("/").pop() || "Home" : "/"}
          </button>
          {currentPath
            .split("/")
            .filter(Boolean)
            .map((segment, i, arr) => {
              const path = "/" + arr.slice(0, i + 1).join("/");
              if (useGrpc && homePath && !path.startsWith(homePath)) return null;
              return (
                <span key={path} className={styles.breadcrumbWrap}>
                  <span className={styles.breadcrumbSep}>/</span>
                  <button
                    type="button"
                    className={styles.breadcrumbItem}
                    onClick={() => setCurrentPath(path)}
                  >
                    {segment}
                  </button>
                </span>
              );
            })}
        </div>
        <div ref={listRef} className={styles.list}>
          {loading ? (
            <div className={styles.empty}>Loading…</div>
          ) : error ? (
            <div className={styles.empty}>{error}</div>
          ) : children.length === 0 ? (
            <div className={styles.empty}>This folder is empty</div>
          ) : (
            children.map((node) => {
              const path = joinPath(currentPath, node.name);
              const isFolder = node.type === "folder";
              const selectable = isFolder || mode === "file";
              return (
                <button
                  key={path}
                  type="button"
                  className={styles.listItem}
                  onClick={() => selectable && handleSelectNode(node)}
                  onDoubleClick={() => selectable && handleDoubleClick(node)}
                  data-type={node.type}
                >
                  {isFolder ? <FolderIcon /> : <FileIcon />}
                  <span className={styles.listItemName}>{node.name}</span>
                </button>
              );
            })
          )}
        </div>
        <div className={styles.footer}>
          <button type="button" className={styles.cancelBtn} onClick={onCancel}>
            Cancel
          </button>
          {mode === "folder" && (
            <button type="button" className={styles.selectBtn} onClick={handleConfirmFolder}>
              Select Folder
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

function FolderIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
      <polyline points="2 9 2 5 8 5 10 9 22 9" />
    </svg>
  );
}

function FileIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
      <polyline points="14 2 14 8 20 8" />
      <line x1="16" y1="13" x2="8" y2="13" />
      <line x1="16" y1="17" x2="8" y2="17" />
      <polyline points="10 9 9 9 8 9" />
    </svg>
  );
}
