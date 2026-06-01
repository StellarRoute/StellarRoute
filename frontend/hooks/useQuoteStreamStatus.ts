"use client";

import { useEffect, useRef, useState } from "react";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type ConnectionStatus = "connected" | "reconnecting" | "disconnected";
export type Mode = "stream" | "polling";

export interface UseQuoteStreamStatusOptions {
  /**
   * Debounce window for connected → reconnecting transitions.
   * Prevents noisy flicker during brief network hiccups.
   * Default: 3000 ms
   */
  reconnectGracePeriodMs?: number;
  /**
   * Active data-delivery mode.
   * "stream" = WebSocket (future); "polling" = HTTP polling (current default).
   */
  mode?: Mode;
}

export interface UseQuoteStreamStatusInputs {
  /** True while transient quote failures are being retried (from useQuoteRefresh). */
  isRecovering: boolean;
  /** Current quote fetch error, if any (from useQuoteRefresh). */
  error: Error | null;
  /** Browser network connectivity status (from useOnlineStatus). */
  isOnline: boolean;
}

export interface UseQuoteStreamStatusResult {
  /** Derived connection status. */
  status: ConnectionStatus;
  /** Active data delivery mode. */
  mode: Mode;
}

// ---------------------------------------------------------------------------
// Pure derivation helper (exported for testing — Property 8: determinism)
// ---------------------------------------------------------------------------

/**
 * Maps raw hook inputs to a ConnectionStatus with no side effects.
 * This is the ground-truth derivation logic used by the hook after any
 * pending grace-period timer resolves.
 */
export function deriveRawStatus(
  isRecovering: boolean,
  error: Error | null,
  isOnline: boolean
): ConnectionStatus {
  if (!isOnline) return "disconnected";
  if (isRecovering || error !== null) return "reconnecting";
  return "connected";
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

const DEFAULT_GRACE_PERIOD_MS = 3_000;

/**
 * Derives a discrete ConnectionStatus from useQuoteRefresh outputs with
 * flicker suppression via a configurable grace-period debounce.
 *
 * Transition rules:
 * - connected → reconnecting: debounced by reconnectGracePeriodMs (default 3 s)
 * - reconnecting → connected: immediate
 * - * → disconnected (isOnline=false): immediate
 *
 * Only one debounce timer is active at a time; rapid transitions reset it.
 */
export function useQuoteStreamStatus(
  inputs: Partial<UseQuoteStreamStatusInputs> = {},
  options: UseQuoteStreamStatusOptions = {}
): UseQuoteStreamStatusResult {
  const {
    isRecovering = false,
    error = null,
    isOnline = true,
  } = inputs;

  const gracePeriodMs =
    typeof options.reconnectGracePeriodMs === "number" &&
    options.reconnectGracePeriodMs > 0
      ? options.reconnectGracePeriodMs
      : DEFAULT_GRACE_PERIOD_MS;

  const mode: Mode =
    options.mode === "stream" || options.mode === "polling"
      ? options.mode
      : "polling";

  const rawStatus = deriveRawStatus(isRecovering, error, isOnline);

  // Emitted status — starts as "connected" (safe default before any request)
  const [status, setStatus] = useState<ConnectionStatus>("connected");

  // Single pending timer ref — reset on each new connected→reconnecting entry
  const timerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  useEffect(() => {
    // Immediate transition: offline always wins
    if (rawStatus === "disconnected") {
      if (timerRef.current !== undefined) {
        clearTimeout(timerRef.current);
        timerRef.current = undefined;
      }
      setStatus("disconnected");
      return;
    }

    // Immediate transition: recovery from reconnecting → connected
    if (rawStatus === "connected") {
      if (timerRef.current !== undefined) {
        clearTimeout(timerRef.current);
        timerRef.current = undefined;
      }
      setStatus("connected");
      return;
    }

    // rawStatus === "reconnecting": debounce the connected → reconnecting transition
    // Reset any existing timer (single timer invariant — Property 5)
    if (timerRef.current !== undefined) {
      clearTimeout(timerRef.current);
    }
    timerRef.current = setTimeout(() => {
      timerRef.current = undefined;
      setStatus("reconnecting");
    }, gracePeriodMs);

    return () => {
      if (timerRef.current !== undefined) {
        clearTimeout(timerRef.current);
        timerRef.current = undefined;
      }
    };
  }, [rawStatus, gracePeriodMs]);

  return { status, mode };
}
