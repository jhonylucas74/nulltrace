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
