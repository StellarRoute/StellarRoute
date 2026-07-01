import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { PriceQuote } from "@/types";
import { StellarRouteApiError, type QuoteFetchResult } from "@/lib/api/client";
import { QUOTE_RETRY_EVENT_NAME } from "@/lib/quote-retry";
import { useQuoteRefresh } from "./useQuoteRefresh";

const { getQuote } = vi.hoisted(() => ({
  getQuote: vi.fn(),
}));

vi.mock("@/components/providers/wallet-provider", () => ({
  useWallet: () => ({ network: "testnet" }),
}));

vi.mock("@/lib/api/client", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/client")>(
    "@/lib/api/client",
  );
  return {
    ...actual,
    createStellarRouteClient: vi.fn(() => ({ getQuote })),
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
    timestamp: Date.now(),
  };
}

function buildQuoteResult(total: string, requestId = "test-req-id"): QuoteFetchResult {
  return {
    quote: buildQuote(total),
    requestId,
  };
}

describe("useQuoteRefresh retries", () => {
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it("auto-retries transient online quote failures and recovers", async () => {
    const getQuoteMock = vi.mocked(getQuote);
    let callCount = 0;
    getQuoteMock.mockImplementation(async () => {
      callCount += 1;
      if (callCount === 1) {
        throw new Error("Failed to fetch");
      }
      return buildQuoteResult("98.0");
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
    const getQuoteMock = vi.mocked(getQuote);
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

  it("blocks manual refresh while Retry-After rate limit is active", async () => {
    const getQuoteMock = vi.mocked(getQuote);
    getQuoteMock.mockRejectedValueOnce(
      new StellarRouteApiError(
        429,
        "rate_limit_exceeded",
        "Too many requests",
        undefined,
        5_000,
      ),
    );

    const { result, unmount } = renderHook(() =>
      useQuoteRefresh("native", "USDC:G...", 100, "sell", {
        debounceMs: 0,
        maxAutoRetries: 0,
        isOnline: true,
      }),
    );

    await waitFor(() => {
      expect(result.current.error).toBeInstanceOf(StellarRouteApiError);
      expect(result.current.rateLimitRemainingMs).toBeGreaterThan(0);
    });

    const callsBeforeRefresh = getQuoteMock.mock.calls.length;
    act(() => {
      result.current.refresh();
    });
    expect(getQuoteMock).toHaveBeenCalledTimes(callsBeforeRefresh);
    unmount();
  });

  it("schedules exponential backoff telemetry for transient quote failures", async () => {
    const getQuoteMock = vi.mocked(getQuote);
    getQuoteMock.mockRejectedValue(new Error("Failed to fetch"));

    const telemetry = vi.fn();

    const { unmount } = renderHook(() =>
      useQuoteRefresh("native", "USDC:G...", 100, "sell", {
        debounceMs: 0,
        maxAutoRetries: 1,
        retryBackoffMs: 250,
        maxRetryBackoffMs: 250,
        retryJitterRatio: 0,
        isOnline: true,
        onRetryEvent: telemetry,
      }),
    );

    await waitFor(
      () => {
        expect(telemetry).toHaveBeenCalledWith(
          expect.objectContaining({
            stage: "scheduled",
            attempt: 1,
            delayMs: 250,
          }),
        );
      },
      { timeout: 2000 },
    );

    unmount();
  });

  it("emits window telemetry events for scheduled and recovered retries", async () => {
    const getQuoteMock = vi.mocked(getQuote);
    let callCount = 0;
    getQuoteMock.mockImplementation(async () => {
      callCount += 1;
      if (callCount === 1) {
        throw new Error("Failed to fetch");
      }
      return buildQuoteResult("101.0");
    });

    const telemetryListener = vi.fn();
    window.addEventListener(QUOTE_RETRY_EVENT_NAME, telemetryListener as EventListener);

    try {
      const { result } = renderHook(() =>
        useQuoteRefresh("native", "USDC:G...", 100, "sell", {
          debounceMs: 1,
          maxAutoRetries: 1,
          retryBackoffMs: 5,
          maxRetryBackoffMs: 50,
          retryJitterRatio: 0,
          isOnline: true,
        }),
      );

      await waitFor(() => {
        expect(result.current.data?.total).toBe("101.0");
      });

      const stages = telemetryListener.mock.calls.map(([event]) =>
        (event as CustomEvent).detail.stage,
      );
      expect(stages).toContain("scheduled");
      expect(stages).toContain("succeeded");
    } finally {
      window.removeEventListener(
        QUOTE_RETRY_EVENT_NAME,
        telemetryListener as EventListener,
      );
    }
  });

  it("allows forced refresh to bypass the manual cooldown", async () => {
    const getQuoteMock = vi.mocked(getQuote);
    getQuoteMock
      .mockResolvedValueOnce(buildQuoteResult("98.0"))
      .mockResolvedValueOnce(buildQuoteResult("99.0"));

    const { result, unmount } = renderHook(() =>
      useQuoteRefresh("native", "USDC:G...", 100, "sell", {
        debounceMs: 0,
        manualRefreshCooldownMs: 5_000,
        isOnline: true,
      }),
    );

    await waitFor(() => {
      expect(result.current.data?.total).toBe("98.0");
    });

    act(() => {
      result.current.refresh();
      result.current.refresh({ force: true });
    });

    await waitFor(() => {
      expect(getQuoteMock).toHaveBeenCalledTimes(2);
      expect(result.current.data?.total).toBe("99.0");
    });

    unmount();
  });
});
