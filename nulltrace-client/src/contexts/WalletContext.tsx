/**
 * Wallet state for balances and transactions. Used by WalletApp for
 * transfer, convert, and statement updates.
 */

import React, { createContext, useContext, useCallback, useMemo, useState } from "react";
import {
  MOCK_WALLET_BALANCES,
  type WalletBalance,
} from "../lib/walletBalances";
import {
  MOCK_WALLET_TRANSACTIONS,
  nextTransactionId,
  type WalletTransaction,
} from "../lib/walletTransactions";
import {
  MOCK_CARD_DEBT,
  MOCK_CARD_LIMIT,
  MOCK_CARD_TRANSACTIONS,
  type CardTransaction,
} from "../lib/walletCards";

/**
 * Parse amount string to number. Accepts comma or dot as decimal separator.
 * Last occurrence of comma or dot is the decimal separator; others are thousand separators.
 * E.g. "1,50" -> 1.5, "1.240,50" -> 1240.5, "1,240.50" -> 1240.5.
 */
export function parseAmount(s: string): number {
  const raw = s.trim().replace(/\s/g, "");
  if (!raw) return 0;
  const lastComma = raw.lastIndexOf(",");
  const lastDot = raw.lastIndexOf(".");
  const decimalIndex = lastComma > lastDot ? lastComma : lastDot;
  let normalized: string;
  if (decimalIndex >= 0) {
    const intPart = raw.slice(0, decimalIndex).replace(/[.,]/g, "");
    const decPart = raw.slice(decimalIndex + 1).replace(/[^0-9]/g, "");
    normalized = intPart ? `${intPart}.${decPart}` : `0.${decPart}`;
  } else {
    normalized = raw.replace(/[.,]/g, "");
  }
  const n = parseFloat(normalized);
  return Number.isFinite(n) ? n : 0;
}

/**
 * Apply mask to amount input: only digits and one comma or dot for decimals.
 * Optionally limit decimal places (e.g. 2 for USD).
 */
export function applyAmountMask(value: string, maxDecimals: number = 8): string {
  let hasDecimal = false;
  let decimalCount = 0;
  const out: string[] = [];
  for (const c of value) {
    if (c >= "0" && c <= "9") {
      if (hasDecimal) {
        if (decimalCount < maxDecimals) {
          out.push(c);
          decimalCount++;
        }
      } else {
        out.push(c);
      }
      continue;
    }
    if ((c === "," || c === ".") && !hasDecimal) {
      out.push(c);
      hasDecimal = true;
      continue;
    }
  }
  return out.join("");
}

/** Format number for display (2–8 decimals depending on currency). */
export function formatAmount(value: number, symbol: string): string {
  if (symbol === "USD") return value.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  if (symbol === "BTC") return value.toLocaleString("en-US", { minimumFractionDigits: 4, maximumFractionDigits: 6 });
  if (symbol === "ETH" || symbol === "SOL") return value.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 4 });
  return value.toFixed(2);
}

interface WalletContextValue {
  /** Balance by symbol (numeric). */
  balances: Record<string, number>;
  /** All transactions (newest first). */
  transactions: WalletTransaction[];
  /** Currency metadata (symbol -> WalletBalance) for labels. */
  balanceMeta: Map<string, WalletBalance>;
  /** Get formatted amount for a symbol. */
  getFormattedBalance: (symbol: string) => string;
  /** Execute transfer: deduct from balance and add transaction. */
  transfer: (currency: string, amount: number, recipientKey: string) => boolean;
  /** Execute conversion: deduct from, add to, append two transactions. */
  convert: (fromSymbol: string, toSymbol: string, fromAmount: number, rate: number) => boolean;
  /** Card (Fkebank): current debt in USD. */
  cardDebt: number;
  /** Card (Fkebank): credit limit in USD. */
  cardLimit: number;
  /** Card-only statement (newest first). */
  cardTransactions: CardTransaction[];
}

