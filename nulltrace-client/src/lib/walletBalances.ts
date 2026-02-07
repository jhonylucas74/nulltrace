/**
 * Mock wallet balances for the in-game Wallet app. Fake player balances only.
 */

export interface WalletBalance {
  currency: string;
  symbol: string;
  amount: string;
}

/** Max 4 currencies. In-game fake balances for the player. */
export const MOCK_WALLET_BALANCES: WalletBalance[] = [
  { currency: "US Dollar", symbol: "USD", amount: "1,240.00" },
  { currency: "Bitcoin", symbol: "BTC", amount: "0.0245" },
  { currency: "Ethereum", symbol: "ETH", amount: "0.89" },
  { currency: "Solana", symbol: "SOL", amount: "12.50" },
];
