"use client";

import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";
import type { ConnectionStatus, Mode } from "@/hooks/useQuoteStreamStatus";

export interface QuoteStreamStatusIndicatorProps {
  /** Current connection status derived from useQuoteStreamStatus. */
  status: ConnectionStatus;
  /** Active data delivery mode. */
  mode: Mode;
  /**
   * When true, the indicator renders nothing when status is "connected".
   * Useful for minimal UIs that only want to surface problems.
   * Default: false
   */
  hideWhenConnected?: boolean;
  /** Additional CSS classes applied to the root element. */
  className?: string;
}

// ---------------------------------------------------------------------------
// Visual config per (status, mode) combination
// ---------------------------------------------------------------------------

interface VisualConfig {
  label: string;
  dotClass: string;
  pulses: boolean;
  ariaLive: "polite" | "assertive";
}

function getVisualConfig(status: ConnectionStatus, mode: Mode): VisualConfig {
  if (status === "disconnected") {
    return {
      label: "Disconnected",
      dotClass: "bg-red-500",
      pulses: false,
      ariaLive: "assertive",
    };
  }

  if (status === "reconnecting") {
    return {
      label: mode === "polling" ? "Reconnecting (polling)" : "Reconnecting",
      dotClass: "bg-amber-500",
      pulses: true,
      ariaLive: "polite",
    };
  }

  // connected
  if (mode === "polling") {
    return {
      label: "Polling",
      dotClass: "bg-blue-500",
      pulses: false,
      ariaLive: "polite",
    };
  }

  return {
    label: "Live",
    dotClass: "bg-green-500",
    pulses: false,
    ariaLive: "polite",
  };
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

/**
 * Displays the real-time quote stream connection status as a colored dot
 * with a text label. Fully accessible: aria-label on root, aria-live region
 * for announcements, aria-hidden on the decorative dot.
 *
 * This is a controlled, stateless presentational component — all state lives
 * in useQuoteStreamStatus.
 */
export function QuoteStreamStatusIndicator({
  status,
  mode,
  hideWhenConnected = false,
  className,
}: QuoteStreamStatusIndicatorProps) {
  // Respect prefers-reduced-motion: replace pulse with static dot
  const [reducedMotion, setReducedMotion] = useState(false);

  useEffect(() => {
    if (typeof window === "undefined") return;
    const mq = window.matchMedia("(prefers-reduced-motion: reduce)");
    setReducedMotion(mq.matches);

    const handler = (e: MediaQueryListEvent) => setReducedMotion(e.matches);
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, []);

  // Conditional rendering: hide when connected if requested
  if (hideWhenConnected && status === "connected") {
    return null;
  }

  const { label, dotClass, pulses, ariaLive } = getVisualConfig(status, mode);
  const shouldPulse = pulses && !reducedMotion;

  return (
    <div
      role="status"
      aria-label={`Quote stream status: ${label}`}
      aria-live={ariaLive}
      aria-atomic="true"
      className={cn(
        "inline-flex items-center gap-1.5 text-xs font-medium text-muted-foreground",
        className
      )}
    >
      {/* Decorative dot — hidden from screen readers */}
      <span
        aria-hidden="true"
        className={cn(
          "inline-block h-2 w-2 rounded-full flex-shrink-0",
          dotClass,
          shouldPulse && "animate-pulse"
        )}
      />
      {/* Text label — always present so colour is never the sole indicator */}
      <span>{label}</span>
    </div>
  );
}
