/**
 * Virtual credit cards (Fkebank). Linked to USD balance.
 * Fkebank charges the invoice every 7 days.
 */

export interface VirtualCard {
  id: string;
  /** Last 4 digits for display. */
  last4: string;
  /** Full number only for internal/display when revealed (mock). */
  number: string;
  expiryMonth: number;
  expiryYear: number;
  holderName: string;
  /** Security code; should be hidden by default in UI. */
  cvv: string;
  /** Display label (e.g. "Shopping", "Subscriptions"). */
  label?: string;
}

/** Next invoice charge is every 7 days (Fkebank). */
export const FKEBANK_BILLING_DAYS = 7;

/** Mock: next charge date (e.g. 5 days from now). */
export function getNextChargeDate(): Date {
  const d = new Date();
  d.setDate(d.getDate() + 5);
  d.setHours(23, 59, 0, 0);
  return d;
}

/** Initial virtual cards (Fkebank, USD). */
export const MOCK_VIRTUAL_CARDS: VirtualCard[] = [
  {
    id: "card1",
    last4: "4242",
    number: "4111111111114242",
    expiryMonth: 12,
    expiryYear: 2028,
    holderName: "Nulltrace User",
    cvv: "123",
    label: "Main",
  },
  {
    id: "card2",
    last4: "8888",
    number: "4111111111118888",
    expiryMonth: 6,
    expiryYear: 2027,
    holderName: "Nulltrace User",
    cvv: "456",
    label: "Virtual",
  },
];

let nextCardId = 100;
export function nextVirtualCardId(): string {
  return `card-${nextCardId++}`;
}

/** Format card number with spaces for display (e.g. 4111 1111 1111 4242). */
export function formatCardNumber(num: string): string {
  const digits = num.replace(/\D/g, "");
  const groups = digits.match(/.{1,4}/g) ?? [];
  return groups.join(" ");
}

/** Card-only statement (Fkebank). Purchase = debt increases, payment = debt decreases. */
export type CardTransactionType = "purchase" | "payment";

export interface CardTransaction {
  id: string;
  type: CardTransactionType;
  amount: string;
  date: string;
  timestamp?: number;
  description: string;
}

function cardTs(y: number, m: number, d: number, h: number, min: number): number {
  return new Date(y, m - 1, d, h, min).getTime();
}

const now = new Date();
const y = now.getFullYear();

/** Mock card transactions (newest first). */
export const MOCK_CARD_TRANSACTIONS: CardTransaction[] = [
  {
    id: "ct1",
    type: "payment",
    amount: "200.00",
    date: "Feb 6, 09:00",
    timestamp: cardTs(y, 2, 6, 9, 0),
    description: "Invoice payment",
  },
  {
    id: "ct2",
    type: "purchase",
    amount: "89.50",
    date: "Feb 5, 14:22",
    timestamp: cardTs(y, 2, 5, 14, 22),
    description: "Online store",
  },
  {
    id: "ct3",
    type: "purchase",
    amount: "150.00",
    date: "Feb 4, 11:10",
    timestamp: cardTs(y, 2, 4, 11, 10),
    description: "Subscription service",
  },
  {
    id: "ct4",
    type: "purchase",
    amount: "80.50",
    date: "Feb 3, 18:45",
    timestamp: cardTs(y, 2, 3, 18, 45),
    description: "Market",
  },
];

/** Mock current card debt (USD). */
export const MOCK_CARD_DEBT = 320.0;

/** Mock credit limit (USD). */
export const MOCK_CARD_LIMIT = 1000.0;

let nextCardTxId = 500;
export function nextCardTransactionId(): string {
  return `ct-${nextCardTxId++}`;
}
