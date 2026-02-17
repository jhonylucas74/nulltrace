/**
 * Virtual file system singleton – in-memory Linux-inspired tree for the Files explorer.
 * Paths like "/", "/home", "/home/user". No real I/O.
 */

export interface FileSystemNode {
  name: string;
  type: "folder" | "file";
  children?: FileSystemNode[];
}

function dir(...nodes: FileSystemNode[]): FileSystemNode {
  return { name: "", type: "folder", children: nodes };
}

function folder(name: string, ...children: FileSystemNode[]): FileSystemNode {
  return { name, type: "folder", children };
}

function file(name: string): FileSystemNode {
  return { name, type: "file" };
}

const ROOT: FileSystemNode = dir(
  folder("home", folder("user", folder("Documents", file("welcome.txt")), folder("Downloads", file("game.zip")), folder("Desktop", file("notes.txt")), file("readme.txt"), file("notes.txt"))),
  folder("etc", file("hostname"), file("os-release")),
  folder("var", folder("log", file("syslog"))),
  folder("tmp", file("temp_1234")),
  folder("usr", folder("bin"), folder("share", file("readme")))
);

/** In-memory file contents (path -> content). Paths normalized with leading slash. */
const fileContents = new Map<string, string>();

function normalizePathForStorage(path: string): string {
  const p = path.replace(/\/+/g, "/").trim();
  return p.startsWith("/") ? p : "/" + p;
}

function parsePath(path: string): string[] {
  const parts = path.replace(/\/+/g, "/").split("/").filter(Boolean);
  const resolved: string[] = [];
  for (const p of parts) {
    if (p === ".") continue;
    if (p === "..") {
      resolved.pop();
      continue;
    }
    resolved.push(p);
  }
  return resolved;
}

function findNode(node: FileSystemNode, segments: string[]): FileSystemNode | null {
  if (segments.length === 0) return node;
  const [first, ...rest] = segments;
  const child = node.children?.find((c) => c.name === first) ?? null;
  if (!child) return null;
  return findNode(child, rest);
}

/**
 * Returns the node at the given path, or null if not found.
 * Resolves "." and ".." in path.
 */
