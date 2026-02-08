/**
 * Simple Lua/Luau syntax highlighter. Returns HTML string with spans for tokens.
 * Used for editor overlay (same font/size as textarea).
 */

/** Lua 5.1 / Luau keywords */
const LUA_KEYWORDS = new Set([
  "and", "break", "do", "else", "elseif", "end", "false", "for", "function",
  "goto", "if", "in", "local", "nil", "not", "or", "repeat", "return",
  "then", "true", "until", "while",
  // Luau extras
  "type", "export", "continue",
]);

/** Common Lua/Luau built-in globals (highlighted as builtin) */
const LUA_BUILTINS = new Set([
  "print", "type", "pairs", "ipairs", "next", "tostring", "tonumber",
  "string", "table", "math", "io", "os", "debug", "coroutine", "package",
  "require", "assert", "error", "pcall", "xpcall", "select", "unpack",
  "rawget", "rawset", "getmetatable", "setmetatable", "rawequal", "rawlen",
]);

export type TokenType = "keyword" | "string" | "comment" | "number" | "builtin" | "default";

interface Token {
  type: TokenType;
  value: string;
}

function tokenize(line: string): Token[] {
  const tokens: Token[] = [];
  let i = 0;
  const n = line.length;

  while (i < n) {
    // Line comment
    if (line.slice(i, i + 2) === "--" && line.slice(i, i + 4) !== "--[[") {
      tokens.push({ type: "comment", value: line.slice(i) });
      break;
    }
    // Block comment start
    if (line.slice(i, i + 4) === "--[[") {
      let end = line.indexOf("]]", i + 4);
      if (end === -1) end = n;
      tokens.push({ type: "comment", value: line.slice(i, end + 2) });
      i = end + 2;
      continue;
    }
    // Double-quoted string
    if (line[i] === '"') {
      let j = i + 1;
      while (j < n) {
        if (line[j] === "\\") j += 2;
        else if (line[j] === '"') { j++; break; }
        else j++;
      }
      tokens.push({ type: "string", value: line.slice(i, j) });
      i = j;
      continue;
    }
    // Single-quoted string
    if (line[i] === "'") {
      let j = i + 1;
      while (j < n) {
        if (line[j] === "\\") j += 2;
        else if (line[j] === "'") { j++; break; }
        else j++;
      }
      tokens.push({ type: "string", value: line.slice(i, j) });
      i = j;
      continue;
    }
    // Long string [[ ... ]]
    if (line.slice(i, i + 2) === "[[" && (i === 0 || /[\s(\[,=]/.test(line[i - 1]))) {
      let j = line.indexOf("]]", i + 2);
      if (j === -1) j = n;
      tokens.push({ type: "string", value: line.slice(i, j + 2) });
      i = j + 2;
      continue;
    }
    // Number
    if (/[0-9]/.test(line[i]) || (line[i] === "." && i + 1 < n && /[0-9]/.test(line[i + 1]))) {
      let j = i;
      if (line[j] === "0" && j + 1 < n && (line[j + 1] === "x" || line[j + 1] === "X")) {
        j += 2;
        while (j < n && /[0-9a-fA-F]/.test(line[j])) j++;
      } else {
        while (j < n && /[0-9]/.test(line[j])) j++;
        if (line[j] === "." && j + 1 < n && /[0-9]/.test(line[j + 1])) {
          j++;
          while (j < n && /[0-9]/.test(line[j])) j++;
        }
        if (j < n && (line[j] === "e" || line[j] === "E")) {
          j++;
          if (line[j] === "+" || line[j] === "-") j++;
          while (j < n && /[0-9]/.test(line[j])) j++;
        }
      }
      tokens.push({ type: "number", value: line.slice(i, j) });
      i = j;
      continue;
    }
    // Identifier, keyword, or builtin
    if (/[a-zA-Z_]/.test(line[i])) {
      let j = i;
      while (j < n && /[a-zA-Z0-9_]/.test(line[j])) j++;
      const word = line.slice(i, j);
      const type = LUA_KEYWORDS.has(word)
        ? "keyword"
        : LUA_BUILTINS.has(word)
          ? "builtin"
          : "default";
      tokens.push({ type, value: word });
      i = j;
      continue;
    }
    // Single char default
    tokens.push({ type: "default", value: line[i] });
    i++;
  }

  return tokens;
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function tokenToSpan(token: Token): string {
  const escaped = escapeHtml(token.value);
  if (token.type === "default") return escaped;
  return `<span class="luau-${token.type}">${escaped}</span>`;
}

/**
 * Convert Lua source to HTML with syntax-highlighted spans.
 * Use CSS classes: .luau-keyword, .luau-string, .luau-comment, .luau-number
 */
export function highlightLua(source: string): string {
  const lines = source.split("\n");
  const out: string[] = [];
  for (const line of lines) {
    const tokens = tokenize(line);
    out.push(tokens.map(tokenToSpan).join(""));
  }
  return out.join("\n");
}

export function isLuaFile(path: string | null): boolean {
  if (!path) return false;
  const lower = path.toLowerCase();
  return lower.endsWith(".lua") || lower.endsWith(".luau");
}
