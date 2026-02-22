/**
 * Wallet state for balances, transactions, keys and cards.
 * All amounts stored as DB-cents internally; use /100 for display.
 */

import React, { createContext, useContext, useCallback, useMemo, useState, useEffect, useRef } from "react";
import { useAuth } from "./AuthContext";
import { useGrpc, type GrpcWalletCard, type GrpcCardStatement } from "./GrpcContext";

// ── Re-exports for consumers ─────────────────────────────────────────────────
export type { GrpcWalletCard as WalletCard, GrpcCardStatement as CardStatement };

export interface WalletKey {
  currency: string;
  key_address: string;
}

export interface WalletTx {
  id: string;
  tx_type: string; // credit | debit | transfer_in | transfer_out | convert
  currency: string;
  amount: number;   // DB-cents
  fee: number;
  description: string;
  counterpart_address: string;
  created_at_ms: number;
}

export interface CardTx {
  id: string;
  tx_type: string; // purchase | payment | refund
  amount: number;  // DB-cents
  description: string;
  created_at_ms: number;
}

// ── Amount helpers ────────────────────────────────────────────────────────────

/**
 * Parse amount string to number. Accepts comma or dot as decimal separator.
 * Last occurrence of comma or dot is the decimal separator; others are thousand separators.
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
 */
