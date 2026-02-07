import { useState, useRef, useEffect, KeyboardEvent } from "react";
import { runMockCommand, isClearCommand } from "../lib/mockCommands";
import styles from "./Terminal.module.css";

interface TerminalProps {
  username: string;
}

interface LineItem {
  type: "prompt" | "output" | "error";
  content: string;
}

export default function Terminal({ username }: TerminalProps) {
  const [lines, setLines] = useState<LineItem[]>([
    { type: "output", content: "Welcome to nulltrace. Type 'help' for commands." },
  ]);
  const [input, setInput] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    scrollRef.current?.scrollTo(0, scrollRef.current.scrollHeight);
  }, [lines, input]);

  const prompt = `${username}@nulltrace:~$ `;

  function handleKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key !== "Enter") return;
    e.preventDefault();
    const cmd = input;
    setInput("");

    if (isClearCommand(cmd)) {
      setLines([]);
      return;
    }

    setLines((prev) => [...prev, { type: "prompt", content: prompt + cmd }]);

    const output = runMockCommand(cmd, username);
    const newLines: LineItem[] = output.map((line) => ({
      type: line.startsWith("Command not found") ? "error" : "output",
      content: line,
    }));
    setLines((prev) => [...prev, ...newLines]);
  }

  return (
    <div
      className={styles.terminal}
      onMouseDown={(e) => {
        e.preventDefault();
        inputRef.current?.focus();
      }}
    >
      <div className={styles.scroll} ref={scrollRef}>
        {lines.map((line, i) => (
          <div key={i} className={line.type === "error" ? styles.lineError : styles.line}>
            {line.type === "prompt" ? (
              <>
                <span className={styles.promptText}>{prompt}</span>
                <span className={styles.cmd}>{line.content.slice(prompt.length)}</span>
              </>
            ) : (
              line.content
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
