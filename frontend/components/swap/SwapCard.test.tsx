import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { SwapCard } from "./SwapCard";
import {
  SESSION_RECOVERY_THRESHOLD_MS,
  STORAGE_KEY,
} from "@/hooks/useTradeFormStorage";

function createQuoteResponse(overrides?: Partial<Record<string, unknown>>) {
  return {
    base_asset: { asset_type: "native" },
    quote_asset: {
      asset_type: "credit_alphanum4",
      asset_code: "USDC",
      asset_issuer: "GQUOTE",
    },
    amount: "10",
    total: "9.5",
    price: "0.95",
    price_impact: "0.5",
    path: [],
    quote_type: "sell",
    timestamp: Date.now(),
    ...overrides,
  };
}

function createResponse(data: unknown) {
  return {
    ok: true,
    headers: new Headers(),
    json: async () => data,
  } as Response;
}

function deferredResponse(data: unknown) {
  let resolve!: (value: Response) => void;
  const promise = new Promise<Response>((res) => {
    resolve = res;
  });

  return {
    promise,
    resolve: () => resolve(createResponse(data)),
  };
}

function setVisibilityState(state: DocumentVisibilityState) {
  Object.defineProperty(document, "visibilityState", {
    configurable: true,
    get: () => state,
  });
}

describe("SwapCard session recovery", () => {
  beforeEach(() => {
    localStorage.clear();
    setVisibilityState("visible");
  });

  afterEach(() => {
    cleanup();
    localStorage.clear();
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it(
    "prompts recovery after refresh and re-fetches the quote before enabling swap",
    async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        amount: "10",
        slippage: 1,
        deadline: 45,
        fromToken: "native",
        toToken: "USDC:GQUOTE",
        savedAt: Date.now(),
      }),
    );

    const quoteDeferred = deferredResponse(createQuoteResponse());
    let quoteCalls = 0;
    const fetchMock = vi.fn(async (input: string | URL | Request) => {
      const url = String(input);
      if (url.includes("/api/v1/pairs")) {
        return createResponse({ pairs: [], total: 0 });
      }
      if (url.includes("/api/v1/quote/")) {
        quoteCalls += 1;
        return quoteDeferred.promise;
      }
      return createResponse({});
    });
    vi.stubGlobal("fetch", fetchMock);

    vi.useFakeTimers();
    render(<SwapCard />);

    await act(async () => {
      await Promise.resolve();
    });

    expect(screen.getByText(/restore previous trade\?/i)).toBeInTheDocument();
    expect(screen.getByTestId("session-recovery-summary")).toHaveTextContent(
      "10",
    );

    fireEvent.click(screen.getByRole("button", { name: /restore session/i }));

    expect(screen.getByLabelText(/you pay/i)).toHaveValue("10");

    fireEvent.click(screen.getByRole("button", { name: /connect wallet/i }));

    expect(
      screen.getByRole("button", { name: /refreshing quote/i }),
    ).toBeDisabled();

    await act(async () => {
      vi.advanceTimersByTime(400);
      await Promise.resolve();
    });

    expect(quoteCalls).toBe(1);

    await act(async () => {
      quoteDeferred.resolve();
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(screen.getByRole("button", { name: /^swap$/i })).toBeEnabled();
    },
    10_000,
  );

  it(
    "prompts recovery after long tab sleep and refreshes the quote before allowing swap",
    async () => {
    const wakeDeferred = deferredResponse(createQuoteResponse({ total: "9.8" }));
    let quoteCalls = 0;

    const fetchMock = vi.fn(async (input: string | URL | Request) => {
      const url = String(input);
      if (url.includes("/api/v1/pairs")) {
        return createResponse({ pairs: [], total: 0 });
      }
      if (url.includes("/api/v1/quote/")) {
        quoteCalls += 1;
        if (quoteCalls === 1) {
          return createResponse(createQuoteResponse());
        }
        return wakeDeferred.promise;
      }
      return createResponse({});
    });
    vi.stubGlobal("fetch", fetchMock);

    vi.useFakeTimers();
    render(<SwapCard />);

    fireEvent.click(screen.getByRole("button", { name: /connect wallet/i }));
    fireEvent.change(screen.getByLabelText(/you pay/i), {
      target: { value: "10" },
    });

    await act(async () => {
      vi.advanceTimersByTime(400);
      await Promise.resolve();
    });

    expect(screen.getByRole("button", { name: /^swap$/i })).toBeEnabled();

    setVisibilityState("hidden");
    act(() => {
      document.dispatchEvent(new Event("visibilitychange"));
    });

    await act(async () => {
      vi.advanceTimersByTime(SESSION_RECOVERY_THRESHOLD_MS + 1);
    });

    setVisibilityState("visible");
    act(() => {
      document.dispatchEvent(new Event("visibilitychange"));
    });

    expect(screen.getByText(/resume in-progress trade\?/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /refresh quote/i }));

    expect(
      screen.getByRole("button", { name: /refreshing quote/i }),
    ).toBeDisabled();
    expect(quoteCalls).toBe(2);

    await act(async () => {
      wakeDeferred.resolve();
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(screen.getByRole("button", { name: /^swap$/i })).toBeEnabled();
    },
    10_000,
  );
});
