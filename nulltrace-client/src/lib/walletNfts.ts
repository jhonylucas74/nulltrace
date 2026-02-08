/**
 * Mock NFTs for the in-game Wallet app. Fictional collections and assets.
 */

export interface WalletNft {
  id: string;
  name: string;
  /** Collection or source; same value = same collection for grouping. */
  collection: string;
  /** Placeholder or data URL for the image. */
  imageUrl: string;
  tokenId: string;
  chain?: string;
  contract?: string;
}

/** Simple 1x1 PNG data URLs as placeholders (hex color). */
const PLACEHOLDERS: Record<string, string> = {
  green: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
  yellow: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
  purple: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
  blue: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
  red: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
};

/** Mock NFTs grouped by collection. */
export const MOCK_WALLET_NFTS: WalletNft[] = [
  {
    id: "nft1",
    name: "Nulltrace Genesis",
    collection: "Nulltrace Official",
    imageUrl: PLACEHOLDERS.green,
    tokenId: "1",
    chain: "mainnet",
    contract: "0xnull…trace",
  },
  {
    id: "nft2",
    name: "Desert Runner",
    collection: "Nulltrace Official",
    imageUrl: PLACEHOLDERS.yellow,
    tokenId: "2",
    chain: "mainnet",
    contract: "0xnull…trace",
  },
  {
    id: "nft3",
    name: "Pixel Guardian",
    collection: "Pixel Collection",
    imageUrl: PLACEHOLDERS.purple,
    tokenId: "101",
    chain: "mainnet",
    contract: "0xpixel…coll",
  },
  {
    id: "nft4",
    name: "Ocean Fragment",
    collection: "Pixel Collection",
    imageUrl: PLACEHOLDERS.blue,
    tokenId: "102",
    chain: "mainnet",
    contract: "0xpixel…coll",
  },
  {
    id: "nft5",
    name: "Beta Tester Badge",
    collection: "Rewards",
    imageUrl: PLACEHOLDERS.red,
    tokenId: "1",
    chain: "mainnet",
    contract: "0xreward…badge",
  },
];

/** Group NFTs by collection. */
export function groupNftsByCollection(
  nfts: WalletNft[]
): Map<string, WalletNft[]> {
  const map = new Map<string, WalletNft[]>();
  for (const nft of nfts) {
    const list = map.get(nft.collection) ?? [];
    list.push(nft);
    map.set(nft.collection, list);
  }
  return map;
}
