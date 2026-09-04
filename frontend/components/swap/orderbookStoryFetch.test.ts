import { afterEach, describe, expect, it, vi } from "vitest";

import {
  ORDERBOOK_STORY_ORIGINAL_FETCH,
  installOrderbookFetchMock,
  originalOrderbookStoryFetch,
  type OrderbookStoryFetch,
} from "./orderbookStoryFetch";
import type { Orderbook } from "@/types";

const book: Orderbook = {
  base_asset: { asset_type: "native" },
  quote_asset: {
    asset_type: "credit_alphanum4",
    asset_code: "USDC",
    asset_issuer: "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
  },
  bids: [],
  asks: [],
  timestamp: 1_700_000_000,
};

const passthrough = vi.fn(async () => new Response("passthrough", { status: 200 }));

afterEach(() => {
  globalThis.fetch = passthrough as unknown as typeof fetch;
  passthrough.mockClear();
});

describe("installOrderbookFetchMock", () => {
  it("tags the mock with the true original fetch", () => {
    globalThis.fetch = passthrough as unknown as typeof fetch;
    const restore = installOrderbookFetchMock(book);
    const tagged = globalThis.fetch as OrderbookStoryFetch;
    expect(tagged[ORDERBOOK_STORY_ORIGINAL_FETCH]).toBe(passthrough);
    expect(originalOrderbookStoryFetch(tagged)).toBe(passthrough);
    restore();
  });

  it("double install keeps the true original, not the first mock", () => {
    globalThis.fetch = passthrough as unknown as typeof fetch;
    const restoreA = installOrderbookFetchMock(book);
    const firstMock = globalThis.fetch;
    const restoreB = installOrderbookFetchMock({ status: 503 });
    const secondMock = globalThis.fetch as OrderbookStoryFetch;

    expect(secondMock).not.toBe(firstMock);
    expect(secondMock[ORDERBOOK_STORY_ORIGINAL_FETCH]).toBe(passthrough);
    expect(originalOrderbookStoryFetch(secondMock)).toBe(passthrough);

    restoreB();
    restoreA();
    expect(globalThis.fetch).toBe(passthrough);
  });

  it("StrictMode keep-first: restoring only the first install unhooks fetch", async () => {
    globalThis.fetch = passthrough as unknown as typeof fetch;
    const restoreA = installOrderbookFetchMock(book);
    installOrderbookFetchMock({ status: 503 });

    restoreA();
    expect(globalThis.fetch).toBe(passthrough);

    const other = await globalThis.fetch("/api/v1/pairs");
    expect(other.status).toBe(200);
    expect(passthrough).toHaveBeenCalled();
  });

  it("restores only when current fetch is still this helper's mock", () => {
    globalThis.fetch = passthrough as unknown as typeof fetch;
    const restore = installOrderbookFetchMock(book);
    const outsider = vi.fn(async () => new Response("outsider")) as unknown as typeof fetch;
    globalThis.fetch = outsider;

    restore();
    expect(globalThis.fetch).toBe(outsider);
  });

  it("intercepts orderbook URLs and passes other URLs through", async () => {
    globalThis.fetch = passthrough as unknown as typeof fetch;
    const restore = installOrderbookFetchMock(book);

    const orderbook = await globalThis.fetch("/api/v1/orderbook/XLM/USDC");
    expect(orderbook.status).toBe(200);
    await expect(orderbook.json()).resolves.toMatchObject({ timestamp: 1_700_000_000 });
    expect(passthrough).not.toHaveBeenCalled();

    const other = await globalThis.fetch("/api/v1/pairs");
    expect(passthrough).toHaveBeenCalled();
    expect(other.status).toBe(200);

    restore();
  });
});
