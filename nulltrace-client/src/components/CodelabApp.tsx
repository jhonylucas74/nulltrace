import { useState, useCallback, useRef, useEffect, useMemo } from "react";
import {
  Search,
  ArrowLeft,
  Play,
  Send,
  CheckCircle2,
  XCircle,
  CircleCheck,
  Trophy,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { highlightLua } from "../lib/luaHighlight";
import {
  CHALLENGES,
  type Challenge,
  type Difficulty,
  type TestCase,
} from "../lib/codelabChallenges";
import styles from "./CodelabApp.module.css";

/* ── Tauri bridge ────────────────────────────────── */

interface LuauResult {
  success: boolean;
  output: string[];
  error: string | null;
}

async function runLuau(code: string): Promise<LuauResult> {
  try {
    return await invoke<LuauResult>("run_luau", { code });
  } catch {
    // Fallback when Tauri is not available (e.g. browser dev mode)
    return { success: false, output: [], error: "Tauri runtime not available." };
  }
}

/* ── Persistence ─────────────────────────────────── */

const STORAGE_KEY = "codelab_solved";

function getSolvedSet(): Set<string> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return new Set(JSON.parse(raw));
  } catch {
    /* ignore */
  }
  return new Set();
}

function markSolved(id: string) {
  const set = getSolvedSet();
  set.add(id);
  localStorage.setItem(STORAGE_KEY, JSON.stringify([...set]));
}

/* ── Constants ───────────────────────────────────── */

const LINE_HEIGHT = 1.5;
const EDITOR_FONT_SIZE = "0.9rem";
const DIFFICULTY_ORDER: Difficulty[] = ["easy", "medium", "hard"];
const FILTER_ALL = "all";

/* ── Component ───────────────────────────────────── */

