"use client";

import { SplitView } from "@/components/swap/SplitView";
import { useSplitView } from "@/hooks/useSplitView";
import { RoutesBetaGate } from "@/src/components/RoutesBetaGate";
import dynamic from "next/dynamic";

const SwapCard = dynamic(
  () => import("@/components/swap/SwapCard").then((m) => m.SwapCard),
  {
    ssr: false,
    loading: () => (
      <div className="w-full max-w-[480px] h-[580px] bg-card/40 backdrop-blur-md rounded-[32px] border border-border/20 flex items-center justify-center shadow-2xl">
        <div className="flex flex-col items-center gap-3">
          <div className="h-8 w-8 rounded-full border-4 border-primary border-t-transparent animate-spin" />
          <span className="text-xs text-muted-foreground font-mono animate-pulse">Initializing swap interface...</span>
        </div>
      </div>
    )
  }
);

const RouteDisplay = dynamic(
  () => import("@/components/swap/RouteDisplay").then((m) => m.RouteDisplay),
  { ssr: false }
);

/**
 * Fallback when `routes_beta` is off (default).
 * Standard swap card without split-view route panel or alternative-route picker.
 */
function SwapLegacyRoutes() {
  return (
    <div className="w-full max-w-[480px] mx-auto">
      <SwapCard showRoutePicker={false} />
    </div>
  );
}

/**
 * Routes beta UI when `routes_beta` is on.
 * Split-view layout with dedicated route details panel and in-card route picker.
 *
 * Enable with `NEXT_PUBLIC_FLAG_ROUTES_BETA=true` or
 * `window.__STELLAR_ROUTE_FLAGS__ = { routes_beta: true }`.
 */
function SwapRoutesBeta() {
  const { isSplit, toggleSplit } = useSplitView();

  return (
    <SplitView
      isSplit={isSplit}
      onToggle={toggleSplit}
      primary={<SwapCard showRoutePicker />}
      secondary={
        <div className="rounded-xl border border-border/50 bg-card p-4">
          <h2 className="text-sm font-semibold mb-3">Route Details</h2>
          <RouteDisplay amountOut="0" />
        </div>
      }
      className="w-full max-w-[960px] mx-auto"
    />
  );
}

export function SwapPageClient() {
  return (
    <RoutesBetaGate fallback={<SwapLegacyRoutes />}>
      <SwapRoutesBeta />
    </RoutesBetaGate>
  );
}
