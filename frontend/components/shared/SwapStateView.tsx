"use client";

/**
 * SwapStateView — unified empty/error/loading primitives for swap modules.
 *
 * Wraps ViewState with swap-specific copy and icons for each context:
 * quote, routes, history, wallet.
 */

import { Button } from "@/components/ui/button";
import { ViewState } from "@/components/shared/ViewState";

type SwapContext = "quote" | "routes" | "history" | "wallet";
type StateVariant = "loading" | "empty" | "error";

interface SwapStateViewProps {
  context: SwapContext;
  variant: StateVariant;
  /** Override the default error message */
  errorMessage?: string;
  /** Retry callback shown on error */
  onRetry?: () => void;
  className?: string;
}

const COPY: Record<SwapContext, Record<StateVariant, { title: string; description: string }>> = {
  quote: {
    loading: { title: "Fetching quote…", description: "Finding the best price for your swap." },
    empty: { title: "No quote yet", description: "Enter an amount to see a price quote." },
    error: { title: "Quote unavailable", description: "Could not fetch a quote. Please try again." },
  },
  routes: {
    loading: { title: "Finding routes…", description: "Searching for optimal swap paths." },
    empty: { title: "No routes found", description: "There are no available routes for this pair." },
    error: { title: "Routes unavailable", description: "Could not load routes. Please try again." },
  },
  history: {
    loading: { title: "Loading history…", description: "Fetching your recent transactions." },
    empty: { title: "No transactions yet", description: "Your completed swaps will appear here." },
    error: { title: "History unavailable", description: "Could not load transaction history." },
  },
  wallet: {
    loading: { title: "Connecting…", description: "Waiting for wallet connection." },
    empty: { title: "Wallet not connected", description: "Connect your wallet to start swapping." },
    error: { title: "Wallet error", description: "There was a problem with your wallet." },
  },
};

export function SwapStateView({
  context,
  variant,
  errorMessage,
  onRetry,
  className,
}: SwapStateViewProps) {
  const { title, description } = COPY[context][variant];

  const action =
    variant === "error" && onRetry ? (
      <Button variant="outline" size="sm" onClick={onRetry}>
        Try again
      </Button>
    ) : undefined;

  return (
    <ViewState
      variant={variant}
      title={title}
      description={errorMessage && variant === "error" ? errorMessage : description}
      action={action}
      className={className}
    />
  );
}
