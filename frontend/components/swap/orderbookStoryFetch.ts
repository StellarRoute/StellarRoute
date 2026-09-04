import type { Orderbook } from "@/types";

/**
 * Tag on a story fetch mock that points at the true original `fetch`.
 * Nested installs (React StrictMode double-invoking a `useState` initializer)
 * walk this instead of treating the previous mock as original.
 */
export const ORDERBOOK_STORY_ORIGINAL_FETCH = Symbol.for(
  "stellarroute.orderbookStoryOriginalFetch",
);

export type OrderbookStoryFetch = typeof fetch & {
  [ORDERBOOK_STORY_ORIGINAL_FETCH]?: typeof fetch;
};

export function originalOrderbookStoryFetch(fn: typeof fetch): typeof fetch {
  return (fn as OrderbookStoryFetch)[ORDERBOOK_STORY_ORIGINAL_FETCH] ?? fn;
}

function requestUrl(input: RequestInfo | URL): string {
  if (typeof input === "string") return input;
  if (input instanceof URL) return input.href;
  return input.url;
}

/**
 * Stub `GET /api/v1/orderbook/{base}/{quote}` so a story can render a fixed
 * book. Everything else falls through to the true original `fetch`.
 *
 * Restores only when the installed mock is still current, or when a later
 * install from this helper replaced it (StrictMode keeps the first
 * initializer's restore and discards the second).
 */
export function installOrderbookFetchMock(
  response: Orderbook | { status: number },
): () => void {
  const originalFetch = originalOrderbookStoryFetch(globalThis.fetch);

  const mockFetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
    if (!requestUrl(input).includes("/api/v1/orderbook/")) {
      return originalFetch(input, init);
    }
    if ("status" in response) {
      return new Response(
        JSON.stringify({ error: "upstream_error", message: "Horizon unavailable" }),
        { status: response.status, headers: { "Content-Type": "application/json" } },
      );
    }
    return new Response(JSON.stringify(response), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });
  }) as OrderbookStoryFetch;

  mockFetch[ORDERBOOK_STORY_ORIGINAL_FETCH] = originalFetch;
  globalThis.fetch = mockFetch;

  return () => {
    const current = globalThis.fetch as OrderbookStoryFetch;
    if (current === mockFetch) {
      globalThis.fetch = originalFetch;
      return;
    }
    // Later install replaced us; its restore was discarded. Unlink the stack.
    if (current[ORDERBOOK_STORY_ORIGINAL_FETCH] === originalFetch) {
      globalThis.fetch = originalFetch;
    }
  };
}
