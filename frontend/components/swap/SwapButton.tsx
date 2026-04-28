'use client';

import { Button } from "@/components/ui/button";
import { Loader2, AlertCircle, Wallet } from "lucide-react";
import { cn } from "@/lib/utils";
import { useSwapI18n } from "@/lib/swap-i18n";

export type SwapButtonState = 
  | "no_wallet"
  | "no_amount"
  | "insufficient_balance"
  | "high_price_impact"
  | "high_impact_warning"
  | "refreshing_quote"
  | "ready"
  | "executing"
  | "error";

interface SwapButtonProps {
  state: SwapButtonState;
  onSwap: () => void;
  onConnectWallet?: () => void;
  isLoading?: boolean;
  className?: string;
}

export function SwapButton({
  state,
  onSwap,
  onConnectWallet,
  isLoading = false,
  className,
}: SwapButtonProps) {
  const { t } = useSwapI18n();
  
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
          variant: "secondary" as const,
          className: "bg-muted/50 text-muted-foreground",
        };
      case "insufficient_balance":
        return {
          label: t("swap.cta.insufficientBalance"),
          disabled: true,
          variant: "destructive" as const,
          className: "bg-destructive/10 text-destructive border-destructive/20 border",
        };
      case "high_price_impact":
        return {
          label: t("swap.simulation.highImpactTitle"),
          disabled: true,
          variant: "destructive" as const,
          icon: <AlertCircle className="mr-2 h-5 w-5" />,
          className: "bg-destructive shadow-destructive/20",
        };
      case "high_impact_warning":
        return {
          label: t("swap.cta.swapAnyway"),
          onClick: onSwap,
          disabled: isLoading,
          variant: "destructive" as const,
          icon: isLoading ? <Loader2 className="mr-2 h-5 w-5 animate-spin" /> : <AlertCircle className="mr-2 h-5 w-5" />,
          className: "bg-destructive hover:bg-destructive/90 shadow-lg shadow-destructive/20 animate-pulse",
        };
      case "executing":
        return {
          label: t("swap.cta.swapping"),
          disabled: true,
          variant: "default" as const,
          icon: <Loader2 className="mr-2 h-5 w-5 animate-spin" />,
        };
      case "refreshing_quote":
        return {
          label: t("swap.cta.loadingQuote"),
          disabled: true,
          variant: "outline" as const,
          icon: <Loader2 className="mr-2 h-5 w-5 animate-spin" />,
          className: "border-primary/40 text-primary",
        };
      case "error":
        return {
          label: t("swap.cta.errorFetchingQuote"),
          disabled: true,
          variant: "outline" as const,
          className: "border-destructive/50 text-destructive",
        };
      case "ready":
      default:
        return {
          label: t("swap.cta.reviewSwap"),
          onClick: onSwap,
          disabled: isLoading,
          variant: "default" as const,
          icon: isLoading ? <Loader2 className="mr-2 h-5 w-5 animate-spin" /> : null,
          className: "bg-primary hover:bg-primary/90 shadow-lg shadow-primary/20 hover:shadow-primary/30 active:scale-[0.98] transition-all",
        };
    }
  };

  const props = getButtonProps();

  return (
    <Button
      size="lg"
      variant={props.variant}
      disabled={props.disabled}
      onClick={props.onClick}
      className={cn(
        "h-14 w-full text-lg font-bold rounded-2xl shadow-md transition-all duration-300",
        props.className,
        className
      )}
    >
      {props.icon}
      {props.label}
    </Button>
  );
}
