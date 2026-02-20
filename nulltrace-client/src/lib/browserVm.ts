/**
 * VM URL handling for the Browser app.
 * Detects VM URLs (host/path or host:port/path) and rejects dangerous schemes.
 */

/** Dangerous URL schemes that must be rejected. */
const DANGEROUS_SCHEMES = ["file:", "javascript:", "data:", "vbscript:"];

/**
 * Returns true if the URL is a VM URL (host/path or host:port/path).
 * Built-in pages (search.example, about:blank, browser://history) are not VM URLs.
 */
export function isVmUrl(url: string): boolean {
  const u = url.trim();
  if (!u) return false;

  const lower = u.toLowerCase();
  for (const scheme of DANGEROUS_SCHEMES) {
    if (lower.startsWith(scheme)) return false;
  }

  // Built-in pages: no VM fetch
  if (
    lower === "search.example" ||
    lower === "about:blank" ||
    lower.startsWith("browser://")
  ) {
    return false;
  }

  // VM URL format: host, host/path, or host:port/path
  return true;
}

/**
 * Normalizes a VM URL for curl. Strips http(s):// prefix, hash fragment, ensures path has leading slash.
 * Only call when isVmUrl returns true.
 */
export function normalizeVmUrl(url: string): string {
  let u = url.trim();
  const hashIdx = u.indexOf("#");
  if (hashIdx >= 0) u = u.slice(0, hashIdx);
  const lower = u.toLowerCase();
  if (lower.startsWith("https://")) {
    u = u.slice(8);
  } else if (lower.startsWith("http://")) {
    u = u.slice(7);
  }
  const slashIdx = u.indexOf("/");
  if (slashIdx < 0) {
    return u + "/";
  }
  const path = u.slice(slashIdx);
  return path.startsWith("/") ? u : u.slice(0, slashIdx) + "/" + path.slice(1);
}

/** Parsed HTTP response from raw curl stdout. */
export interface ParsedHttpResponse {
  status: number;
  contentType: string | null;
  body: string;
  raw: string;
}

/**
 * Parses raw HTTP response string (from curl stdout) into status, headers, and body.
 * Splits only on the HTTP header/body boundary (CRLF CRLF or first empty line),
 * so that a body containing blank lines (e.g. YAML with "head:\n\nbody:") is not truncated.
 */
export function parseHttpResponse(raw: string): ParsedHttpResponse | null {
  if (!raw || !raw.trim()) return null;

  let head: string;
  let body: string;
  const crlfBoundary = raw.indexOf("\r\n\r\n");
  if (crlfBoundary >= 0) {
    head = raw.slice(0, crlfBoundary);
    body = raw.slice(crlfBoundary + 4);
  } else {
    // No CRLF CRLF: find first empty line so we don't split on \n\n inside body (e.g. NTML YAML)
    const lines = raw.split(/\r?\n/);
    let i = 0;
    for (; i < lines.length; i++) {
      if (lines[i].trim() === "") break;
    }
    head = lines.slice(0, i).join("\n");
    body = lines.slice(i + 1).join("\n");
  }

  const lines = head.split(/\r?\n/);
  const statusLine = lines[0] ?? "";
  const statusMatch = statusLine.match(/HTTP\/[\d.]+\s+(\d+)/);
  const status = statusMatch ? parseInt(statusMatch[1], 10) : 0;

  let contentType: string | null = null;
  for (let i = 1; i < lines.length; i++) {
    const line = lines[i];
    const colonIdx = line.indexOf(":");
    if (colonIdx >= 0) {
      const name = line.slice(0, colonIdx).trim().toLowerCase();
      const value = line.slice(colonIdx + 1).trim();
      if (name === "content-type") {
        contentType = value.split(";")[0]?.trim() ?? value;
        break;
      }
    }
  }

  return { status, contentType, body, raw };
}

/**
 * Extracts the host part (host or host:port) from a VM URL for use as base.
 * E.g. ntml.org/robot -> ntml.org, 10.0.1.5:80/about -> 10.0.1.5:80
 */
export function getBaseHost(url: string): string {
  const u = normalizeVmUrl(url);
  const slashIdx = u.indexOf("/");
  return slashIdx >= 0 ? u.slice(0, slashIdx) : u;
}

/**
 * Resolves a relative path (e.g. scripts/main.lua) against a base VM URL.
 * Base URL format: host/path or host:port/path (e.g. ntml.org/robot).
 * Returns full URL for curl: host/scripts/main.lua or host:port/scripts/main.lua.
 */
export function resolveScriptUrl(baseUrl: string, relativePath: string): string {
  const hostPart = getBaseHost(baseUrl);
  const rel = relativePath.startsWith("/") ? relativePath.slice(1) : relativePath;
  return `${hostPart}/${rel}`;
}
