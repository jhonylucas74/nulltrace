import { useState, useCallback, useEffect, useRef } from "react";
import Modal from "./Modal";
import styles from "./MinesweeperApp.module.css";

const ROWS = 9;
const COLS = 9;
const MINES = 10;

type CellState = "hidden" | "revealed" | "flagged";

interface Cell {
  isMine: boolean;
  adjacentMines: number;
  state: CellState;
}

type GameStatus = "playing" | "won" | "lost" | null;

function createEmptyGrid(): Cell[][] {
  return Array(ROWS)
    .fill(null)
    .map(() =>
      Array(COLS)
        .fill(null)
        .map(() => ({
          isMine: false,
          adjacentMines: 0,
          state: "hidden" as CellState,
        }))
    );
}

function placeMinesAndCounts(grid: Cell[][], excludeRow: number, excludeCol: number): void {
  const exclude = new Set<string>();
  for (let dr = -1; dr <= 1; dr++) {
    for (let dc = -1; dc <= 1; dc++) {
      const r = excludeRow + dr;
      const c = excludeCol + dc;
      if (r >= 0 && r < ROWS && c >= 0 && c < COLS) exclude.add(`${r},${c}`);
    }
  }
  let placed = 0;
  while (placed < MINES) {
    const r = Math.floor(Math.random() * ROWS);
    const c = Math.floor(Math.random() * COLS);
    if (exclude.has(`${r},${c}`) || grid[r][c].isMine) continue;
    grid[r][c].isMine = true;
    placed++;
  }
  for (let r = 0; r < ROWS; r++) {
    for (let c = 0; c < COLS; c++) {
      if (grid[r][c].isMine) continue;
      let count = 0;
      for (let dr = -1; dr <= 1; dr++) {
        for (let dc = -1; dc <= 1; dc++) {
          const nr = r + dr;
          const nc = c + dc;
          if (nr >= 0 && nr < ROWS && nc >= 0 && nc < COLS && grid[nr][nc].isMine) count++;
        }
      }
      grid[r][c].adjacentMines = count;
    }
  }
}

function floodReveal(grid: Cell[][], startR: number, startC: number): void {
  const stack: [number, number][] = [[startR, startC]];
  while (stack.length > 0) {
    const [r, c] = stack.pop()!;
    const cell = grid[r][c];
    if (cell.state === "revealed" || cell.state === "flagged") continue;
    cell.state = "revealed";
    if (cell.isMine) continue;
    if (cell.adjacentMines === 0) {
      for (let dr = -1; dr <= 1; dr++) {
        for (let dc = -1; dc <= 1; dc++) {
          const nr = r + dr;
          const nc = c + dc;
          if (nr >= 0 && nr < ROWS && nc >= 0 && nc < COLS) stack.push([nr, nc]);
        }
      }
    }
  }
}

