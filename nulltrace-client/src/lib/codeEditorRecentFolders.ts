/**
 * Recent folders for the Code app. Stored in localStorage, scoped by user (playerId)
 * so multiple users on the same machine get their own list.
 */

const STORAGE_KEY_PREFIX = "nulltrace_code_recent_folders";
const MAX_RECENT = 10;

function storageKey(playerId: string): string {
  return `${STORAGE_KEY_PREFIX}_${playerId}`;
}

export function getRecentFolders(playerId: string | null): string[] {
  if (!playerId || typeof localStorage === "undefined") return [];
  try {
    const raw = localStorage.getItem(storageKey(playerId));
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((p): p is string => typeof p === "string").slice(0, MAX_RECENT);
  } catch {
    return [];
  }
}

export function addRecentFolder(playerId: string | null, path: string): void {
  if (!playerId || typeof localStorage === "undefined" || !path.trim()) return;
  const normalized = path.replace(/\/+$/, "") || path;
  const current = getRecentFolders(playerId);
  const filtered = current.filter((p) => p !== normalized);
  const next = [normalized, ...filtered].slice(0, MAX_RECENT);
  try {
    localStorage.setItem(storageKey(playerId), JSON.stringify(next));
  } catch {
    // ignore quota or other errors
  }
}
