import { STAGING_DEMO_ISSUER } from "@/lib/trading-pairs";

export interface SwapPreset {
  id: string;
  label: string;
  baseAsset: string;
  quoteAsset: string;
}

const I = STAGING_DEMO_ISSUER;

/** Quick-pair chips aligned with testnet staging orderbooks (GDMVY5 issuer). */
export const DEFAULT_SWAP_PRESETS: SwapPreset[] = [
  {
    id: "eur-usdy",
    label: "EUR / USDy",
    baseAsset: `EUR:${I}`,
    quoteAsset: `USDy:${I}`,
  },
  {
    id: "btc-ext",
    label: "BTC / EXT",
    baseAsset: `BTC:${I}`,
    quoteAsset: `EXT:${I}`,
  },
  {
    id: "btc-usdy",
    label: "BTC / USDy",
    baseAsset: `BTC:${I}`,
    quoteAsset: `USDy:${I}`,
  },
  {
    id: "ext-usdy",
    label: "EXT / USDy",
    baseAsset: `EXT:${I}`,
    quoteAsset: `USDy:${I}`,
  },
  {
    id: "eury-usd",
    label: "EURy / USD",
    baseAsset: `EURy:${I}`,
    quoteAsset: `USD:${I}`,
  },
  {
    id: "ars-eury",
    label: "ARS / EURy",
    baseAsset: `ARS:${I}`,
    quoteAsset: `EURy:${I}`,
  },
];
