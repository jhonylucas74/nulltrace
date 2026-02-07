import { useState, useMemo } from "react";
import { getChildren, getHomePath, type FileSystemNode } from "../lib/fileSystem";
import styles from "./Explorer.module.css";

const PLACES: { label: string; path: string }[] = [
  { label: "Home", path: getHomePath() },
  { label: "File system", path: "/" },
  { label: "Documents", path: "/home/user/Documents" },
  { label: "Downloads", path: "/home/user/Downloads" },
  { label: "Desktop", path: "/home/user/Desktop" },
];

function getParentPath(path: string): string | null {
  const normalized = path.replace(/\/+/g, "/").replace(/\/$/, "") || "";
  if (!normalized) return null;
  const parts = normalized.split("/");
  parts.pop();
  return parts.length === 0 ? "/" : "/" + parts.join("/");
}

function pathToBreadcrumb(path: string): string[] {
  const normalized = path.replace(/\/+/g, "/").replace(/^\//, "").replace(/\/$/, "") || "";
  if (!normalized) return ["/"];
  return normalized.split("/");
}

export default function Explorer() {
  const [currentPath, setCurrentPath] = useState(getHomePath);

  const entries = useMemo(() => getChildren(currentPath), [currentPath]);
  const parentPath = getParentPath(currentPath);
  const breadcrumb = pathToBreadcrumb(currentPath);

  function handleBack() {
    if (parentPath !== null) setCurrentPath(parentPath);
  }

  function handlePlace(path: string) {
    setCurrentPath(path);
  }

  function handleOpen(node: FileSystemNode) {
    if (node.type === "folder") {
      const newPath = currentPath.replace(/\/$/, "") + "/" + node.name;
      setCurrentPath(newPath);
    } else {
      // File: no-op or could show "Preview not available"
    }
  }

  function handleBreadcrumbClick(index: number) {
    if (index === 0) {
      setCurrentPath("/");
      return;
    }
    const segs = breadcrumb.slice(0, index + 1);
    setCurrentPath("/" + segs.join("/"));
  }

  return (
    <div className={styles.app}>
      <aside className={styles.sidebar}>
        <div className={styles.sidebarSection}>Places</div>
        {PLACES.map(({ label, path }) => (
          <button
            key={path}
            type="button"
            className={`${styles.place} ${currentPath === path ? styles.placeActive : ""}`}
            onClick={() => handlePlace(path)}
          >
            <span className={styles.placeIcon}>{path === getHomePath() ? <HomeIcon /> : <FolderIcon />}</span>
            <span className={styles.placeLabel}>{label}</span>
          </button>
        ))}
      </aside>
      <div className={styles.main}>
        <div className={styles.toolbar}>
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
                  {seg || "File system"}
                </button>
              </span>
            ))}
          </div>
        </div>
        <div className={styles.list}>
          {entries.length === 0 ? (
            <div className={styles.empty}>This folder is empty</div>
          ) : (
            entries.map((node) => (
              <button
                key={node.name}
                type="button"
                className={styles.row}
                data-type={node.type}
                onDoubleClick={() => handleOpen(node)}
              >
                <span className={styles.rowIcon}>
                  {node.type === "folder" ? <FolderIcon /> : <FileIcon />}
                </span>
                <span className={styles.rowName}>{node.name}</span>
              </button>
            ))
          )}
        </div>
      </div>
    </div>
  );
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
