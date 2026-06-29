/**
 * Tests for useQuoteStreamStatus and deriveRawStatus.
 *
 * Covers:
 *  - deriveRawStatus pure helper
 *  - mode derivation: polling default, stream via wsConnected, explicit override
 *  - Status transitions: connected, disconnected (immediate), reconnecting (debounced)
 *  - Grace-period debounce / single-timer invariant
 *  - Polling fallback when wsConnected is false / not provided
 */

import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  deriveRawStatus,
  useQuoteStreamStatus,
} from "./useQuoteStreamStatus";

// ---------------------------------------------------------------------------
// deriveRawStatus — pure unit tests (no timers needed)
// ---------------------------------------------------------------------------

describe("deriveRawStatus", () => {
  it("returns disconnected when offline, regardless of other inputs", () => {
    expect(deriveRawStatus(false, null, false)).toBe("disconnected");
    expect(deriveRawStatus(true, new Error("x"), false)).toBe("disconnected");
  });

  it("returns reconnecting when online and recovering", () => {
    expect(deriveRawStatus(true, null, true)).toBe("reconnecting");
  });

  it("returns reconnecting when online and error is set", () => {
    expect(deriveRawStatus(false, new Error("oops"), true)).toBe("reconnecting");
  });

  it("returns reconnecting when online, recovering AND error is set", () => {
    expect(deriveRawStatus(true, new Error("oops"), true)).toBe("reconnecting");
  });

  it("returns connected when online with no error and not recovering", () => {
    expect(deriveRawStatus(false, null, true)).toBe("connected");
  });
});

// ---------------------------------------------------------------------------
// useQuoteStreamStatus — mode derivation
// ---------------------------------------------------------------------------

describe("useQuoteStreamStatus — mode", () => {
  it("defaults to polling when no mode option and wsConnected is false", () => {
    const { result } = renderHook(() =>
      useQuoteStreamStatus({ wsConnected: false })
    );
    expect(result.current.mode).toBe("polling");
  });

  it("defaults to polling when wsConnected is not provided", () => {
    const { result } = renderHook(() => useQuoteStreamStatus());
    expect(result.current.mode).toBe("polling");
  });

  it("reports mode: stream when wsConnected is true", () => {
    const { result } = renderHook(() =>
      useQuoteStreamStatus({ wsConnected: true })
    );
    expect(result.current.mode).toBe("stream");
  });

  it("respects explicit options.mode = 'stream' even when wsConnected is false", () => {
    const { result } = renderHook(() =>
      useQuoteStreamStatus({ wsConnected: false }, { mode: "stream" })
    );
    expect(result.current.mode).toBe("stream");
  });

  it("respects explicit options.mode = 'polling' even when wsConnected is true", () => {
    const { result } = renderHook(() =>
      useQuoteStreamStatus({ wsConnected: true }, { mode: "polling" })
    );
    expect(result.current.mode).toBe("polling");
  });

  it("switches to stream mode dynamically when wsConnected flips to true", () => {
    const { result, rerender } = renderHook(
      ({ wsConnected }: { wsConnected: boolean }) =>
        useQuoteStreamStatus({ wsConnected }),
      { initialProps: { wsConnected: false } }
    );
    expect(result.current.mode).toBe("polling");

    rerender({ wsConnected: true });
    expect(result.current.mode).toBe("stream");
  });

  it("falls back to polling mode when wsConnected drops to false", () => {
    const { result, rerender } = renderHook(
      ({ wsConnected }: { wsConnected: boolean }) =>
        useQuoteStreamStatus({ wsConnected }),
      { initialProps: { wsConnected: true } }
    );
    expect(result.current.mode).toBe("stream");

    rerender({ wsConnected: false });
    expect(result.current.mode).toBe("polling");
  });
});

// ---------------------------------------------------------------------------
// useQuoteStreamStatus — status transitions (fake timers)
// ---------------------------------------------------------------------------

