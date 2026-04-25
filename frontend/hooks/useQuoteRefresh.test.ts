import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { PriceQuote } from "@/types";
import { StellarRouteApiError, stellarRouteClient } from "@/lib/api/client";
import { useQuoteRefresh } from "./useQuoteRefresh";

vi.mock("@/lib/api/client", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/client")>(
    "@/lib/api/client",
  );
  return {
    ...actual,
    stellarRouteClient: {
      ...actual.stellarRouteClient,
      getQuote: vi.fn(),
    },
  };
});

function buildQuote(total: string): PriceQuote {
  return {
    base_asset: { asset_type: "native" },
    quote_asset: { asset_type: "credit_alphanum4", asset_code: "USDC", asset_issuer: "G..." },
    amount: "100",
    price: "0.98",
    total,
    quote_type: "sell",
    path: [],
    timestamp: Math.floor(Date.now() / 1000),
  };
}

describe("useQuoteRefresh retries", () => {
  afterEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it("auto-retries transient online quote failures and recovers", async () => {
    const getQuoteMock = vi.mocked(stellarRouteClient.getQuote);
    let callCount = 0;
    getQuoteMock.mockImplementation(async () => {
      callCount += 1;
      if (callCount === 1) {
        throw new Error("Failed to fetch");
      }
      return buildQuote("98.0");
    });

    const { result } = renderHook(() =>
      useQuoteRefresh("native", "USDC:G...", 100, "sell", {
        debounceMs: 1,
        maxAutoRetries: 2,
        retryBackoffMs: 5,
        isOnline: true,
      }),
    );
    await waitFor(
      () => {
        expect(result.current.data?.total).toBe("98.0");
        expect(result.current.isRecovering).toBe(false);
        expect(result.current.retryAttempt).toBe(0);
      },
      { timeout: 2000 },
    );
  });

  it("does not auto-retry non-transient client errors", async () => {
    const getQuoteMock = vi.mocked(stellarRouteClient.getQuote);
    getQuoteMock.mockRejectedValueOnce(
      new StellarRouteApiError(400, "bad_request", "Invalid amount"),
    );

    const { result } = renderHook(() =>
      useQuoteRefresh("native", "USDC:G...", 100, "sell", {
        debounceMs: 0,
        maxAutoRetries: 2,
        retryBackoffMs: 10,
        isOnline: true,
      }),
    );

    await waitFor(() => {
      expect(result.current.error).toBeInstanceOf(StellarRouteApiError);
      expect(result.current.isRecovering).toBe(false);
      expect(result.current.retryAttempt).toBe(0);
    });

    expect(getQuoteMock).toHaveBeenCalledTimes(1);
  });

  it("respects Retry-After before allowing another manual refresh on 429s", async () => {
    vi.useFakeTimers();

    const getQuoteMock = vi.mocked(stellarRouteClient.getQuote);
    getQuoteMock
      .mockRejectedValueOnce(
        new StellarRouteApiError(
          429,
          "rate_limit_exceeded",
          "Too many requests",
          undefined,
          5_000,
        ),
      )
      .mockResolvedValueOnce(buildQuote("98.0"));

    const { result } = renderHook(() =>
      useQuoteRefresh("native", "USDC:G...", 100, "sell", {
        debounceMs: 0,
        maxAutoRetries: 0,
        retryBackoffMs: 10,
        isOnline: true,
      }),
    );

    await act(async () => {
      vi.advanceTimersByTime(1);
      await Promise.resolve();
    });

    expect(result.current.error).toBeInstanceOf(StellarRouteApiError);
    expect(result.current.rateLimitRemainingMs).toBeGreaterThan(0);

    act(() => {
      result.current.refresh();
    });
    expect(getQuoteMock).toHaveBeenCalledTimes(1);

    act(() => {
      vi.advanceTimersByTime(5_000);
    });

    act(() => {
      result.current.refresh();
    });

    await act(async () => {
      await Promise.resolve();
    });

    expect(getQuoteMock).toHaveBeenCalledTimes(2);
    expect(result.current.data?.total).toBe("98.0");
  });
});
