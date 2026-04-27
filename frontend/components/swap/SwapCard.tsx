'use client';

import { useState, useCallback, useEffect, useMemo, useRef } from 'react';
import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { ArrowUpDown, RefreshCw } from 'lucide-react';
import { AmountInput } from './AmountInput';
import { TokenSelector } from './TokenSelector';
import { PriceInfoPanel } from './PriceInfoPanel';
import { RouteDisplay } from './RouteDisplay';
import type { AlternativeRoute } from './RouteDisplay';
import { SwapButton, SwapButtonState } from './SwapButton';
import { SettingsPanel } from '../settings/SettingsPanel';
import { HighImpactConfirmModal } from './HighImpactConfirmModal';
import { QuoteStreamStatusIndicator } from './QuoteStreamStatusIndicator';
import { SessionRecoveryModal } from './SessionRecoveryModal';
import { useSwapState } from '@/hooks/useSwapState';
import {
  SESSION_RECOVERY_THRESHOLD_MS,
  type TradeFormSnapshot,
} from '@/hooks/useTradeFormStorage';
import { useOnlineStatus } from '@/hooks/useOnlineStatus';
import { useQuoteStreamStatus } from '@/hooks/useQuoteStreamStatus';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';
import { useSwapI18n } from '@/lib/swap-i18n';
import { quoteExportToCsv, type QuoteExportPayload } from '@/lib/quote-export';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

