import { useState, useRef, useEffect, useCallback, KeyboardEvent } from "react";
import { useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { runMockCommand, isClearCommand } from "../lib/mockCommands";
import { useAuth } from "../contexts/AuthContext";
import { useWindowManager } from "../contexts/WindowManagerContext";
import Modal from "./Modal";
import styles from "./Terminal.module.css";

interface TerminalProps {
  username: string;
  /** Window ID for closing on connect error (Tauri only). */
  windowId?: string;
}

interface LineItem {
  type: "prompt" | "output" | "error";
  content: string;
}

/** Supported inline tag types: colors and formatting (strong/bold, italic/em). */
type SegmentType =
  | "normal"
  | "red"
  | "green"
  | "yellow"
  | "blue"
  | "cyan"
  | "magenta"
  | "strong"
  | "italic";

interface ColorSegment {
  type: SegmentType;
  text: string;
}

const TAG_REGEX =
  /<(red|green|yellow|blue|cyan|magenta|strong|b|italic|i|em)>(.*?)<\/\1>/gs;

function normalizeTag(tag: string): SegmentType {
  switch (tag.toLowerCase()) {
    case "b":
      return "strong";
    case "i":
    case "em":
      return "italic";
    default:
      return tag as SegmentType;
  }
}

/** Parses shell tags (e.g. <red>...</red>, <strong>...</strong>) into segments. */
function parseColorTags(text: string): ColorSegment[] {
  const segments: ColorSegment[] = [];
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  TAG_REGEX.lastIndex = 0;
  while ((match = TAG_REGEX.exec(text)) !== null) {
    if (match.index > lastIndex) {
      segments.push({ type: "normal", text: text.slice(lastIndex, match.index) });
    }
    segments.push({ type: normalizeTag(match[1]), text: match[2] });
    lastIndex = TAG_REGEX.lastIndex;
  }
  if (lastIndex < text.length) {
    segments.push({ type: "normal", text: text.slice(lastIndex) });
  }
  if (segments.length === 0) {
    segments.push({ type: "normal", text: text });
  }
  return segments;
}

const SEGMENT_CLASS: Record<Exclude<SegmentType, "normal">, string> = {
  red: styles.segmentRed,
  green: styles.segmentGreen,
  yellow: styles.segmentYellow,
  blue: styles.segmentBlue,
  cyan: styles.segmentCyan,
  magenta: styles.segmentMagenta,
  strong: styles.segmentStrong,
  italic: styles.segmentItalic,
};

function renderLineContent(content: string, _lineType: "output" | "error") {
  const segments = parseColorTags(content);
  return (
    <>
      {segments.map((seg, j) =>
        seg.type === "normal" ? (
          <span key={j}>{seg.text}</span>
        ) : (
          <span key={j} className={SEGMENT_CLASS[seg.type]}>
            {renderLineContent(seg.text, _lineType)}
          </span>
        )
      )}
    </>
  );
}

interface TerminalOutputPayload {
  sessionId: string;
  type: "stdout" | "closed" | "error" | "prompt_ready";
  data?: string;
}

const CONNECT_ERROR_MESSAGE =
  "Unexpected error. Could not load the shell (missing or corrupted). The terminal will close.";

const MAX_LINES = 100;

/** Tab size for expanding tabs in terminal output (column alignment). */
const TAB_SIZE = 8;

/**
 * Expands tab characters to spaces so columns align in monospace.
 * Backend may send tab-separated output (e.g. old ls); this keeps alignment in the UI.
 */
function expandTabs(line: string, tabSize: number = TAB_SIZE): string {
  if (!line.includes("\t")) return line;
  let col = 0;
  let out = "";
  for (let i = 0; i < line.length; i++) {
    const c = line[i];
    if (c === "\t") {
      const spaces = tabSize - (col % tabSize);
      out += " ".repeat(spaces);
      col += spaces;
    } else {
      out += c;
      col += 1;
    }
  }
  return out;
}

function padRight(s: string, w: number): string {
  return s + " ".repeat(Math.max(0, w - s.length));
}

function padLeft(s: string, w: number): string {
  return " ".repeat(Math.max(0, w - s.length)) + s;
}

/** Fixed column widths for ls-style 4-column output (name, type, size, owner). */
const LS_COL_NAME = 20;
const LS_COL_TYPE = 12;
const LS_COL_SIZE = 8;

/**
 * Normalizes a line for display: if it looks like ls output (4 tab-separated fields),
 * re-formats with fixed-width columns so they align; otherwise expands tabs as usual.
 */
function normalizeOutputLine(content: string): string {
  if (!content.includes("\t")) return content;
  const parts = content.split("\t");
  if (parts.length === 4) {
    const [name, type_, size, owner] = parts;
    return (
      padRight(name, LS_COL_NAME) +
      "  " +
      padRight(type_, LS_COL_TYPE) +
      "  " +
      padLeft(size.trim(), LS_COL_SIZE) +
      "  " +
      owner
    );
  }
  return expandTabs(content);
}

function trimToLast<T>(arr: T[]): T[] {
  return arr.length > MAX_LINES ? arr.slice(-MAX_LINES) : arr;
}

export default function Terminal({ username, windowId }: TerminalProps) {
  const { playerId, token, logout } = useAuth();
  const navigate = useNavigate();
  const { close } = useWindowManager();
  const [lines, setLines] = useState<LineItem[]>([
    { type: "output", content: "Welcome to nulltrace. Type 'help' for commands." },
  ]);
  const [input, setInput] = useState("");
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [sessionEnded, setSessionEnded] = useState(false);
  const [connectErrorModalOpen, setConnectErrorModalOpen] = useState(false);
  const [memoryLimitModalOpen, setMemoryLimitModalOpen] = useState(false);
  const [cursorVisible, setCursorVisible] = useState(true);
  const [cursorPosition, setCursorPosition] = useState(0);
  const [commandHistory, setCommandHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [waitingForPrompt, setWaitingForPrompt] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRowRef = useRef<HTMLDivElement>(null);
  const hiddenInputRef = useRef<HTMLInputElement>(null);
  const sessionIdRef = useRef<string | null>(null);
  sessionIdRef.current = sessionId;

  // Effect "generation": only the latest mount's deferred connect runs (avoids double shell in React Strict Mode).
  const connectGenerationRef = useRef(0);

  // Auto-scroll to bottom when content changes or when prompt reappears after command (waitingForPrompt -> false).
  // When the prompt row reappears, layout updates asynchronously; we scroll in multiple passes so the view stays at bottom.
  useEffect(() => {
    const scrollToBottom = () => {
      if (scrollRef.current) {
        scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
      }
    };
    const bringInputRowIntoView = () => {
      inputRowRef.current?.scrollIntoView({ block: "end", behavior: "auto" });
    };
    scrollToBottom();
    bringInputRowIntoView();
    const raf1 = requestAnimationFrame(() => {
      scrollToBottom();
      bringInputRowIntoView();
    });
    const raf2 = requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        scrollToBottom();
        bringInputRowIntoView();
      });
    });
    const t1 = setTimeout(scrollToBottom, 0);
    const t2 = setTimeout(() => {
      scrollToBottom();
      bringInputRowIntoView();
    }, 80);
    return () => {
      cancelAnimationFrame(raf1);
      cancelAnimationFrame(raf2);
      clearTimeout(t1);
      clearTimeout(t2);
    };
  }, [lines, input, waitingForPrompt]);

  // Cursor blink effect
  useEffect(() => {
    const interval = setInterval(() => {
      setCursorVisible((v) => !v);
    }, 530);
    return () => clearInterval(interval);
  }, []);

  // Reset cursor visibility when typing
  useEffect(() => {
    setCursorVisible(true);
  }, [input]);

  // Sync cursor position from hidden input
  useEffect(() => {
    const updateCursorPosition = () => {
      requestAnimationFrame(() => {
        const pos = hiddenInputRef.current?.selectionStart ?? input.length;
        setCursorPosition(pos);
      });
    };

    const inputElement = hiddenInputRef.current;
    if (inputElement) {
      updateCursorPosition();
      inputElement.addEventListener("keydown", updateCursorPosition);
      inputElement.addEventListener("keyup", updateCursorPosition);
      inputElement.addEventListener("click", updateCursorPosition);
      inputElement.addEventListener("select", updateCursorPosition);
      inputElement.addEventListener("input", updateCursorPosition);

      return () => {
        inputElement.removeEventListener("keydown", updateCursorPosition);
        inputElement.removeEventListener("keyup", updateCursorPosition);
        inputElement.removeEventListener("click", updateCursorPosition);
        inputElement.removeEventListener("select", updateCursorPosition);
        inputElement.removeEventListener("input", updateCursorPosition);
      };
    }
  }, [input]);

  const prompt = `${username}@nulltrace:~$ `;

  // Word navigation helpers
  const findPreviousWordStart = useCallback((text: string, currentPos: number): number => {
    if (currentPos === 0) return 0;

    let pos = currentPos - 1;

    // Skip whitespace
    while (pos > 0 && /\s/.test(text[pos])) {
      pos--;
    }

    // Skip to beginning of word
    while (pos > 0 && /\S/.test(text[pos - 1])) {
      pos--;
    }

    return pos;
  }, []);

  const findNextWordStart = useCallback((text: string, currentPos: number): number => {
    if (currentPos >= text.length) return text.length;

    let pos = currentPos;

    // Skip current word
    while (pos < text.length && /\S/.test(text[pos])) {
      pos++;
    }

    // Skip whitespace
    while (pos < text.length && /\s/.test(text[pos])) {
      pos++;
    }

    return pos;
  }, []);

  // Connect to VM shell when we have playerId (Tauri only).
  // Deferred so that in React Strict Mode only the final mount opens one terminal (not two).
  useEffect(() => {
    if (!playerId || !token) return;

    const generation = ++connectGenerationRef.current;
    let unlisten: (() => void) | undefined;

    const runConnect = async () => {
      if (generation !== connectGenerationRef.current) return;
      try {
        const sid = await invoke<string>("terminal_connect", { token });
        if (generation !== connectGenerationRef.current) {
          invoke("terminal_disconnect", { sessionId: sid }).catch(() => {});
          return;
        }
        sessionIdRef.current = sid;
        setSessionId(sid);
        setSessionEnded(false);
        setWaitingForPrompt(false);
        setLines([]);

        unlisten = await listen<TerminalOutputPayload>("terminal-output", (event) => {
          const payload = event.payload;
          if (payload.sessionId !== sid) return;

          if (payload.type === "stdout" && payload.data !== undefined) {
            const TABCOMPLETE_PREFIX = "\x01TABCOMPLETE\t";
            if (payload.data.startsWith(TABCOMPLETE_PREFIX)) {
              const replacement = payload.data.slice(TABCOMPLETE_PREFIX.length).split("\n")[0] ?? "";
              setInput(replacement);
              setCursorPosition(replacement.length);
              return;
            }
            setLines((prev) =>
              trimToLast([
                ...prev,
                ...payload.data!
                  .split("\n")
                  .filter((s) => s.length > 0)
                  .map((content) => ({ type: "output" as const, content: normalizeOutputLine(content) })),
              ])
            );
          } else if (payload.type === "prompt_ready") {
            setWaitingForPrompt(false);
          } else if (payload.type === "closed") {
            setSessionEnded(true);
            setWaitingForPrompt(false);
            setLines((prev) => trimToLast([...prev, { type: "output", content: "Session ended." }]));
            sessionIdRef.current = null;
            setSessionId(null);
          } else if (payload.type === "error" && payload.data) {
            const msg = payload.data;
            setLines((prev) => trimToLast([...prev, { type: "error", content: msg }]));
            setSessionEnded(true);
            sessionIdRef.current = null;
            setSessionId(null);
            setWaitingForPrompt(false);
            if (msg.toLowerCase().includes("memory limit")) {
              setMemoryLimitModalOpen(true);
            }
          }
        });
      } catch (e) {
        if (generation !== connectGenerationRef.current) return;
        const errorMsg = e instanceof Error ? e.message : String(e);
        if (errorMsg === "UNAUTHENTICATED") {
          logout();
          navigate("/login");
          return;
        }
        setConnectErrorModalOpen(true);
      }
    };

    queueMicrotask(runConnect);

    return () => {
      unlisten?.();
      const sid = sessionIdRef.current;
      if (sid) {
        invoke("terminal_disconnect", { sessionId: sid }).catch(() => {});
      }
    };
  }, [playerId, token, logout, navigate]);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent<HTMLInputElement>) => {
      // Handle Ctrl+C - send special sequence via stdin (kill foreground or forward through SSH), echo ^C
      if (e.ctrlKey && e.key === "c") {
        e.preventDefault();
        if (sessionId && !sessionEnded) {
          setLines((prev) => trimToLast([...prev, { type: "output", content: "^C" }]));
          invoke("terminal_send_stdin", { sessionId, data: "\x03" }).catch((err) => {
            setLines((prev) => trimToLast([...prev, { type: "error", content: String(err) }]));
          });
        }
        return;
      }

      // Handle Tab - send autocomplete request via stdin (input + \x09); shell responds with \x01TABCOMPLETE\t + replacement
      if (e.key === "Tab") {
        e.preventDefault();
        if (sessionId && !sessionEnded) {
          invoke("terminal_send_stdin", { sessionId, data: input + "\x09" }).catch((err) => {
            setLines((prev) => trimToLast([...prev, { type: "error", content: String(err) }]));
          });
        }
        return;
      }

      // Handle Alt + ArrowLeft - jump to previous word
      if (e.key === "ArrowLeft" && e.altKey) {
        e.preventDefault();
        const newPos = findPreviousWordStart(input, cursorPosition);
        if (hiddenInputRef.current) {
          hiddenInputRef.current.selectionStart = newPos;
          hiddenInputRef.current.selectionEnd = newPos;
          setCursorPosition(newPos);
        }
        return;
      }

      // Handle Alt + ArrowRight - jump to next word
      if (e.key === "ArrowRight" && e.altKey) {
        e.preventDefault();
        const newPos = findNextWordStart(input, cursorPosition);
        if (hiddenInputRef.current) {
          hiddenInputRef.current.selectionStart = newPos;
          hiddenInputRef.current.selectionEnd = newPos;
          setCursorPosition(newPos);
        }
        return;
      }

      // Handle arrow up - navigate to previous command
      if (e.key === "ArrowUp") {
        e.preventDefault();
        if (commandHistory.length === 0) return;

        const newIndex = historyIndex + 1;
        if (newIndex < commandHistory.length) {
          setHistoryIndex(newIndex);
          const cmd = commandHistory[commandHistory.length - 1 - newIndex];
          setInput(cmd);
          // Move cursor to end of line
          setTimeout(() => {
            if (hiddenInputRef.current) {
              hiddenInputRef.current.selectionStart = cmd.length;
              hiddenInputRef.current.selectionEnd = cmd.length;
              setCursorPosition(cmd.length);
            }
          }, 0);
        }
        return;
      }

      // Handle arrow down - navigate to next command
      if (e.key === "ArrowDown") {
        e.preventDefault();
        if (historyIndex > 0) {
          const newIndex = historyIndex - 1;
          setHistoryIndex(newIndex);
          const cmd = commandHistory[commandHistory.length - 1 - newIndex];
          setInput(cmd);
          // Move cursor to end of line
          setTimeout(() => {
            if (hiddenInputRef.current) {
              hiddenInputRef.current.selectionStart = cmd.length;
              hiddenInputRef.current.selectionEnd = cmd.length;
              setCursorPosition(cmd.length);
            }
          }, 0);
        } else if (historyIndex === 0) {
          setHistoryIndex(-1);
          setInput("");
          setTimeout(() => {
            if (hiddenInputRef.current) {
              hiddenInputRef.current.selectionStart = 0;
              hiddenInputRef.current.selectionEnd = 0;
              setCursorPosition(0);
            }
          }, 0);
        }
        return;
      }

      // Handle Enter - submit command
      if (e.key !== "Enter") return;
      e.preventDefault();
      const cmd = input;
      setInput("");
      setHistoryIndex(-1);
      setCursorPosition(0);

      // Add non-empty commands to history
      if (cmd.trim()) {
        setCommandHistory((prev) => [...prev, cmd]);
      }

      if (isClearCommand(cmd)) {
        setLines([]);
        return;
      }

      setLines((prev) => trimToLast([...prev, { type: "prompt", content: prompt + cmd }]));

      if (sessionId && !sessionEnded) {
        setWaitingForPrompt(true);
        invoke("terminal_send_stdin", { sessionId, data: cmd + "\n" }).catch((err) => {
          setLines((prev) => trimToLast([...prev, { type: "error", content: String(err) }]));
          setWaitingForPrompt(false);
        });
      } else {
        const output = runMockCommand(cmd, username);
        const newLines: LineItem[] = output.map((line) => ({
          type: line.startsWith("Command not found") ? "error" : "output",
          content: line,
        }));
        setLines((prev) => trimToLast([...prev, ...newLines]));
      }
    },
    [
      input,
      prompt,
      sessionId,
      sessionEnded,
      username,
      commandHistory,
      historyIndex,
      cursorPosition,
      findPreviousWordStart,
      findNextWordStart,
    ]
  );

  const handleContainerMouseUp = useCallback((e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    if (target.closest(`.${styles.inputRow}`)) return;
    const selection = window.getSelection?.();
    if (!selection?.toString().trim()) {
      hiddenInputRef.current?.focus();
    }
  }, []);

  function handleConnectErrorModalClose() {
    setConnectErrorModalOpen(false);
    if (windowId) {
      close(windowId);
    }
  }

  function handleMemoryLimitModalClose() {
    setMemoryLimitModalOpen(false);
    if (windowId) {
      close(windowId);
    }
  }

  return (
    <div
      className={styles.terminal}
      onMouseDown={(e) => {
        const target = e.target as HTMLElement;
        if (target.closest(`.${styles.inputRow}`)) {
          e.preventDefault();
          hiddenInputRef.current?.focus();
        }
      }}
      onMouseUp={handleContainerMouseUp}
    >
      <Modal
        open={connectErrorModalOpen}
        onClose={handleConnectErrorModalClose}
        title="Unexpected Error"
        primaryButton={{ label: "OK", onClick: handleConnectErrorModalClose }}
      >
        <p>{CONNECT_ERROR_MESSAGE}</p>
      </Modal>
      <Modal
        open={memoryLimitModalOpen}
        onClose={handleMemoryLimitModalClose}
        title="Memory Limit Reached"
        primaryButton={{ label: "OK", onClick: handleMemoryLimitModalClose }}
      >
        <p className={styles.memoryModalText}>
          All processes have been killed.
        </p>
        <p className={styles.memoryModalSubtext}>
          You can open a new terminal session to continue.
        </p>
      </Modal>
      <div className={styles.scroll} ref={scrollRef}>
        {lines.map((line, i) => (
          <div
            key={i}
            className={`${line.type === "error" ? styles.lineError : styles.line} ${line.type !== "prompt" ? styles.lineAnimated : ""}`}
          >
            {line.type === "prompt" ? (
              <>
                <span className={styles.promptText}>{prompt}</span>
                <span className={styles.cmd}>{line.content.slice(prompt.length)}</span>
              </>
            ) : (
              renderLineContent(line.content, line.type === "error" ? "error" : "output")
            )}
          </div>
        ))}
        <div className={styles.inputRow} ref={inputRowRef}>
          {!waitingForPrompt && <span className={styles.promptText}>{prompt}</span>}
          <div className={styles.inputContainer}>
            {/* Hidden input for keyboard capture; disabled while waiting for prompt (command running) */}
            <input
              ref={hiddenInputRef}
              type="text"
              className={styles.hiddenInput}
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              spellCheck={false}
              autoComplete="off"
              autoCorrect="off"
              autoCapitalize="off"
              autoFocus
              aria-label="Command input"
              readOnly={waitingForPrompt}
              tabIndex={waitingForPrompt ? -1 : 0}
            />
            {/* Visual text display with cursor in the middle; hide when waiting for prompt */}
            {!waitingForPrompt && (
              <>
                <span className={styles.inputText}>{input.slice(0, cursorPosition)}</span>
                <span
                  className={`${styles.cursor} ${cursorVisible ? styles.cursorVisible : styles.cursorHidden}`}
                >
                  {input[cursorPosition] || "\u00A0"}
                </span>
                <span className={styles.inputText}>{input.slice(cursorPosition + 1)}</span>
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
