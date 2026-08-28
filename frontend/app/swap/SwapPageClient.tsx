"use client";

import { OnboardingChecklist } from "@/components/swap/OnboardingChecklist";
import { SplitView } from "@/components/swap/SplitView";
import { useSplitView } from "@/hooks/useSplitView";
import { RoutesBetaGate } from "@/src/components/RoutesBetaGate";
import { useFeatureFlag } from "@/hooks/useFeatureFlag";
import {
  CrossChainSwapDeck,
} from "@/components/swap/cross-chain/CrossChainSwapDeck";
import dynamic from "next/dynamic";

const SwapCard = dynamic(
  () => import("@/components/swap/SwapCard").then((m) => m.SwapCard),
  {
    ssr: false,
    loading: () => (
      <div className="flex h-[520px] w-full max-w-[480px] items-center justify-center rounded-2xl chart-panel sm:h-[580px] sm:rounded-3xl">
        <div className="flex flex-col items-center gap-3">
          <div className="h-8 w-8 rounded-full border-4 border-primary border-t-transparent animate-spin" />
          <span className="font-mono text-xs text-muted-foreground animate-pulse">
            Initializing swap interface...
          </span>
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
      className="w-full"
    />
  );
}

/**
 * Swap UI v2 — cross-chain route deck (flag-gated).
 * Disabled falls back to the legacy routes/swap card experience unchanged.
 */
function SwapUiV2Surface() {
  const { enabled, loading } = useFeatureFlag("swap_ui_v2");

  if (loading) {
    return (
      <RoutesBetaGate fallback={<SwapLegacyRoutes />}>
        <SwapRoutesBeta />
      </RoutesBetaGate>
    );
  }

  if (!enabled) {
    return (
      <RoutesBetaGate fallback={<SwapLegacyRoutes />}>
        <SwapRoutesBeta />
      </RoutesBetaGate>
    );
  }

  return <CrossChainSwapDeck />;
}

export function SwapPageClient() {
  return (
    <div className="mx-auto w-full max-w-[960px] space-y-4 overflow-x-hidden px-0 sm:space-y-5">
      <div className="space-y-1 px-1 sm:px-0">
        <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-primary">
          Trade deck
        </p>
        <h1 className="brand-wordmark text-2xl text-foreground sm:text-3xl">
          Route &amp; swap
        </h1>
        <p className="max-w-xl text-sm text-muted-foreground">
          Compare venues, lock a quote, then sign — built for Stellar SDEX and
          Soroban liquidity.
        </p>
      </div>
      <OnboardingChecklist />
      <SwapUiV2Surface />
    </div>
  );
}