export default function MinesweeperApp() {
  const [grid, setGrid] = useState<Cell[][]>(() => createEmptyGrid());
  const [gameStarted, setGameStarted] = useState(false);
  const [gameStatus, setGameStatus] = useState<GameStatus>(null);
  const [elapsedSeconds, setElapsedSeconds] = useState(0);
  const gameStartTimeRef = useRef<number | null>(null);

  const revealedCount = grid.flat().filter((c) => c.state === "revealed").length;
  const nonMineCount = ROWS * COLS - MINES;

  useEffect(() => {
    if (gameStarted && gameStatus === "playing" && revealedCount === nonMineCount) {
      setGameStatus("won");
    }
  }, [gameStarted, gameStatus, revealedCount, nonMineCount]);

  useEffect(() => {
    if (!gameStarted || gameStatus !== "playing") return;
    const interval = setInterval(() => {
      if (gameStartTimeRef.current != null) {
        setElapsedSeconds(Math.floor((Date.now() - gameStartTimeRef.current) / 1000));
      }
    }, 1000);
    return () => clearInterval(interval);
  }, [gameStarted, gameStatus]);

  const handleNewGame = useCallback(() => {
    setGrid(createEmptyGrid());
    setGameStarted(false);
    setGameStatus(null);
    setElapsedSeconds(0);
    gameStartTimeRef.current = null;
  }, []);

  const revealCell = useCallback(
    (r: number, c: number) => {
      if (gameStatus === "won" || gameStatus === "lost") return;
      const cell = grid[r][c];
      if (cell.state === "revealed" || cell.state === "flagged") return;

      if (!gameStarted) {
        gameStartTimeRef.current = Date.now();
        setElapsedSeconds(0);
        setGrid((prev) => {
          const next = prev.map((row) => row.map((cell) => ({ ...cell })));
          placeMinesAndCounts(next, r, c);
          floodReveal(next, r, c);
          return next;
        });
        setGameStarted(true);
        setGameStatus("playing");
        return;
      }

      if (cell.isMine) {
        setGrid((prev) =>
          prev.map((row) =>
            row.map((cell) => ({ ...cell, state: cell.isMine ? "revealed" as CellState : cell.state }))
          )
        );
        setGameStatus("lost");
        return;
      }

      setGrid((prev) => {
        const next = prev.map((row) => row.map((cell) => ({ ...cell })));
        floodReveal(next, r, c);
        return next;
      });
    },
    [gameStatus, gameStarted, grid]
  );

  const toggleFlag = useCallback(
    (r: number, c: number) => {
      if (gameStatus === "won" || gameStatus === "lost") return;
      if (grid[r][c].state === "revealed") return;
      setGrid((prev) => {
        const next = prev.map((row) => row.map((cell) => ({ ...cell })));
        const cell = next[r][c];
        cell.state = cell.state === "flagged" ? "hidden" : "flagged";
        return next;
      });
    },
    [gameStatus, grid]
  );

  return (
    <div className={styles.app}>
      <div className={styles.toolbar}>
        <button type="button" className={styles.newGameBtn} onClick={handleNewGame}>
          New game
        </button>
        <span className={styles.timer} aria-live="polite">
          Time: {elapsedSeconds}s
        </span>
        {gameStatus === "won" && <span className={styles.status}>You win!</span>}
        {gameStatus === "lost" && <span className={styles.statusLost}>Game over</span>}
      </div>
      <p className={styles.hint}>
        Left click: reveal. Right click: mark bomb.
      </p>
      <div className={styles.gameArea}>
        <div className={styles.grid} style={{ "--cols": COLS } as React.CSSProperties}>
          {grid.map((row, r) =>
            row.map((cell, c) => (
              <button
                key={`${r}-${c}`}
                type="button"
                className={`${styles.cell} ${styles[`cell_${cell.state}`]} ${
                  gameStatus === "lost" && cell.isMine ? styles.cell_mine : ""
                }`}
                onClick={() => revealCell(r, c)}
                onContextMenu={(e) => {
                  e.preventDefault();
                  toggleFlag(r, c);
                }}
                disabled={gameStatus === "won" || gameStatus === "lost"}
                aria-label={`Cell ${r + 1} ${c + 1}`}
              >
                {cell.state === "flagged" && "F"}
                {cell.state === "revealed" && !cell.isMine && (cell.adjacentMines > 0 ? cell.adjacentMines : "")}
                {cell.state === "revealed" && cell.isMine && "*"}
                {gameStatus === "lost" && cell.isMine && cell.state !== "revealed" && "*"}
              </button>
            ))
          )}
        </div>
      </div>

      <Modal
        open={gameStatus === "lost"}
        onClose={() => {}}
        title="Game over"
        primaryButton={{ label: "Restart", onClick: handleNewGame }}
      >
        <p className={styles.modalText}>You hit a mine. Better luck next time!</p>
      </Modal>

      <Modal
        open={gameStatus === "won"}
        onClose={() => {}}
        title="You win!"
        primaryButton={{ label: "Restart", onClick: handleNewGame }}
      >
        <p className={styles.modalText}>
          You finished in <strong>{elapsedSeconds}</strong> seconds.
        </p>
      </Modal>
    </div>
  );
}
