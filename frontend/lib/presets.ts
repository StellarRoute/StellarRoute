export interface SwapPreset {
  id: string;
  label: string;
  baseAsset: string;
  quoteAsset: string;
}

export const DEFAULT_SWAP_PRESETS: SwapPreset[] = [
  {
    id: "xlm-usdc",
    label: "XLM / USDC",
    baseAsset: "native",
    quoteAsset: "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
  },
  {
    id: "xlm-aqua",
    label: "XLM / AQUA",
    baseAsset: "native",
    quoteAsset: "AQUA:GBNZILSTVQZ4R7IKQDGHYGY2QXL5QOFJYQMXPKWRRM5PAV7Y4M67AQUA",
  },
  {
    id: "usdc-xlm",
    label: "USDC / XLM",
    baseAsset: "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
    quoteAsset: "native",
  },
  {
    id: "aqua-xlm",
    label: "AQUA / XLM",
    baseAsset: "AQUA:GBNZILSTVQZ4R7IKQDGHYGY2QXL5QOFJYQMXPKWRRM5PAV7Y4M67AQUA",
    quoteAsset: "native",
  },
];
