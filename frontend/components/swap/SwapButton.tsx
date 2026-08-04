'use client';

import { AlertCircle, Wallet, ShieldOff, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { useSwapI18n } from "@/lib/swap-i18n";
import { useReducedMotion } from "@/hooks/useReducedMotion";

export type SwapButtonState =
  | "no_wallet"
  | "no_amount"
  | "insufficient_balance"
  | "high_price_impact"
  | "high_impact_warning"
  | "slippage_ack_required"
  | "refreshing_quote"
  | "stale_quote"
  | "ready"
  | "executing"
  | "error"
  | "permission_blocked";

interface SwapButtonProps {
  state: SwapButtonState;
  onSwap: () => void;
  onConnectWallet?: () => void;
  onRefreshQuote?: () => void;
  isLoading?: boolean;
  className?: string;
}

export function SwapButton({
  state,
  onSwap,
  onConnectWallet,
  onRefreshQuote,
  isLoading = false,
  className,
}: SwapButtonProps) {
  const { t } = useSwapI18n();
  const prefersReducedMotion = useReducedMotion();

  const getButtonProps = () => {
    switch (state) {
      case "no_wallet":
        return {
          label: t("swap.cta.connectWallet"),
          onClick: onConnectWallet,
          disabled: false,
          variant: "default" as const,
          icon: <Wallet className="mr-2 h-5 w-5" />,
          className: "bg-primary hover:bg-primary/90 shadow-primary/20",
        };
      case "no_amount":
        return {
          label: t("swap.cta.enterAmount"),
          disabled: true,
          disabledReason: "Enter an amount to see available routes.",
          variant: "secondary" as const,
          className: "bg-muted/50 text-muted-foreground",
        };
      case "insufficient_balance":
        return {
          label: t("swap.cta.insufficientBalance"),
          disabled: true,
          disabledReason: "Your wallet balance is too low to complete this swap.",
          variant: "destructive" as const,
          icon: <AlertCircle className="mr-2 h-5 w-5" />,
          className: "bg-destructive/10 text-destructive border-destructive/20 border",
        };
      case "high_price_impact":
        return {
          label: t("swap.simulation.highImpactTitle"),
          disabled: true,
          disabledReason: "This trade's price impact is too high to execute safely.",
          variant: "destructive" as const,
          icon: <AlertCircle className="mr-2 h-5 w-5" />,
          className: "bg-destructive shadow-destructive/20",
        };
      case "high_impact_warning":
        return {
          label: t("swap.cta.swapAnyway"),
          onClick: onSwap,
          disabled: isLoading,
          disabledReason: isLoading ? "Transaction is being submitted to the network." : undefined,
          variant: "destructive" as const,
          icon: isLoading ? <Spinner className="mr-2" label={t("swap.cta.swapping")} /> : <AlertCircle className="mr-2 h-5 w-5" />,
          className: cn(
            "bg-destructive hover:bg-destructive/90 shadow-lg shadow-destructive/20",
            !prefersReducedMotion && "animate-pulse"
          ),
        };
      case "slippage_ack_required":
        return {
          label: "Acknowledge Slippage",
          disabled: true,
          disabledReason: "Acknowledge the slippage warning before continuing.",
          variant: "destructive" as const,
          icon: <AlertCircle className="mr-2 h-5 w-5" />,
          className: "bg-destructive/10 text-destructive border-destructive/20 border",
        };
      case "executing":
        return {
          label: t("swap.cta.swapping"),
          disabled: true,
          disabledReason: "Transaction is being submitted to the network.",
          variant: "default" as const,
          icon: <Spinner className="mr-2" label={t("swap.cta.swapping")} />,
        };
      case "refreshing_quote":
        return {
          label: t("swap.cta.loadingQuote"),
          disabled: true,
          disabledReason: "Waiting for a fresh quote before you can swap.",
          variant: "outline" as const,
          icon: <Spinner className="mr-2" label={t("swap.cta.loadingQuote")} />,
          className: "border-primary/40 text-primary",
        };
      case "stale_quote":
        return {
          label: t("swap.card.refreshQuote"),
          onClick: onRefreshQuote,
          disabled: isLoading || !onRefreshQuote,
          disabledReason: isLoading
            ? "Fetching a fresh quote."
            : "Quote expired — refresh to continue.",
          variant: "default" as const,
          icon: <RefreshCw className={cn("mr-2 h-5 w-5", isLoading && "animate-spin")} />,
          className: "bg-primary hover:bg-primary/90 shadow-lg shadow-primary/20",
        };
      case "error":
        return {
          label: t("swap.cta.errorFetchingQuote"),
          disabled: true,
          disabledReason: "Unable to fetch a quote. Try again shortly.",
          variant: "outline" as const,
          icon: <AlertCircle className="mr-2 h-5 w-5" />,
          className: "border-destructive/50 text-destructive",
        };
      case "permission_blocked":
        return {
          label: "Wallet permissions required",
          disabled: true,
          disabledReason: "Your wallet has not granted the required permissions.",
          variant: "destructive" as const,
          icon: <ShieldOff className="mr-2 h-5 w-5" />,
          className: "bg-destructive/10 text-destructive border border-destructive/20",
        };
      case "ready":
      default:
        return {
          label: t("swap.cta.reviewSwap"),
          onClick: onSwap,
          disabled: isLoading,
          disabledReason: isLoading ? "Transaction is being submitted to the network." : undefined,
          variant: "default" as const,
          icon: isLoading ? <Spinner className="mr-2" label={t("swap.cta.swapping")} /> : null,
          className: cn(
            "bg-primary hover:bg-primary/90 shadow-lg shadow-primary/20 hover:shadow-primary/30",
            !prefersReducedMotion && "active:scale-[0.98] transition-all"
          ),
        };
    }
  };

  const props = getButtonProps();

  const button = (
    <Button
      size="lg"
      variant={props.variant}
      disabled={props.disabled}
      aria-disabled={props.disabled}
      onClick={props.disabled ? undefined : props.onClick}
      className={cn(
        "h-14 w-full text-lg font-bold rounded-2xl shadow-md",
        !prefersReducedMotion && "transition-all duration-300",
        props.className,
        className
      )}
    >
      {props.icon}
      {props.label}
    </Button>
  );

  if (!props.disabled || !props.disabledReason) {
    return button;
  }

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>{button}</TooltipTrigger>
        <TooltipContent className="max-w-[240px] text-xs leading-relaxed">
          {props.disabledReason}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
