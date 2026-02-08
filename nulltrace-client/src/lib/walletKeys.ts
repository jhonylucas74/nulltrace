/**
 * Mock receive keys for the in-game Wallet app. Not real keys.
 * USD is managed by Fkebank and uses a key style similar to instant payment (e.g. PIX).
 */

export interface WalletKeys {
  /** USD receive key (Fkebank). Long key for receiving USD; trackable, like instant payment. */
  usdReceiveKey: string;
  /** Public wallet address for crypto assets. */
  cryptoAddress: string;
}

export const MOCK_WALLET_KEYS: WalletKeys = {
  usdReceiveKey: "fkebank-a7b2c9d4e1f6g8h0i3j5k7l2m4n6o8p0q1r3s5t7u9v1w3x5",
  cryptoAddress: "0x7f3a9b2c1e4d5f6a8b0c2d3e4f5a6b7c8d9e0f1a",
};