export default function CodelabApp() {
  const [activeChallenge, setActiveChallenge] = useState<Challenge | null>(null);
  const [searchTerm, setSearchTerm] = useState("");
  const [diffFilter, setDiffFilter] = useState<Difficulty | typeof FILTER_ALL>(FILTER_ALL);
  const [solvedIds, setSolvedIds] = useState<Set<string>>(() => getSolvedSet());

  // Challenge view state
  const [code, setCode] = useState("");
  const [consoleLogs, setConsoleLogs] = useState<
    Array<{ type: "stdout" | "error" | "system"; text: string }>
  >([]);
  const [testResults, setTestResults] = useState<
    Array<{ testCase: TestCase; passed: boolean; actual: string }> | null
  >(null);
  const [running, setRunning] = useState(false);
  const [showSuccess, setShowSuccess] = useState(false);

  const editorRef = useRef<HTMLTextAreaElement | null>(null);
  const gutterRef = useRef<HTMLDivElement | null>(null);
  const highlightRef = useRef<HTMLDivElement | null>(null);
  const consoleEndRef = useRef<HTMLDivElement | null>(null);
  const pendingTabRef = useRef<number | null>(null);

  /* ── Filtered list ──────────────────────────────── */

  const filteredChallenges = useMemo(() => {
    const q = searchTerm.toLowerCase().trim();
    return CHALLENGES.filter((c) => {
      if (diffFilter !== FILTER_ALL && c.difficulty !== diffFilter) return false;
      if (q && !c.title.toLowerCase().includes(q) && !c.description.toLowerCase().includes(q))
        return false;
      return true;
    });
  }, [searchTerm, diffFilter]);

  /* ── Open challenge ─────────────────────────────── */

  const openChallenge = useCallback((c: Challenge) => {
    setActiveChallenge(c);
    setCode(c.starterCode);
    setConsoleLogs([]);
    setTestResults(null);
    setShowSuccess(false);
  }, []);

  const goBack = useCallback(() => {
    setActiveChallenge(null);
    setCode("");
    setConsoleLogs([]);
    setTestResults(null);
    setShowSuccess(false);
  }, []);

  /* ── Editor helpers ─────────────────────────────── */

  function handleEditorChange(e: React.ChangeEvent<HTMLTextAreaElement>) {
    setCode(e.target.value);
  }

  function handleEditorKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key !== "Tab") return;
    e.preventDefault();
    const ta = editorRef.current;
    if (!ta) return;
    const start = ta.selectionStart;
    const end = ta.selectionEnd;
    const tab = "  ";
    const next = code.slice(0, start) + tab + code.slice(end);
    pendingTabRef.current = start + tab.length;
    setCode(next);
  }

  useEffect(() => {
    if (pendingTabRef.current === null) return;
    const pos = pendingTabRef.current;
    pendingTabRef.current = null;
    const ta = editorRef.current;
    if (ta) {
      ta.focus();
      ta.setSelectionRange(pos, pos);
    }
  }, [code]);

  function handleTextareaScroll() {
    const top = editorRef.current?.scrollTop ?? 0;
    if (gutterRef.current) gutterRef.current.scrollTop = top;
    if (highlightRef.current) highlightRef.current.scrollTop = top;
  }

  const lineCount = Math.max(1, code.split("\n").length);

  /* Auto-scroll console */
  useEffect(() => {
    consoleEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [consoleLogs, testResults]);

  /* ── Run / Submit ───────────────────────────────── */

  const handleRun = useCallback(async () => {
    if (!activeChallenge || running) return;
    setRunning(true);
    setTestResults(null);
    setConsoleLogs([{ type: "system", text: "> Running with test case 1..." }]);

    const tc = activeChallenge.testCases[0];
    const fullCode = code + "\n" + tc.setupCode;
    const result = await runLuau(fullCode);

    if (result.success) {
      result.output.forEach((line) =>
        setConsoleLogs((prev) => [...prev, { type: "stdout", text: line }])
      );
      setConsoleLogs((prev) => [...prev, { type: "system", text: "Done." }]);
    } else {
      if (result.output.length > 0) {
        result.output.forEach((line) =>
          setConsoleLogs((prev) => [...prev, { type: "stdout", text: line }])
        );
      }
      setConsoleLogs((prev) => [
        ...prev,
        { type: "error", text: result.error ?? "Unknown error" },
      ]);
    }
    setRunning(false);
  }, [activeChallenge, code, running]);

  const handleSubmit = useCallback(async () => {
    if (!activeChallenge || running) return;
    setRunning(true);
    setConsoleLogs([{ type: "system", text: "> Submitting... running all test cases." }]);
    setTestResults(null);
    setShowSuccess(false);

    const results: Array<{ testCase: TestCase; passed: boolean; actual: string }> = [];

    for (const tc of activeChallenge.testCases) {
      const fullCode = code + "\n" + tc.setupCode;
      const result = await runLuau(fullCode);

      const actual = result.success
        ? result.output.join("\n")
        : result.error ?? "Error";
      const passed = result.success && actual.trim() === tc.expectedOutput.trim();

      results.push({ testCase: tc, passed, actual });
    }

    setTestResults(results);

    const allPassed = results.every((r) => r.passed);
    if (allPassed) {
      setConsoleLogs((prev) => [
        ...prev,
        { type: "system", text: "All test cases passed!" },
      ]);
      markSolved(activeChallenge.id);
      setSolvedIds((prev) => new Set([...prev, activeChallenge.id]));
      setShowSuccess(true);
    } else {
      const passCount = results.filter((r) => r.passed).length;
      setConsoleLogs((prev) => [
        ...prev,
        {
          type: "error",
          text: `${passCount}/${results.length} test cases passed. Keep trying!`,
        },
      ]);
    }
    setRunning(false);
  }, [activeChallenge, code, running]);

  /* ── Render: List View ──────────────────────────── */

  if (!activeChallenge) {
    return (
      <div className={styles.app}>
        <div className={styles.listView}>
          <div className={styles.header}>
            <span className={styles.headerTitle}>Codelab</span>
            <div className={styles.searchWrap}>
              <Search size={16} className={styles.searchIcon} />
              <input
                type="text"
                className={styles.searchInput}
                placeholder="Search challenges..."
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
              />
            </div>
          </div>

          <div className={styles.filters}>
            <button
              type="button"
              className={`${styles.filterPill} ${diffFilter === FILTER_ALL ? styles.filterPillActive : ""}`}
              onClick={() => setDiffFilter(FILTER_ALL)}
            >
              All
            </button>
            {DIFFICULTY_ORDER.map((d) => (
              <button
                key={d}
                type="button"
                className={`${styles.filterPill} ${diffFilter === d ? styles.filterPillActive : ""}`}
                onClick={() => setDiffFilter(d)}
              >
                {d.charAt(0).toUpperCase() + d.slice(1)}
              </button>
            ))}
          </div>

          <div className={styles.challengeList}>
            {filteredChallenges.length === 0 ? (
              <div className={styles.emptyState}>No challenges found.</div>
            ) : (
              filteredChallenges.map((c, idx) => (
                <button
                  key={c.id}
                  type="button"
                  className={styles.challengeRow}
                  onClick={() => openChallenge(c)}
                >
                  <span className={styles.challengeIndex}>{idx + 1}</span>
                  <span className={styles.challengeTitle}>{c.title}</span>
                  <span
                    className={`${styles.badge} ${
                      c.difficulty === "easy"
                        ? styles.badgeEasy
                        : c.difficulty === "medium"
                          ? styles.badgeMedium
                          : styles.badgeHard
                    }`}
                  >
                    {c.difficulty}
                  </span>
                  {solvedIds.has(c.id) && (
                    <CircleCheck size={16} className={styles.solvedIcon} />
                  )}
                </button>
              ))
            )}
          </div>
        </div>
      </div>
    );
  }

  /* ── Render: Challenge View ─────────────────────── */

  return (
    <div className={styles.app}>
      <div className={styles.challengeView}>
        {/* Top bar */}
        <div className={styles.challengeHeader}>
          <button type="button" className={styles.backBtn} onClick={goBack}>
            <ArrowLeft size={14} /> Back
          </button>
          <span className={styles.challengeHeaderTitle}>{activeChallenge.title}</span>
          <div className={styles.challengeHeaderActions}>
            <button
              type="button"
              className={styles.runBtn}
              onClick={handleRun}
              disabled={running}
            >
              <Play size={14} /> Run
            </button>
            <button
              type="button"
              className={styles.submitBtn}
              onClick={handleSubmit}
              disabled={running}
            >
              <Send size={14} /> Submit
            </button>
          </div>
        </div>

        <div className={styles.splitBody}>
          {/* Left: description */}
          <div className={styles.descriptionPanel}>
            <div className={styles.descriptionContent}>
              <h2 className={styles.descTitle}>{activeChallenge.title}</h2>
              <span
                className={`${styles.badge} ${styles.descBadge} ${
                  activeChallenge.difficulty === "easy"
                    ? styles.badgeEasy
                    : activeChallenge.difficulty === "medium"
                      ? styles.badgeMedium
                      : styles.badgeHard
                }`}
              >
                {activeChallenge.difficulty}
              </span>

              <p className={styles.descText}>{activeChallenge.description}</p>

              <div className={styles.descSectionTitle}>Examples</div>
              <pre className={styles.descExamples}>{activeChallenge.examples}</pre>

              <div className={styles.descSectionTitle}>Test Cases</div>
              <div className={styles.descTestCases}>
                {activeChallenge.testCases.map((tc) => (
                  <div key={tc.id} className={styles.testCaseItem}>
                    <div className={styles.testCaseLabel}>Test {tc.id}</div>
                    <div>Input: {tc.input}</div>
                    <div>Expected: {tc.expectedOutput}</div>
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* Right: editor + console */}
          <div className={styles.editorPanel}>
            <div className={styles.editorAndConsole}>
              <div className={styles.editorWithGutter}>
                <div
                  ref={gutterRef}
                  className={styles.gutter}
                  style={{ lineHeight: LINE_HEIGHT, fontSize: EDITOR_FONT_SIZE }}
                >
                  {Array.from({ length: lineCount }, (_, i) => (
                    <div key={i} className={styles.gutterLine}>
                      {i + 1}
                    </div>
                  ))}
                </div>
                <div className={styles.editorScrollWrap}>
                  <div
                    ref={highlightRef}
                    className={styles.highlightLayer}
                    style={{ lineHeight: LINE_HEIGHT, fontSize: EDITOR_FONT_SIZE }}
                    aria-hidden
                    dangerouslySetInnerHTML={{ __html: highlightLua(code) }}
                  />
                  <textarea
                    ref={editorRef}
                    className={`${styles.textarea} ${styles.textareaHighlight}`}
                    value={code}
                    onChange={handleEditorChange}
                    onKeyDown={handleEditorKeyDown}
                    onScroll={handleTextareaScroll}
                    spellCheck={false}
                    style={{ lineHeight: LINE_HEIGHT, fontSize: EDITOR_FONT_SIZE }}
                  />
                </div>
              </div>

              {/* Console */}
              <div className={styles.consolePanel}>
                <div className={styles.consoleHeader}>Console</div>
                <div className={styles.consoleOutput}>
                  {consoleLogs.length === 0 && testResults === null ? (
                    <div className={styles.consoleEmpty}>
                      Click Run to test or Submit to verify all test cases.
                    </div>
                  ) : (
                    <>
                      {consoleLogs.map((entry, i) => (
                        <div
                          key={i}
                          className={
                            entry.type === "error"
                              ? styles.consoleLineError
                              : entry.type === "system"
                                ? styles.consoleLineSystem
                                : styles.consoleLine
                          }
                        >
                          {entry.text}
                        </div>
                      ))}
                      {testResults &&
                        testResults.map((r) => (
                          <div key={r.testCase.id} className={styles.testResult}>
                            <span
                              className={`${styles.testResultIcon} ${
                                r.passed ? styles.testPass : styles.testFail
                              }`}
                            >
                              {r.passed ? (
                                <CheckCircle2 size={16} />
                              ) : (
                                <XCircle size={16} />
                              )}
                            </span>
                            <div className={styles.testResultBody}>
                              <div className={styles.testResultLabel}>
                                Test {r.testCase.id}: {r.passed ? "Passed" : "Failed"}
                              </div>
                              {!r.passed && (
                                <div className={styles.testResultDetail}>
                                  Expected: {r.testCase.expectedOutput}
                                  {"\n"}Got: {r.actual}
                                </div>
                              )}
                            </div>
                          </div>
                        ))}
                    </>
                  )}
                  <div ref={consoleEndRef} />
                </div>
              </div>
            </div>

            {/* Success overlay */}
            {showSuccess && (
              <div className={styles.successOverlay}>
                <Trophy size={48} className={styles.successIcon} />
                <div className={styles.successTitle}>Challenge Completed!</div>
                <div className={styles.successSubtitle}>
                  All 3 test cases passed.
                </div>
                <button
                  type="button"
                  className={styles.successBtn}
                  onClick={goBack}
                >
                  Back to Challenges
                </button>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