export function SwapCard() {
  const { t } = useSwapI18n();
  const {
    fromToken,
    setFromToken,
    toToken,
    setToToken,
    fromAmount,
    setFromAmount,
    toAmount,
    slippage,
    setSlippage,
    deadline,
    setDeadline,
    quote,
    switchTokens,
    formattedRate,
    pendingRecovery,
    restorePending,
    discardPending,
    hasRecoverableState,
    snapshotCurrent,
    reset,
  } = useSwapState();

  const [isConnected, setIsConnected] = useState(false);
  const [isSwapping, setIsSwapping] = useState(false);
  const [isConfirmModalOpen, setIsConfirmModalOpen] = useState(false);
  const [selectedRoute, setSelectedRoute] = useState<AlternativeRoute | null>(null);
  const [wakeSnapshot, setWakeSnapshot] = useState<TradeFormSnapshot | null>(null);
  const [wakeRecoveryOpen, setWakeRecoveryOpen] = useState(false);
  const [recoveryRequestedAt, setRecoveryRequestedAt] = useState<number | null>(null);
  const [isRecoveringSession, setIsRecoveringSession] = useState(false);
  const [shortcutHelpOpen, setShortcutHelpOpen] = useState(false);
  const lastFocusedElementRef = useRef<HTMLElement | null>(null);
  const hiddenAtRef = useRef<number | null>(null);
  const recoveryReason: 'refresh' | 'wake' | null = wakeRecoveryOpen
    ? 'wake'
    : pendingRecovery
      ? 'refresh'
      : null;
  const requiresFreshQuote =
    recoveryRequestedAt !== null &&
    (quote.lastQuotedAtMs === null ||
      quote.lastQuotedAtMs < recoveryRequestedAt ||
      quote.loading ||
      quote.isStale);

  // Connection status indicator
  const { isOnline } = useOnlineStatus();
  const { status: streamStatus, mode: streamMode } = useQuoteStreamStatus({
    isRecovering: quote.isRecovering,
    error: quote.error,
    isOnline,
  });
  
  // Mock balance
  const fromBalance = "100.00"; 
  const fromSymbol = fromToken === 'native' ? 'XLM' : fromToken.split(':')[0];
  const toSymbol = toToken === 'native' ? 'XLM' : toToken.split(':')[0];

  const buttonState = useMemo<SwapButtonState>(() => {
    if (isSwapping) return "executing";
    if (!isConnected) return "no_wallet";
    if (!fromAmount || parseFloat(fromAmount) === 0) return "no_amount";
    if (quote.error) return "error";
    if (requiresFreshQuote) return "refreshing_quote";
    if (parseFloat(fromAmount) > parseFloat(fromBalance)) return "insufficient_balance";
    if (quote.priceImpact > 10) return "high_impact_warning";
    if (quote.loading) return "refreshing_quote";
    if (quote.isStale) return "error";
    return "ready";
  }, [
    fromAmount,
    fromBalance,
    isConnected,
    isSwapping,
    quote.error,
    quote.isStale,
    quote.loading,
    quote.priceImpact,
    requiresFreshQuote,
  ]);

  useEffect(() => {
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'hidden') {
        hiddenAtRef.current = Date.now();
        return;
      }

      const hiddenAt = hiddenAtRef.current;
      hiddenAtRef.current = null;

      if (
        document.visibilityState !== 'visible' ||
        hiddenAt === null ||
        Date.now() - hiddenAt < SESSION_RECOVERY_THRESHOLD_MS ||
        !hasRecoverableState
      ) {
        return;
      }

      setWakeRecoveryOpen(true);
      setWakeSnapshot(snapshotCurrent());
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);
    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange);
    };
  }, [hasRecoverableState, snapshotCurrent]);

  const closeRecoveryModal = useCallback(() => {
    setWakeRecoveryOpen(false);
    setWakeSnapshot(null);
  }, []);

  const handleDiscardRecovery = useCallback(() => {
    if (recoveryReason === 'refresh') {
      discardPending();
    } else {
      reset();
      setSelectedRoute(null);
    }
    setRecoveryRequestedAt(null);
    closeRecoveryModal();
  }, [closeRecoveryModal, discardPending, recoveryReason, reset]);

  const handleRestoreRecovery = useCallback(async () => {
    setSelectedRoute(null);
    setRecoveryRequestedAt(Date.now());
    setIsRecoveringSession(true);

    try {
      if (recoveryReason === 'refresh') {
        restorePending();
        closeRecoveryModal();
        // Force quote refresh after restoring form state
        await quote.refresh({ force: true });
      } else {
        closeRecoveryModal();
        await quote.refresh({ force: true });
      }
    } catch (error) {
      console.error('Failed to restore session:', error);
      throw error; // Let modal handle the error display
    } finally {
      setIsRecoveringSession(false);
    }
  }, [closeRecoveryModal, quote, recoveryReason, restorePending]);

  const executeSwap = useCallback(async () => {
    setIsSwapping(true);
    // Simulate transaction delay
    await new Promise(resolve => setTimeout(resolve, 2000));
    setIsSwapping(false);
    toast.success(`Successfully swapped ${fromAmount} ${fromSymbol} for ${parseFloat(toAmount).toFixed(4)} ${toSymbol}`);
  }, [fromAmount, fromSymbol, toAmount, toSymbol]);

  const handleSwap = useCallback(async () => {
    if (quote.priceImpact > 5) {
      setIsConfirmModalOpen(true);
      return;
    }
    await executeSwap();
  }, [quote.priceImpact, executeSwap]);

  const handleMax = useCallback(() => {
    setFromAmount(fromBalance);
  }, [fromBalance, setFromAmount]);

  const handleSwitchTokens = useCallback(() => {
    setSelectedRoute(null);
    switchTokens();
  }, [switchTokens]);

  useEffect(() => {
    const onKeydown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const isEditable = target
        ? target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.isContentEditable
        : false;

      if (event.key === "?" && !isEditable) {
        event.preventDefault();
        lastFocusedElementRef.current = document.activeElement as HTMLElement;
        setShortcutHelpOpen(true);
      }

      if (event.key.toLowerCase() === "r" && event.altKey) {
        event.preventDefault();
        quote.refresh({ force: true });
      }

      if (event.key === "1" && event.altKey) {
        event.preventDefault();
        document.querySelectorAll<HTMLInputElement>('input[placeholder="0.00"]')[0]?.focus();
      }

      if (event.key === "2" && event.altKey) {
        event.preventDefault();
        document.querySelectorAll<HTMLInputElement>('input[placeholder="0.00"]')[1]?.focus();
      }
    };

    window.addEventListener("keydown", onKeydown);
    return () => window.removeEventListener("keydown", onKeydown);
  }, [quote]);

  const handleShortcutOpenChange = useCallback((open: boolean) => {
    setShortcutHelpOpen(open);
    if (!open) {
      lastFocusedElementRef.current?.focus();
    }
  }, []);

  const handleExport = useCallback((format: "json" | "csv") => {
    const payload: QuoteExportPayload = {
      exportedAt: new Date().toISOString(),
      market: {
        fromAsset: fromSymbol,
        toAsset: toSymbol,
        fromAmount,
        expectedToAmount: toAmount,
      },
      pricing: {
        rate: formattedRate,
        priceImpactPct: quote.priceImpact.toFixed(2),
        minimumReceived: `${(parseFloat(toAmount || '0') * (1 - slippage / 100)).toFixed(4)} ${toSymbol}`,
        networkFee: quote.fee ? `${quote.fee.toFixed(5)} XLM` : '0.00001 XLM',
      },
      route: {
        selectedVenue: selectedRoute?.venue ?? "auto",
        routeSummary:
          selectedRoute?.hops?.map((hop) => `${hop.fromAsset}->${hop.toAsset}`).join(" | ") ??
          "best-route",
      },
    };
    const serialized = format === "json"
      ? JSON.stringify(payload, null, 2)
      : quoteExportToCsv(payload);
    const blob = new Blob([serialized], {
      type: format === "json" ? "application/json" : "text/csv;charset=utf-8",
    });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `stellarroute-quote-summary.${format}`;
    anchor.click();
    URL.revokeObjectURL(url);
    toast.success(t("swap.quote.exportSuccess", { format: format.toUpperCase() }));
  }, [formattedRate, fromAmount, fromSymbol, quote.fee, quote.priceImpact, selectedRoute, slippage, t, toAmount, toSymbol]);

  return (
    <div data-testid="swap-card" className="w-full max-w-[480px] mx-auto perspective-1000">
      <Card className="relative overflow-hidden border-border/40 bg-background/60 backdrop-blur-xl shadow-2xl rounded-[32px] transition-all duration-500 hover:shadow-primary/5">
        {/* Animated Background Gradients */}
        <div className="absolute -top-24 -left-24 w-48 h-48 bg-primary/10 rounded-full blur-3xl animate-pulse" />
        <div className="absolute -bottom-24 -right-24 w-48 h-48 bg-blue-500/10 rounded-full blur-3xl animate-pulse delay-700" />
        
        <CardContent className="p-6 space-y-4">
          {/* Header */}
          <div className="flex items-center justify-between mb-2">
            <h2 className="text-xl font-bold tracking-tight bg-gradient-to-br from-foreground to-foreground/60 bg-clip-text text-transparent">
              Swap
            </h2>
            <div className="flex items-center gap-1">
              <QuoteStreamStatusIndicator status={streamStatus} mode={streamMode} />
              <SettingsPanel
                slippage={slippage}
                onSlippageChange={setSlippage}
                deadline={deadline}
                onDeadlineChange={setDeadline}
                onReset={() => {
      reset();
      setSelectedRoute(null);
    }}
              />
              <Button 
                variant="ghost" 
                size="icon" 
                onClick={() => quote.refresh()} 
                disabled={quote.loading}
                aria-label={t("swap.card.refreshQuote")}
                className="h-9 w-9 rounded-xl hover:bg-muted/80"
              >
                <RefreshCw className={cn("h-4.5 w-4.5 text-muted-foreground", quote.loading && "animate-spin")} />
              </Button>
            </div>
          </div>

          {/* Pay Section */}
          <div className="space-y-2 group">
            <div className="bg-muted/30 hover:bg-muted/40 transition-colors rounded-2xl p-4 border border-border/20 focus-within:border-primary/30 focus-within:ring-4 focus-within:ring-primary/5">
              <div className="flex justify-between items-start mb-1">
                <AmountInput
                  label={t("swap.pair.youPay")}
                  value={fromAmount}
                  onChange={setFromAmount}
                  onMax={handleMax}
                  balance={`${fromBalance} ${fromSymbol}`}
                  className="flex-1"
                />
                <TokenSelector
                  selectedAsset={fromToken}
                  onSelect={setFromToken}
                  className="mt-6"
                />
              </div>
            </div>
          </div>

          {/* Toggle Button */}
          <div className="relative h-2 flex items-center justify-center z-10">
            <Button
              variant="outline"
              size="icon"
              onClick={handleSwitchTokens}
              className="absolute h-10 w-10 rounded-xl bg-background border-border/40 shadow-lg hover:shadow-primary/20 hover:border-primary/40 hover:scale-110 active:scale-95 transition-all duration-300 group"
            >
              <ArrowUpDown className="h-4 w-4 text-primary group-hover:rotate-180 transition-transform duration-500" />
            </Button>
          </div>

          {/* Receive Section */}
          <div className="space-y-2">
            <div className="bg-muted/30 rounded-2xl p-4 border border-border/20">
              <div className="flex justify-between items-start mb-1">
                <AmountInput
                  label={t("swap.pair.youReceive")}
                  value={selectedRoute?.expectedAmount ?? toAmount}
                  readOnly
                  placeholder="0.00"
                  className="flex-1"
                  showMax={false}
                />
                <TokenSelector
                  selectedAsset={toToken}
                  onSelect={setToToken}
                  className="mt-6"
                />
              </div>
            </div>
          </div>

          {/* Info Panels (Conditional) */}
          {parseFloat(fromAmount) > 0 && (
            <div className="space-y-3 pt-2 animate-in fade-in slide-in-from-bottom-2 duration-500">
              <PriceInfoPanel
                rate={formattedRate}
                priceImpact={quote.priceImpact}
                minReceived={`${(parseFloat(toAmount || '0') * (1 - slippage / 100)).toFixed(4)} ${toSymbol}`}
                networkFee={quote.fee ? `${quote.fee.toFixed(5)} XLM` : '0.00001 XLM'}
                isLoading={quote.loading}
                onExportJson={() => handleExport("json")}
                onExportCsv={() => handleExport("csv")}
              />
              <RouteDisplay
                amountOut={selectedRoute?.expectedAmount ?? toAmount}
                isLoading={quote.loading}
                onSelect={setSelectedRoute}
              />
            </div>
          )}

          {/* Stale Indicator */}
          {quote.isStale && (
            <span
              data-testid="stale-indicator"
              className="text-xs text-amber-500 font-medium"
            >
              {t("swap.card.outdated")}
            </span>
          )}
          {quote.isRecovering && (
            <div
              data-testid="recovering-indicator"
              className="flex items-center justify-between gap-3 rounded-xl border border-blue-500/20 bg-blue-500/5 px-3 py-2"
            >
              <span className="text-xs text-blue-500 font-medium">
                {quote.hasPendingRetry
                  ? t("swap.card.recoveringQuoteCountdown", {
                      seconds: Math.max(1, Math.ceil(quote.pendingRetryRemainingMs / 1000)),
                    })
                  : t("swap.card.recoveringQuote")}
              </span>
              {quote.hasPendingRetry && (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={quote.cancelRetry}
                  className="h-7 rounded-lg px-2 text-[11px] font-semibold text-blue-600 hover:bg-blue-500/10 hover:text-blue-700"
                >
                  {t("swap.card.cancelRetry")}
                </Button>
              )}
            </div>
          )}

          {requiresFreshQuote && (
            <span
              data-testid="recovery-refresh-indicator"
              className="text-xs font-medium text-primary"
            >
              {t("swap.card.sessionRestored")}
            </span>
          )}

          {/* Action Button */}
          <div className="pt-2">
            <SwapButton
              state={buttonState}
              onSwap={handleSwap}
              onConnectWallet={() => setIsConnected(true)}
              isLoading={quote.loading}
            />
          </div>
          
          {/* Status/Error Messages */}
          {quote.error && (
            <p className="text-center text-xs font-medium text-destructive animate-pulse">
              {quote.error.message}
            </p>
          )}
        </CardContent>
      </Card>
      
      {/* High Impact Confirmation Modal */}
      <HighImpactConfirmModal
        isOpen={isConfirmModalOpen}
        onClose={() => setIsConfirmModalOpen(false)}
        onConfirm={executeSwap}
        priceImpact={quote.priceImpact}
        fromAmount={fromAmount}
        fromSymbol={fromSymbol}
        toAmount={toAmount}
        toSymbol={toSymbol}
      />

      <SessionRecoveryModal
        isOpen={recoveryReason !== null}
        reason={recoveryReason ?? 'refresh'}
        snapshot={recoveryReason === 'refresh' ? pendingRecovery : wakeSnapshot}
        isRecovering={isRecoveringSession}
        onRestore={handleRestoreRecovery}
        onDiscard={handleDiscardRecovery}
      />

      {/* Footer Info */}
      <p className="text-center text-[10px] text-muted-foreground/60 mt-4 px-8 uppercase tracking-widest font-bold">
        {t("swap.card.poweredBy")}
      </p>

      <Dialog open={shortcutHelpOpen} onOpenChange={handleShortcutOpenChange}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("swap.shortcuts.title")}</DialogTitle>
          </DialogHeader>
          <ul className="space-y-3 text-sm">
            <li className="flex justify-between"><span>{t("swap.shortcuts.openHelp")}</span><kbd className="font-mono">?</kbd></li>
            <li className="flex justify-between"><span>{t("swap.shortcuts.closeHelp")}</span><kbd className="font-mono">Esc</kbd></li>
            <li className="flex justify-between"><span>{t("swap.shortcuts.refreshQuote")}</span><kbd className="font-mono">Alt+R</kbd></li>
            <li className="flex justify-between"><span>{t("swap.shortcuts.focusPayAmount")}</span><kbd className="font-mono">Alt+1</kbd></li>
            <li className="flex justify-between"><span>{t("swap.shortcuts.focusReceiveAmount")}</span><kbd className="font-mono">Alt+2</kbd></li>
          </ul>
        </DialogContent>
      </Dialog>
    </div>
  );
}