const WalletContext = createContext<WalletContextValue | null>(null);

function initialBalances(): Record<string, number> {
  const out: Record<string, number> = {};
  for (const b of MOCK_WALLET_BALANCES) {
    out[b.symbol] = parseAmount(b.amount);
  }
  return out;
}

export function WalletProvider({ children }: { children: React.ReactNode }) {
  const [balances, setBalances] = useState<Record<string, number>>(initialBalances);
  const [transactions, setTransactions] = useState<WalletTransaction[]>(MOCK_WALLET_TRANSACTIONS);
  const [cardDebt] = useState<number>(MOCK_CARD_DEBT);
  const [cardLimit] = useState<number>(MOCK_CARD_LIMIT);
  const [cardTransactions] = useState<CardTransaction[]>(MOCK_CARD_TRANSACTIONS);

  const balanceMeta = useMemo(() => {
    const m = new Map<string, WalletBalance>();
    for (const b of MOCK_WALLET_BALANCES) m.set(b.symbol, b);
    return m;
  }, []);

  const getFormattedBalance = useCallback(
    (symbol: string) => formatAmount(balances[symbol] ?? 0, symbol),
    [balances]
  );

  const transfer = useCallback(
    (currency: string, amount: number, recipientKey: string): boolean => {
      const current = balances[currency] ?? 0;
      if (amount <= 0 || amount > current) return false;
      setBalances((prev) => ({ ...prev, [currency]: (prev[currency] ?? 0) - amount }));
      const tx: WalletTransaction = {
        id: nextTransactionId(),
        type: "transfer",
        amount: formatAmount(amount, currency),
        currency,
        date: new Date().toLocaleString("en-US", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" }),
        timestamp: Date.now(),
        description: `Transfer to ${recipientKey.slice(0, 8)}…`,
      };
      setTransactions((prev) => [tx, ...prev]);
      return true;
    },
    [balances]
  );

  const convert = useCallback(
    (fromSymbol: string, toSymbol: string, fromAmount: number, rate: number): boolean => {
      const fromBalance = balances[fromSymbol] ?? 0;
      if (fromAmount <= 0 || fromAmount > fromBalance || fromSymbol === toSymbol) return false;
      const toAmount = fromAmount * rate;
      setBalances((prev) => ({
        ...prev,
        [fromSymbol]: (prev[fromSymbol] ?? 0) - fromAmount,
        [toSymbol]: (prev[toSymbol] ?? 0) + toAmount,
      }));
      const dateStr = new Date().toLocaleString("en-US", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
      const now = Date.now();
      const debitTx: WalletTransaction = {
        id: nextTransactionId(),
        type: "convert",
        amount: `-${formatAmount(fromAmount, fromSymbol)}`,
        currency: fromSymbol,
        date: dateStr,
        timestamp: now,
        description: `Convert ${fromSymbol} → ${toSymbol}`,
      };
      const creditTx: WalletTransaction = {
        id: nextTransactionId(),
        type: "credit",
        amount: formatAmount(toAmount, toSymbol),
        currency: toSymbol,
        date: dateStr,
        timestamp: now,
        description: `Convert ${fromSymbol} → ${toSymbol}`,
      };
      setTransactions((prev) => [debitTx, creditTx, ...prev]);
      return true;
    },
    [balances]
  );

  const value: WalletContextValue = useMemo(
    () => ({
      balances,
      transactions,
      balanceMeta,
      getFormattedBalance,
      transfer,
      convert,
      cardDebt,
      cardLimit,
      cardTransactions,
    }),
    [balances, transactions, balanceMeta, getFormattedBalance, transfer, convert, cardDebt, cardLimit, cardTransactions]
  );

  return (
    <WalletContext.Provider value={value}>
      {children}
    </WalletContext.Provider>
  );
}

export function useWallet(): WalletContextValue {
  const ctx = useContext(WalletContext);
  if (!ctx) throw new Error("useWallet must be used within WalletProvider");
  return ctx;
}