describe("useQuoteStreamStatus — status transitions", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("starts with status: connected", () => {
    const { result } = renderHook(() => useQuoteStreamStatus());
    expect(result.current.status).toBe("connected");
  });

  it("transitions to disconnected immediately when isOnline = false", () => {
    const { result, rerender } = renderHook(
      ({ isOnline }: { isOnline: boolean }) =>
        useQuoteStreamStatus({ isOnline }),
      { initialProps: { isOnline: true } }
    );
    expect(result.current.status).toBe("connected");

    rerender({ isOnline: false });
    expect(result.current.status).toBe("disconnected");
  });

  it("transitions back to connected immediately when isOnline recovers", () => {
    const { result, rerender } = renderHook(
      ({ isOnline }: { isOnline: boolean }) =>
        useQuoteStreamStatus({ isOnline }),
      { initialProps: { isOnline: false } }
    );
    expect(result.current.status).toBe("disconnected");

    rerender({ isOnline: true });
    expect(result.current.status).toBe("connected");
  });

  it("debounces connected → reconnecting by the grace period", () => {
    const gracePeriod = 500;
    const { result, rerender } = renderHook(
      ({ isRecovering }: { isRecovering: boolean }) =>
        useQuoteStreamStatus(
          { isRecovering, isOnline: true },
          { reconnectGracePeriodMs: gracePeriod }
        ),
      { initialProps: { isRecovering: false } }
    );
    expect(result.current.status).toBe("connected");

    rerender({ isRecovering: true });
    // Before the grace period elapses the status should still be "connected"
    expect(result.current.status).toBe("connected");

    act(() => {
      vi.advanceTimersByTime(gracePeriod);
    });
    expect(result.current.status).toBe("reconnecting");
  });

  it("cancels pending reconnecting timer when recovery resolves within grace period", () => {
    const gracePeriod = 1_000;
    const { result, rerender } = renderHook(
      ({ isRecovering }: { isRecovering: boolean }) =>
        useQuoteStreamStatus(
          { isRecovering, isOnline: true },
          { reconnectGracePeriodMs: gracePeriod }
        ),
      { initialProps: { isRecovering: false } }
    );

    // Enter the debounce window
    rerender({ isRecovering: true });
    expect(result.current.status).toBe("connected");

    // Recover before the timer fires
    rerender({ isRecovering: false });
    act(() => {
      vi.advanceTimersByTime(gracePeriod + 100);
    });
    // Should stay connected because recovery was immediate
    expect(result.current.status).toBe("connected");
  });

  it("immediately transitions to disconnected even while reconnecting timer is pending", () => {
    const gracePeriod = 2_000;
    const { result, rerender } = renderHook(
      ({
        isRecovering,
        isOnline,
      }: {
        isRecovering: boolean;
        isOnline: boolean;
      }) =>
        useQuoteStreamStatus(
          { isRecovering, isOnline },
          { reconnectGracePeriodMs: gracePeriod }
        ),
      { initialProps: { isRecovering: true, isOnline: true } }
    );
    // Timer is pending but status is still "connected"
    expect(result.current.status).toBe("connected");

    // Going offline should bypass the timer immediately
    rerender({ isRecovering: true, isOnline: false });
    expect(result.current.status).toBe("disconnected");
  });

  it("resets the grace-period timer on rapid isRecovering toggles (single timer invariant)", () => {
    const gracePeriod = 500;
    const { result, rerender } = renderHook(
      ({ isRecovering }: { isRecovering: boolean }) =>
        useQuoteStreamStatus(
          { isRecovering, isOnline: true },
          { reconnectGracePeriodMs: gracePeriod }
        ),
      { initialProps: { isRecovering: false } }
    );

    // First toggle — starts the timer
    rerender({ isRecovering: true });
    act(() => {
      vi.advanceTimersByTime(300);
    });
    // Not elapsed yet
    expect(result.current.status).toBe("connected");

    // Second toggle — resets the timer
    rerender({ isRecovering: false });
    rerender({ isRecovering: true });
    act(() => {
      vi.advanceTimersByTime(300);
    });
    // Still within the reset window
    expect(result.current.status).toBe("connected");

    act(() => {
      vi.advanceTimersByTime(gracePeriod);
    });
    expect(result.current.status).toBe("reconnecting");
  });
});

// ---------------------------------------------------------------------------
// useQuoteStreamStatus — stream mode with status transitions
// ---------------------------------------------------------------------------

describe("useQuoteStreamStatus — stream mode status", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("reports mode:stream and status:connected while socket is healthy", () => {
    const { result } = renderHook(() =>
      useQuoteStreamStatus({
        wsConnected: true,
        isRecovering: false,
        error: null,
        isOnline: true,
      })
    );
    expect(result.current.mode).toBe("stream");
    expect(result.current.status).toBe("connected");
  });

  it("reports mode:polling and status:reconnecting after grace period when socket disconnects", () => {
    const gracePeriod = 500;
    const { result, rerender } = renderHook(
      ({ wsConnected, isRecovering }: { wsConnected: boolean; isRecovering: boolean }) =>
        useQuoteStreamStatus(
          { wsConnected, isRecovering, isOnline: true },
          { reconnectGracePeriodMs: gracePeriod }
        ),
      { initialProps: { wsConnected: true, isRecovering: false } }
    );
    expect(result.current.mode).toBe("stream");
    expect(result.current.status).toBe("connected");

    // Socket disconnects → useQuote sets isRecovering:true, wsConnected:false
    rerender({ wsConnected: false, isRecovering: true });
    expect(result.current.mode).toBe("polling"); // immediate mode fallback
    expect(result.current.status).toBe("connected"); // status still debounced

    act(() => {
      vi.advanceTimersByTime(gracePeriod);
    });
    expect(result.current.status).toBe("reconnecting");
  });

  it("returns mode:stream and status:connected once socket reconnects", () => {
    const gracePeriod = 500;
    const { result, rerender } = renderHook(
      ({ wsConnected, isRecovering }: { wsConnected: boolean; isRecovering: boolean }) =>
        useQuoteStreamStatus(
          { wsConnected, isRecovering, isOnline: true },
          { reconnectGracePeriodMs: gracePeriod }
        ),
      { initialProps: { wsConnected: false, isRecovering: true } }
    );

    // Simulate reconnect
    rerender({ wsConnected: true, isRecovering: false });
    expect(result.current.mode).toBe("stream");
    expect(result.current.status).toBe("connected");
  });
});