export function getItem(path: string): FileSystemNode | null {
  const normalized = path.replace(/\/+/g, "/").replace(/^\//, "").replace(/\/$/, "") || "";
  const segments = normalized ? parsePath(normalized) : [];
  const node = findNode(ROOT, segments);
  return node && (node.name || segments.length === 0) ? node : null;
}

/**
 * Returns children of the folder at path. Folders first, then files. Empty array for invalid path or file.
 */
export function getChildren(path: string): FileSystemNode[] {
  const node = getItem(path);
  if (!node || node.type !== "folder" || !node.children) return [];
  const list = [...node.children];
  list.sort((a, b) => {
    if (a.type !== b.type) return a.type === "folder" ? -1 : 1;
    return a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
  });
  return list;
}

/**
 * Home path for the default user (can later use username from auth).
 */
export function getHomePath(): string {
  return "/home/user";
}

/**
 * Returns the parent path, or null if already at root.
 * e.g. "/home/user/docs" -> "/home/user", "/home" -> ""
 */
export function getParentPath(path: string): string | null {
  const normalized = path.replace(/\/+/g, "/").trim().replace(/\/$/, "") || "";
  if (!normalized) return null;
  const lastSlash = normalized.lastIndexOf("/");
  if (lastSlash < 0) return null;
  if (lastSlash === 0) return "/";
  return normalized.slice(0, lastSlash);
}

/**
 * Creates a new file under parentPath. Returns false if parent is not a folder or name already exists.
 */
export function createFile(parentPath: string, name: string): boolean {
  const parent = getItem(parentPath);
  if (!parent || parent.type !== "folder") return false;
  const children = parent.children ?? [];
  if (children.some((c) => c.name === name)) return false;
  parent.children = [...children, file(name)];
  return true;
}

/**
 * Creates a new folder under parentPath. Returns false if parent is not a folder or name already exists.
 */
export function createFolder(parentPath: string, name: string): boolean {
  const parent = getItem(parentPath);
  if (!parent || parent.type !== "folder") return false;
  const children = parent.children ?? [];
  if (children.some((c) => c.name === name)) return false;
  parent.children = [...children, folder(name)];
  return true;
}

/**
 * Renames (moves) a node from oldPath to newPath. Parent of newPath must match parent of oldPath (same directory rename).
 * Returns false if oldPath not found, new name already exists, or paths are invalid.
 */
export function renamePath(oldPath: string, newPath: string): boolean {
  const node = getItem(oldPath);
  if (!node) return false;
  const parentPath = getParentPath(oldPath);
  if (parentPath === null) return false;
  const parent = getItem(parentPath);
  if (!parent || parent.type !== "folder" || !parent.children) return false;
  const newName = newPath.replace(/\/+$/, "").split("/").pop() ?? "";
  if (!newName || newName === node.name) return false;
  if (parent.children.some((c) => c.name === newName)) return false;

  const idx = parent.children.findIndex((c) => c.name === node.name);
  if (idx < 0) return false;

  const keyOld = normalizePathForStorage(oldPath);
  const keyNew = normalizePathForStorage(newPath);

  if (node.type === "file") {
    const content = fileContents.get(keyOld) ?? "";
    fileContents.set(keyNew, content);
    fileContents.delete(keyOld);
  } else {
    // Folder: re-key all file contents under oldPath to newPath
    const prefix = keyOld.endsWith("/") ? keyOld : keyOld + "/";
    const toMove = Array.from(fileContents.entries()).filter(([k]) => k === keyOld || k.startsWith(prefix));
    for (const [k, content] of toMove) {
      fileContents.delete(k);
      const suffix = k.slice(prefix.length);
      fileContents.set(keyNew + (keyNew.endsWith("/") ? "" : "/") + suffix, content);
    }
  }
  // Update node name and parent's child reference
  const newNode = { ...node, name: newName };
  const nextChildren = [...parent.children];
  nextChildren[idx] = newNode;
  parent.children = nextChildren;
  return true;
}

/**
 * Moves the node at oldPath into the folder newParentPath. Returns false if invalid or name clash.
 */
export function movePath(oldPath: string, newParentPath: string): boolean {
  const node = getItem(oldPath);
  if (!node) return false;
  const oldParentPath = getParentPath(oldPath);
  if (oldParentPath === null) return false;
  const newParent = getItem(newParentPath);
  if (!newParent || newParent.type !== "folder") return false;
  if (newParentPath === oldPath || newParentPath.startsWith(oldPath + "/")) return false;
  const name = node.name;
  const newChildren = newParent.children ?? [];
  if (newChildren.some((c) => c.name === name)) return false;

  const oldParent = getItem(oldParentPath);
  if (!oldParent || oldParent.type !== "folder" || !oldParent.children) return false;
  const idx = oldParent.children.findIndex((c) => c.name === name);
  if (idx < 0) return false;

  const keyOld = normalizePathForStorage(oldPath);
  const base = newParentPath.replace(/\/+$/, "");
  const newPath = base ? `${base}/${name}` : `/${name}`;

  if (node.type === "file") {
    const content = fileContents.get(keyOld) ?? "";
    fileContents.delete(keyOld);
    fileContents.set(normalizePathForStorage(newPath), content);
  } else {
    const prefix = (keyOld.endsWith("/") ? keyOld : keyOld + "/");
    const toMove = Array.from(fileContents.entries()).filter(([k]) => k.startsWith(prefix));
    for (const [k, content] of toMove) {
      fileContents.delete(k);
      const suffix = k.slice(prefix.length);
      const nKey = normalizePathForStorage(newPath + "/" + suffix);
      fileContents.set(nKey, content);
    }
  }

  const nextOldChildren = oldParent.children.filter((_, i) => i !== idx);
  oldParent.children = nextOldChildren;
  newParent.children = [...newChildren, node];
  return true;
}

/**
 * Returns the stored content for a file path, or "" if none.
 */
export function getFileContent(path: string): string {
  const key = normalizePathForStorage(path);
  return fileContents.get(key) ?? "";
}

/**
 * Sets the content for a file path. Only valid for existing file nodes.
 */
export function setFileContent(path: string, content: string): void {
  const node = getItem(path);
  if (!node || node.type !== "file") return;
  const key = normalizePathForStorage(path);
  fileContents.set(key, content);
}
