/**
 * Mock variable exchange rates for the in-game Wallet conversion simulator.
 * Rates change over time (fake variation). No real API.
 */

const CURRENCIES = ["USD", "BTC", "ETH", "SOL"] as const;

/** Base rates: 1 USD = x of each (fake). */
const BASE_RATES: Record<string, number> = {
  USD: 1,
  BTC: 0.00004,
  ETH: 0.0005,
  SOL: 0.01,
};

/** Variation amplitude (relative). */
const AMPLITUDE = 0.001;
/** Period in ms for sine wave. */
const PERIOD_MS = 60000;

/**
 * Get current mock rate: fromSymbol per 1 toSymbol.
 * E.g. getRate("USD", "BTC") = how many BTC per 1 USD (small number).
 */
export function getRate(fromSymbol: string, toSymbol: string): number {
  if (fromSymbol === toSymbol) return 1;
  const baseFrom = BASE_RATES[fromSymbol] ?? 1;
  const baseTo = BASE_RATES[toSymbol] ?? 1;
  const baseRate = baseTo / baseFrom;
  const variation = 1 + AMPLITUDE * Math.sin(Date.now() / PERIOD_MS);
  return baseRate * variation;
}

/**
 * Convert amount from one currency to another using current rate.
 */
export function convertAmount(
  amount: number,
  fromSymbol: string,
  toSymbol: string
): number {
  const rate = getRate(fromSymbol, toSymbol);
  return amount * rate;
}

export { CURRENCIES };
