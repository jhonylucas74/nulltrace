/**
 * Mock wallet transactions (bank statement) for the in-game Wallet app.
 * Types: credit, debit, transfer, convert.
 */

export type TransactionType = "credit" | "debit" | "transfer" | "convert";

export interface WalletTransaction {
  id: string;
  type: TransactionType;
  amount: string;
  currency: string;
  date: string;
  /** Unix ms for filtering by period (today, 7d, 30d). */
  timestamp?: number;
  /** Short description or counterparty (e.g. "Transfer to 0x…", "Convert USD → BTC"). */
  description: string;
  /** Optional balance after this tx (for display). */
  balanceAfter?: string;
}

/** Build timestamp for mock data (year, month 1-based, day, hour, min). */
function ts(y: number, m: number, d: number, h: number, min: number): number {
  return new Date(y, m - 1, d, h, min).getTime();
}

const now = new Date();
const y = now.getFullYear();

/** Initial mock transactions (newest first). */
export const MOCK_WALLET_TRANSACTIONS: WalletTransaction[] = [
  {
    id: "tx1",
    type: "credit",
    amount: "120.00",
    currency: "USD",
    date: "Feb 7, 10:15",
    timestamp: ts(y, 2, 7, 10, 15),
    description: "Deposit from Example Corp",
  },
  {
    id: "tx2",
    type: "debit",
    amount: "45.50",
    currency: "USD",
    date: "Feb 6, 16:42",
    timestamp: ts(y, 2, 6, 16, 42),
    description: "Payment to Store",
  },
  {
    id: "tx3",
    type: "transfer",
    amount: "0.005",
    currency: "BTC",
    date: "Feb 5, 09:00",
    timestamp: ts(y, 2, 5, 9, 0),
    description: "Transfer to 0x7f3…a2c",
  },
  {
    id: "tx4",
    type: "convert",
    amount: "50.00",
    currency: "USD",
    date: "Feb 4, 14:20",
    timestamp: ts(y, 2, 4, 14, 20),
    description: "Convert USD → ETH",
  },
];

/** Generate a unique tx id (simple counter for mock). */
let nextTxId = 1000;
export function nextTransactionId(): string {
  return `tx-${nextTxId++}`;
}
