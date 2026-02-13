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
  type: "stdout" | "closed" | "error";
  data?: string;
}

const CONNECT_ERROR_MESSAGE =
  "Unexpected error. Could not load the shell (missing or corrupted). The terminal will close.";

const MAX_LINES = 100;

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
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const sessionIdRef = useRef<string | null>(null);
  sessionIdRef.current = sessionId;

  // Effect "generation": only the latest mount's deferred connect runs (avoids double shell in React Strict Mode).
  const connectGenerationRef = useRef(0);

  useEffect(() => {
    scrollRef.current?.scrollTo(0, scrollRef.current.scrollHeight);
  }, [lines, input]);

  const prompt = `${username}@nulltrace:~$ `;

  // Connect to VM shell when we have playerId (Tauri only).
  // Deferred so that in React Strict Mode only the final mount opens one terminal (not two).
  useEffect(() => {
    if (!playerId || !token) return;

    const generation = ++connectGenerationRef.current;
    let unlisten: (() => void) | undefined;

    const runConnect = async () => {
      if (generation !== connectGenerationRef.current) return;
      try {
        const sid = await invoke<string>("terminal_connect", { playerId, token });
        if (generation !== connectGenerationRef.current) {
          invoke("terminal_disconnect", { sessionId: sid }).catch(() => {});
          return;
        }
        sessionIdRef.current = sid;
        setSessionId(sid);
        setSessionEnded(false);
        setLines([]);

        unlisten = await listen<TerminalOutputPayload>("terminal-output", (event) => {
          const payload = event.payload;
          if (payload.sessionId !== sid) return;

          if (payload.type === "stdout" && payload.data !== undefined) {
            setLines((prev) =>
              trimToLast([
                ...prev,
                ...payload.data!
                  .split("\n")
                  .filter((s) => s.length > 0)
                  .map((content) => ({ type: "output" as const, content })),
              ])
            );
          } else if (payload.type === "closed") {
            setSessionEnded(true);
            setLines((prev) => trimToLast([...prev, { type: "output", content: "Session ended." }]));
            sessionIdRef.current = null;
            setSessionId(null);
          } else if (payload.type === "error" && payload.data) {
            const msg = payload.data;
            setLines((prev) => trimToLast([...prev, { type: "error", content: msg }]));
            setSessionEnded(true);
            sessionIdRef.current = null;
            setSessionId(null);
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
      if (e.key !== "Enter") return;
      e.preventDefault();
      const cmd = input;
      setInput("");

      if (isClearCommand(cmd)) {
        setLines([]);
        return;
      }

      setLines((prev) => trimToLast([...prev, { type: "prompt", content: prompt + cmd }]));

      if (sessionId && !sessionEnded) {
        invoke("terminal_send_stdin", { sessionId, data: cmd + "\n" }).catch((err) => {
          setLines((prev) => trimToLast([...prev, { type: "error", content: String(err) }]));
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
    [input, prompt, sessionId, sessionEnded, username]
  );

  const handleContainerMouseUp = useCallback((e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    if (target.closest(`.${styles.inputRow}`)) return;
    const selection = window.getSelection?.();
    if (!selection?.toString().trim()) {
      inputRef.current?.focus();
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
          inputRef.current?.focus();
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
          <div key={i} className={line.type === "error" ? styles.lineError : styles.line}>
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
        <div className={styles.inputRow}>
          <span className={styles.promptText}>{prompt}</span>
          <input
            ref={inputRef}
            type="text"
            className={styles.input}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            spellCheck={false}
            autoFocus
            aria-label="Command input"
          />
        </div>
      </div>
    </div>
  );
}
