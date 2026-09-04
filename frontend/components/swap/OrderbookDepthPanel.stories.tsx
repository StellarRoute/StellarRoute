import type { Story } from "@ladle/react";
import { useEffect, useState } from "react";

import { OrderbookDepthPanel } from "./OrderbookDepthPanel";
import { installOrderbookFetchMock } from "./orderbookStoryFetch";
import type { Asset, Orderbook, OrderbookEntry } from "@/types";

const meta = { title: "Swap/OrderbookDepthPanel" };
export default meta;

const XLM: Asset = { asset_type: "native" };
const USDC: Asset = {
  asset_type: "credit_alphanum4",
  asset_code: "USDC",
  asset_issuer: "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
};

function book(bids: OrderbookEntry[], asks: OrderbookEntry[]): Orderbook {
  return {
    base_asset: XLM,
    quote_asset: USDC,
    bids,
    asks,
    timestamp: 1_700_000_000,
  };
}

/** Both sides empty — the pair has no resting offers at all. */
const EMPTY_BOOK = book([], []);

/** One level per side: the panel still has to render a spread and two rows. */
const THIN_BOOK = book(
  [{ price: "0.1050000", amount: "120.0000000", total: "12.6000000" }],
  [{ price: "0.1080000", amount: "95.0000000", total: "10.2600000" }],
);

/** Bids only — asks side falls back to its "No asks" empty row. */
const ONE_SIDED_BOOK = book(
  [
    { price: "0.1050000", amount: "120.0000000", total: "12.6000000" },
    { price: "0.1040000", amount: "80.0000000", total: "8.3200000" },
  ],
  [],
);

const DEEP_BOOK = book(
  Array.from({ length: 10 }, (_, i) => ({
    price: (0.105 - i * 0.0005).toFixed(7),
    amount: ((i + 1) * 100).toFixed(7),
    total: ((i + 1) * 10.5).toFixed(7),
  })),
  Array.from({ length: 10 }, (_, i) => ({
    price: (0.108 + i * 0.0005).toFixed(7),
    amount: ((i + 1) * 90).toFixed(7),
    total: ((i + 1) * 9.7).toFixed(7),
  })),
);

/**
 * Installs the stub during render — the panel's own fetch effect runs before
 * the harness's effects, so an effect-time install would be too late — and
 * restores the original `fetch` on unmount. The helper is stack-safe under
 * StrictMode double-invoking the `useState` initializer.
 */
function StoryHarness({
  response,
  maxRows,
}: {
  response: Orderbook | { status: number };
  maxRows?: number;
}) {
  const [restore] = useState(() => installOrderbookFetchMock(response));
  useEffect(() => restore, [restore]);

  return (
    <div className="max-w-xl p-4">
      <OrderbookDepthPanel base="XLM" quote="USDC" maxRows={maxRows} />
    </div>
  );
}

export const Default: Story = () => <StoryHarness response={DEEP_BOOK} />;

/** No bids and no asks: both sides show their empty row, spread is absent. */
export const Empty: Story = () => <StoryHarness response={EMPTY_BOOK} />;

/** A single level each side — the thinnest book that still has a spread. */
export const ThinBook: Story = () => <StoryHarness response={THIN_BOOK} />;

/** Half-empty book: bids present, asks missing. */
export const OneSidedBook: Story = () => <StoryHarness response={ONE_SIDED_BOOK} />;

/** A thin book truncated further by `maxRows`. */
export const ThinBookSingleRow: Story = () => (
  <StoryHarness response={THIN_BOOK} maxRows={1} />
);

/** Upstream failure — the panel swaps to its error `ViewState`. */
export const ErrorState: Story = () => <StoryHarness response={{ status: 503 }} />;