export function applyAmountMask(value: string, maxDecimals: number = 8): string {
  let hasDecimal = false;
  let decimalCount = 0;
  const out: string[] = [];
  for (const c of value) {
    if (c >= "0" && c <= "9") {
      if (hasDecimal) {
        if (decimalCount < maxDecimals) { out.push(c); decimalCount++; }
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

/** Format a DB-cents value for display. */
export function formatAmount(cents: number, symbol: string): string {
  const value = cents / 100;
  if (symbol === "USD") return value.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  if (symbol === "BTC") return value.toLocaleString("en-US", { minimumFractionDigits: 4, maximumFractionDigits: 6 });
  if (symbol === "ETH" || symbol === "SOL") return value.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 4 });
  return value.toFixed(2);
}

// ── Context interface ─────────────────────────────────────────────────────────

interface WalletContextValue {
  /** Balances in DB-cents, keyed by currency symbol. */
  balances: Record<string, number>;
  /** Keys/addresses for each currency. */
  keys: WalletKey[];
  /** Credit cards. */
  cards: GrpcWalletCard[];
  /** All wallet transactions (newest first). */
  transactions: WalletTx[];
  /** Card transactions for the currently selected card. */
  cardTransactions: CardTx[];
  /** Current open card statement (for the first/active card). */
  cardStatement: GrpcCardStatement | null;
  /** Total card debt across all cards (DB-cents). */
  cardDebt: number;
  /** Total card credit limit across all cards (DB-cents). */
  cardLimit: number;
  isLoading: boolean;
  /** Get formatted display balance for a symbol. */
  getFormattedBalance: (symbol: string) => string;
  /** Transfer funds. Returns error string or null on success. */
  transfer: (currency: string, amountCents: number, recipientKey: string) => Promise<string | null>;
  /** Convert funds. Returns error string or null on success. */
  convert: (fromSymbol: string, toSymbol: string, amountCents: number) => Promise<string | null>;
  /** Pay credit card bill. Returns error string or null on success. */
  payBill: (cardId: string) => Promise<string | null>;
  /**
   * Debit a display-unit amount from the wallet balance synchronously (optimistic).
   * Fires the backend debit in the background. Returns false if local balance is insufficient.
   * Used by NullCloudContext for in-game purchases (amount is display value, e.g. 5.99 USD).
   */
  pay: (currency: string, displayAmount: number, description: string) => boolean;
  /** Refresh balances. */
  refreshBalances: () => Promise<void>;
  /** Fetch transactions for a given filter. */
  fetchTransactions: (filter: string) => Promise<void>;
  /** Fetch card transactions for a given card and filter. */
  fetchCardTransactions: (cardId: string, filter: string) => Promise<void>;
  /** Fetch (or refresh) the open card statement. */
  fetchCardStatement: (cardId: string) => Promise<void>;
  /** Create a new virtual card. Returns created card or null. */
  createCard: (label: string, creditLimitCents: number) => Promise<GrpcWalletCard | null>;
  /** Delete a card. Returns true if successful. */
  deleteCard: (cardId: string) => Promise<boolean>;
}

const WalletContext = createContext<WalletContextValue | null>(null);

export function WalletProvider({ children }: { children: React.ReactNode }) {
  const { token } = useAuth();
  const grpc = useGrpc();

  const [balances, setBalances] = useState<Record<string, number>>({});
  const [keys, setKeys] = useState<WalletKey[]>([]);
  const [cards, setCards] = useState<GrpcWalletCard[]>([]);
  const [transactions, setTransactions] = useState<WalletTx[]>([]);
  const [cardTransactions, setCardTransactions] = useState<CardTx[]>([]);
  const [cardStatement, setCardStatement] = useState<GrpcCardStatement | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const tokenRef = useRef(token);
  tokenRef.current = token;
  const balancesRef = useRef(balances);
  balancesRef.current = balances;

  // ── Fetch helpers ───────────────────────────────────────────────────────────

  const refreshBalances = useCallback(async () => {
    const tok = tokenRef.current;
    if (!tok) return;
    try {
      const res = await grpc.getWalletBalances(tok);
      if (!res.error_message) {
        const map: Record<string, number> = {};
        for (const b of res.balances) map[b.currency] = b.amount;
        setBalances(map);
      }
    } catch { /* network error – keep stale data */ }
  }, [grpc]);

  const fetchKeys = useCallback(async () => {
    const tok = tokenRef.current;
    if (!tok) return;
    try {
      const res = await grpc.getWalletKeys(tok);
      if (!res.error_message) setKeys(res.keys);
    } catch { /* ignore */ }
  }, [grpc]);

  const fetchCards = useCallback(async () => {
    const tok = tokenRef.current;
    if (!tok) return;
    try {
      const res = await grpc.getWalletCards(tok);
      if (!res.error_message) setCards(res.cards);
    } catch { /* ignore */ }
  }, [grpc]);

  const fetchTransactions = useCallback(async (filter: string) => {
    const tok = tokenRef.current;
    if (!tok) return;
    try {
      const res = await grpc.getWalletTransactions(tok, filter);
      if (!res.error_message) setTransactions(res.transactions);
    } catch { /* ignore */ }
  }, [grpc]);

  const fetchCardTransactions = useCallback(async (cardId: string, filter: string) => {
    const tok = tokenRef.current;
    if (!tok) return;
    try {
      const res = await grpc.getCardTransactions(tok, cardId, filter);
      if (!res.error_message) setCardTransactions(res.transactions);
    } catch { /* ignore */ }
  }, [grpc]);

  const fetchCardStatement = useCallback(async (cardId: string) => {
    const tok = tokenRef.current;
    if (!tok) return;
    try {
      const res = await grpc.getCardStatement(tok, cardId);
      if (!res.error_message && res.statement) setCardStatement(res.statement);
    } catch { /* ignore */ }
  }, [grpc]);

  // ── Initial load ────────────────────────────────────────────────────────────

  useEffect(() => {
    if (!token) return;
    setIsLoading(true);
    Promise.all([
      refreshBalances(),
      fetchKeys(),
      fetchCards(),
      fetchTransactions("all"),
    ]).finally(() => setIsLoading(false));
  }, [token]); // eslint-disable-line react-hooks/exhaustive-deps

  // ── Derived values ──────────────────────────────────────────────────────────

  const cardDebt = useMemo(() => cards.reduce((sum, c) => sum + c.current_debt, 0), [cards]);
  const cardLimit = useMemo(() => cards.reduce((sum, c) => sum + c.credit_limit, 0), [cards]);

  const getFormattedBalance = useCallback(
    (symbol: string) => formatAmount(balances[symbol] ?? 0, symbol),
    [balances]
  );

  // ── Mutations ───────────────────────────────────────────────────────────────

  const transfer = useCallback(
    async (currency: string, amountCents: number, recipientKey: string): Promise<string | null> => {
      const tok = tokenRef.current;
      if (!tok) return "Not authenticated";
      try {
        const res = await grpc.transferFunds(tok, recipientKey, currency, amountCents);
        if (!res.success) return res.error_message || "Transfer failed";
        await refreshBalances();
        await fetchTransactions("all");
        return null;
      } catch (e) {
        return String(e);
      }
    },
    [grpc, refreshBalances, fetchTransactions]
  );

  const convert = useCallback(
    async (fromSymbol: string, toSymbol: string, amountCents: number): Promise<string | null> => {
      const tok = tokenRef.current;
      if (!tok) return "Not authenticated";
      try {
        const res = await grpc.convertFunds(tok, fromSymbol, toSymbol, amountCents);
        if (!res.success) return res.error_message || "Conversion failed";
        await refreshBalances();
        await fetchTransactions("all");
        return null;
      } catch (e) {
        return String(e);
      }
    },
    [grpc, refreshBalances, fetchTransactions]
  );

  const payBill = useCallback(
    async (cardId: string): Promise<string | null> => {
      const tok = tokenRef.current;
      if (!tok) return "Not authenticated";
      try {
        const res = await grpc.payCardBill(tok, cardId);
        if (!res.success) return res.error_message || "Payment failed";
        await Promise.all([refreshBalances(), fetchCards(), fetchCardStatement(cardId)]);
        return null;
      } catch (e) {
        return String(e);
      }
    },
    [grpc, refreshBalances, fetchCards, fetchCardStatement]
  );

  const createCard = useCallback(
    async (label: string, creditLimitCents: number): Promise<GrpcWalletCard | null> => {
      const tok = tokenRef.current;
      if (!tok) return null;
      try {
        const res = await grpc.createWalletCard(tok, label, creditLimitCents);
        if (res.card) {
          setCards((prev) => [...prev, res.card!]);
          return res.card;
        }
        return null;
      } catch { return null; }
    },
    [grpc]
  );

  const deleteCard = useCallback(
    async (cardId: string): Promise<boolean> => {
      const tok = tokenRef.current;
      if (!tok) return false;
      try {
        const res = await grpc.deleteWalletCard(tok, cardId);
        if (res.success) {
          setCards((prev) => prev.filter((c) => c.id !== cardId));
        }
        return res.success;
      } catch { return false; }
    },
    [grpc]
  );

  /**
   * Synchronous optimistic debit for in-game purchases (NullCloud etc.).
   * Checks local balance, deducts optimistically, fires backend debit in background.
   */
  const pay = useCallback(
    (currency: string, displayAmount: number, _description: string): boolean => {
      const amountCents = Math.round(displayAmount * 100);
      const current = balancesRef.current[currency] ?? 0;
      if (amountCents <= 0 || current < amountCents) return false;
      // Optimistic local deduction
      setBalances((prev) => ({ ...prev, [currency]: (prev[currency] ?? 0) - amountCents }));
      // Fire-and-forget backend debit (best-effort; reconcile on next balance refresh)
      const tok = tokenRef.current;
      if (tok) {
        grpc.transferFunds(tok, "", currency, amountCents).then(() => {
          // Refresh to reconcile — ignore errors
          refreshBalances().catch(() => {});
        }).catch(() => {});
      }
      return true;
    },
    [grpc, refreshBalances]
  );

  // ── Context value ───────────────────────────────────────────────────────────

  const value: WalletContextValue = useMemo(
    () => ({
      balances,
      keys,
      cards,
      transactions,
      cardTransactions,
      cardStatement,
      cardDebt,
      cardLimit,
      isLoading,
      getFormattedBalance,
      transfer,
      convert,
      payBill,
      pay,
      refreshBalances,
      fetchTransactions,
      fetchCardTransactions,
      fetchCardStatement,
      createCard,
      deleteCard,
    }),
    [
      balances, keys, cards, transactions, cardTransactions, cardStatement,
      cardDebt, cardLimit, isLoading,
      getFormattedBalance, transfer, convert, payBill, pay,
      refreshBalances, fetchTransactions, fetchCardTransactions, fetchCardStatement,
      createCard, deleteCard,
    ]
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
