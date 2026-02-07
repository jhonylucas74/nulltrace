import { useState, useEffect } from "react";
import { MOCK_WALLET_BALANCES } from "../lib/walletBalances";
import styles from "./WalletApp.module.css";

const STORAGE_KEY = "wallet-visible-currencies";

function loadVisibleSymbols(): Set<string> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const arr = JSON.parse(raw) as string[];
      if (Array.isArray(arr)) return new Set(arr);
    }
  } catch {
    /* ignore */
  }
  return new Set(MOCK_WALLET_BALANCES.map((b) => b.symbol));
}

function saveVisibleSymbols(symbols: Set<string>) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify([...symbols]));
}

type View = "main" | "select";

export default function WalletApp() {
  const [visibleSymbols, setVisibleSymbols] = useState<Set<string>>(loadVisibleSymbols);
  const [view, setView] = useState<View>("main");
  const [draftSelection, setDraftSelection] = useState<Set<string>>(visibleSymbols);

  useEffect(() => {
    saveVisibleSymbols(visibleSymbols);
  }, [visibleSymbols]);

  const openSelectPage = () => {
    setDraftSelection(new Set(visibleSymbols));
    setView("select");
  };

  const toggleDraft = (symbol: string) => {
    setDraftSelection((prev) => {
      const next = new Set(prev);
      if (next.has(symbol)) {
        if (next.size <= 1) return prev;
        next.delete(symbol);
      } else next.add(symbol);
      return next;
    });
  };

  const confirmSelection = () => {
    setVisibleSymbols(new Set(draftSelection));
    setView("main");
  };

  const visibleBalances = MOCK_WALLET_BALANCES.filter((b) => visibleSymbols.has(b.symbol));

  if (view === "select") {
    return (
      <div className={styles.app}>
        <div className={styles.selectPage}>
          <div className={styles.selectHeader}>
            <button
              type="button"
              className={styles.backBtn}
              onClick={() => setView("main")}
              title="Back"
              aria-label="Back"
            >
              <BackIcon />
            </button>
            <h2 className={styles.selectTitle}>Select currencies</h2>
          </div>
          <p className={styles.selectSubtitle}>
            Choose which currencies to show on your wallet. At least one must be selected.
          </p>
          <ul className={styles.selectList}>
            {MOCK_WALLET_BALANCES.map((b) => {
              const isChecked = draftSelection.has(b.symbol);
              const isOnlyOne = isChecked && draftSelection.size === 1;
              return (
                <li key={b.symbol}>
                  <button
                    type="button"
                    className={styles.selectItem}
                    onClick={() => !isOnlyOne && toggleDraft(b.symbol)}
                    aria-pressed={isChecked}
                    disabled={isOnlyOne}
                  >
                    <span className={styles.selectItemCheck}>{isChecked ? <CheckIcon /> : null}</span>
                    <span className={styles.selectItemSymbol}>{b.symbol}</span>
                    <span className={styles.selectItemCurrency}>{b.currency}</span>
                  </button>
                </li>
              );
            })}
          </ul>
          <div className={styles.selectFooter}>
            <button type="button" className={styles.doneBtn} onClick={confirmSelection}>
              Done
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className={styles.app}>
      <div className={styles.header}>
        <div>
          <h2 className={styles.title}>Wallet</h2>
          <p className={styles.subtitle}>Your balances.</p>
        </div>
        <button
          type="button"
          className={styles.configBtn}
          onClick={openSelectPage}
          title="Choose which currencies to show"
        >
          Manage currencies
        </button>
      </div>
      <div className={styles.cards}>
        {visibleBalances.map((item) => (
          <div key={item.symbol} className={styles.card}>
            <div className={styles.cardHeader}>
              <span className={styles.symbol}>{item.symbol}</span>
              <span className={styles.currency}>{item.currency}</span>
            </div>
            <div className={styles.amount}>{item.amount}</div>
          </div>
        ))}
      </div>
    </div>
  );
}

function BackIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M19 12H5M12 19l-7-7 7-7" />
    </svg>
  );
}

function CheckIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
      <polyline points="20 6 9 17 4 12" />
    </svg>
  );
}
