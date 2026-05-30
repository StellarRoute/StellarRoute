'use client';

import { useState, useCallback, useEffect, useMemo, useRef } from 'react';
import { useReducedMotion } from '@/hooks/useReducedMotion';
import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { ArrowUpDown, RefreshCw } from 'lucide-react';
import { AmountInput } from './AmountInput';
import { TokenSelector } from './TokenSelector';
import { PriceInfoPanel } from './PriceInfoPanel';
import RouteDisplay from './RoutePanelAsync';
import type { AlternativeRoute } from './RouteDisplay';
import { useRoutes } from '@/hooks/useApi';
import { SwapButton, SwapButtonState } from './SwapButton';
import { SettingsPanel } from '../settings/SettingsPanel';
import { HighImpactConfirmModal } from './HighImpactConfirmModal';
import { TransactionConfirmationModal } from './TransactionConfirmationModal';
import { QuoteStreamStatusIndicator } from './QuoteStreamStatusIndicator';
import { SessionRecoveryModal } from './SessionRecoveryModal';
import { useSwapState } from '@/hooks/useSwapState';
import { useOptimisticSwap } from '@/hooks/useOptimisticSwap';
import type { PreSubmitSnapshot } from '@/types/transaction';
import { useOptionalTradingPair } from '@/contexts/TradingPairContext';
import { useExpertSettings } from '@/hooks/useExpertSettings';
import { useBrowserNotifications } from '@/hooks/useBrowserNotifications';
import {
  SESSION_RECOVERY_THRESHOLD_MS,
  type TradeFormSnapshot,
} from '@/hooks/useTradeFormStorage';
import { useOnlineStatus } from '@/hooks/useOnlineStatus';
import { useQuoteStreamStatus } from '@/hooks/useQuoteStreamStatus';
import { useCompactMode } from '@/hooks/useCompactMode';
import { useShareableQuote } from '@/hooks/useShareableQuote';
import { ShareQuoteButton } from './ShareQuoteButton';
import { QuoteCountdownTimer } from './QuoteCountdownTimer';
import { NetworkMismatchBanner } from '@/components/shared/NetworkMismatchBanner';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';
import { useSwapI18n } from '@/lib/swap-i18n';
import { quoteExportToCsv, type QuoteExportPayload } from '@/lib/quote-export';
import { Maximize2, Minimize2 } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

export function SwapCard() {
  const { t } = useSwapI18n();
  const { isCompact, toggleCompact } = useCompactMode();
  const prefersReducedMotion = useReducedMotion();
  const tradingPairContext = useOptionalTradingPair();
  
  // Wrap useSearchParams in try-catch for SSR
  let parseParams: ReturnType<typeof useShareableQuote>['parseParams'] | null =
    null;
  let isSharedQuoteStale = false;
  let refreshSharedQuote:
    | ReturnType<typeof useShareableQuote>['refreshQuote']
    | null = null;

  try {
    const shareableQuote = useShareableQuote();
    parseParams = shareableQuote.parseParams;
    isSharedQuoteStale = shareableQuote.isStale;
    refreshSharedQuote = shareableQuote.refreshQuote;
  } catch (e) {
    // SSR or missing searchParams context
  }

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

  // Initialize from URL parameters on mount
  useEffect(() => {
    if (!parseParams) return;
    
    const urlParams = parseParams();
    if (!urlParams) return;

    // Apply URL parameters to form state
    if (urlParams.from && urlParams.from !== fromToken) {
      setFromToken(urlParams.from);
    }
    if (urlParams.to && urlParams.to !== toToken) {
      setToToken(urlParams.to);
    }
    if (urlParams.amount && urlParams.amount !== fromAmount) {
      setFromAmount(urlParams.amount);
    }
    if (urlParams.slippage && parseFloat(urlParams.slippage) !== slippage) {
      setSlippage(parseFloat(urlParams.slippage));
    }
    if (urlParams.side && urlParams.side !== side) {
      setSide(urlParams.side);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [parseParams]); // Only run on mount when parseParams becomes available

  // Update trading pair context when tokens change
  useEffect(() => {
    if (tradingPairContext && fromToken && toToken) {
      tradingPairContext.setTradingPair(fromToken, toToken);
    }
  }, [fromToken, toToken, tradingPairContext]);
  const {
    expertMode,
    bypassConfirmation,
    extendedRouteDetails,
    updateExpertMode,
    updateBypassConfirmation,
    updateExtendedRouteDetails,
  } = useExpertSettings();

  const {
    browserNotifications,
    permissionState: notificationPermissionState,
    isDisabled: notificationsDisabled,
    enableNotifications: onEnableNotifications,
    disableNotifications: onDisableNotifications,
  } = useBrowserNotifications();

  const [isConnected, setIsConnected] = useState(false);
  const [isConfirmModalOpen, setIsConfirmModalOpen] = useState(false);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [selectedRoute, setSelectedRoute] = useState<AlternativeRoute | null>(
    null
  );
  const [wakeSnapshot, setWakeSnapshot] = useState<TradeFormSnapshot | null>(
    null
  );
  const [wakeRecoveryOpen, setWakeRecoveryOpen] = useState(false);
  const [recoveryRequestedAt, setRecoveryRequestedAt] = useState<number | null>(
    null
  );
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
    recoveryRequestedAt !== null && (quote.loading || quote.isStale);

  // Connection status indicator
  const { isOnline } = useOnlineStatus();
  const { status: streamStatus, mode: streamMode } = useQuoteStreamStatus({
    isRecovering: quote.isRecovering,
    error: quote.error,
    isOnline,
  });

  const optimistic = useOptimisticSwap({
    rollbackTarget: {
      setFromToken,
      setToToken,
      setFromAmount,
      setSlippage,
      setSelectedRoute: (id) =>
        setSelectedRoute(id ? { id, venue: '', expectedAmount: '' } : null),
      refreshQuote: quote.refresh,
    },
    notificationPreference: { enabled: browserNotifications },
  });

  const fromAmountNum = fromAmount ? Number.parseFloat(fromAmount) : undefined;
  const {
    data: routesData,
    loading: routesLoading,
    error: routesError,
  } = useRoutes(fromToken, toToken, fromAmountNum);

  const alternativeRoutes: AlternativeRoute[] | undefined = useMemo(() => {
    if (!routesData?.routes?.length) return undefined;
    return routesData.routes.map((r, i) => ({
      id: `api-route-${i}`,
      venue: r.path?.[0]?.source ?? 'unknown',
      expectedAmount: `≈ ${r.estimated_output}`,
      score: r.score,
      impactBps: r.impact_bps,
      hopCount: r.path?.length ?? 0,
      hops: r.path?.map((hop, j) => ({
        id: `${i}-${j}`,
        fromAsset: hop.from_asset?.asset_code ?? 'native',
        toAsset: hop.to_asset?.asset_code ?? 'native',
        venue: hop.source,
        fee: hop.fee_bps ? `${hop.fee_bps} bps` : '0 bps',
      })),
    }));
  }, [routesData]);

  // Handle background transaction toasts when bypassConfirmation is enabled
  useEffect(() => {
    if (!bypassConfirmation || !isModalOpen) return;

    if (optimistic.status === 'pending') {
      toast.loading('Signing transaction...', { id: 'swap-toast' });
    } else if (optimistic.status === 'submitted') {
      toast.loading('Transaction submitted, awaiting confirmation...', { id: 'swap-toast' });
    } else if (optimistic.status === 'confirmed') {
      toast.success('Swap confirmed successfully!', { id: 'swap-toast' });
      setIsModalOpen(false);
      reset();
      setSelectedRoute(null);
    } else if (optimistic.status === 'failed') {
      toast.error(optimistic.errorMessage || 'Swap failed. Please try again.', { id: 'swap-toast' });
      setIsModalOpen(false);
    } else if (optimistic.status === 'dropped') {
      toast.error('Transaction timed out.', { id: 'swap-toast' });
      setIsModalOpen(false);
    }
  }, [optimistic.status, optimistic.errorMessage, bypassConfirmation, isModalOpen, reset, setSelectedRoute]);

  // Mock balance
  const fromBalance = '100.00';
  const fromSymbol = fromToken === 'native' ? 'XLM' : fromToken.split(':')[0];
  const toSymbol = toToken === 'native' ? 'XLM' : toToken.split(':')[0];

  const buttonState = useMemo<SwapButtonState>(() => {
    if (optimistic.submitLock) return 'executing';
    if (!isConnected) return 'no_wallet';
    if (!fromAmount || parseFloat(fromAmount) === 0) return 'no_amount';
    if (quote.error) return 'error';
    if (requiresFreshQuote) return 'refreshing_quote';
    if (parseFloat(fromAmount) > parseFloat(fromBalance))
      return 'insufficient_balance';
    if (quote.priceImpact > 10) return 'high_impact_warning';
    if (quote.loading) return 'refreshing_quote';
    if (quote.isStale) return 'error';
    return 'ready';
  }, [
    fromAmount,
    fromBalance,
    isConnected,
    optimistic.submitLock,
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
        quote.refresh();
      } else {
        closeRecoveryModal();
        quote.refresh();
      }
    } catch (error) {
      console.error('Failed to restore session:', error);
      throw error; // Let modal handle the error display
    } finally {
      setIsRecoveringSession(false);
    }
  }, [closeRecoveryModal, quote, recoveryReason, restorePending]);

  const handleConfirm = useCallback(() => {
    const snap: PreSubmitSnapshot = {
      fromToken,
      toToken,
      fromAmount,
      slippage,
      selectedRouteId: selectedRoute?.id ?? null,
    };
    setIsModalOpen(true);
    optimistic.initiateSwap({
      fromAsset: fromToken,
      fromAmount,
      toAsset: toToken,
      toAmount: selectedRoute?.expectedAmount ?? toAmount,
      exchangeRate: formattedRate,
      priceImpact: quote.priceImpact.toString(),
      minReceived: `${(parseFloat(toAmount || '0') * (1 - slippage / 100)).toFixed(4)} ${toSymbol}`,
      networkFee: quote.fee ? `${quote.fee.toFixed(5)} XLM` : '0.00001 XLM',
      routePath: [],
      walletAddress: 'mock_wallet_address',
      snapshot: snap,
    });
  }, [
    fromToken,
    toToken,
    fromAmount,
    slippage,
    selectedRoute,
    toAmount,
    formattedRate,
    quote,
    toSymbol,
    optimistic,
  ]);

  const handleSwap = useCallback(() => {
    if (quote.priceImpact > 5) {
      setIsConfirmModalOpen(true);
      return;
    }
    handleConfirm();
  }, [quote.priceImpact, handleConfirm]);

  const handleMax = useCallback(() => {
    setFromAmount(fromBalance);
  }, [fromBalance, setFromAmount]);

  const handlePresetSelect = useCallback(
    (percentage: number) => {
      const balanceNum = parseFloat(fromBalance);
      if (isNaN(balanceNum) || balanceNum === 0) return;

      const amount = balanceNum * percentage;
      // Round to 7 decimals to respect asset precision
      const rounded = Math.floor(amount * 10000000) / 10000000;
      setFromAmount(rounded.toString());
    },
    [fromBalance, setFromAmount]
  );

  const handleSwitchTokens = useCallback(() => {
    setSelectedRoute(null);
    switchTokens();
  }, [switchTokens]);

  useEffect(() => {
    const onKeydown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const isEditable = target
        ? target.tagName === 'INPUT' ||
          target.tagName === 'TEXTAREA' ||
          target.isContentEditable
        : false;

      if (event.key === '?' && !isEditable) {
        event.preventDefault();
        lastFocusedElementRef.current = document.activeElement as HTMLElement;
        setShortcutHelpOpen(true);
      }

      if (event.key.toLowerCase() === 'r' && event.altKey) {
        event.preventDefault();
        quote.refresh();
      }

      if (event.key === '1' && event.altKey) {
        event.preventDefault();
        document
          .querySelectorAll<HTMLInputElement>('input[placeholder="0.00"]')[0]
          ?.focus();
      }

      if (event.key === '2' && event.altKey) {
        event.preventDefault();
        document
          .querySelectorAll<HTMLInputElement>('input[placeholder="0.00"]')[1]
          ?.focus();
      }

      if (event.key.toLowerCase() === 'f' && event.shiftKey && !isEditable) {
        event.preventDefault();
        handleSwitchTokens();
      }

      if (event.key.toLowerCase() === 'm' && event.shiftKey && !isEditable) {
        event.preventDefault();
        handleMax();
      }
    };

    window.addEventListener('keydown', onKeydown);
    return () => window.removeEventListener('keydown', onKeydown);
  }, [quote, handleSwitchTokens, handleMax]);

  const handleShortcutOpenChange = useCallback((open: boolean) => {
    setShortcutHelpOpen(open);
    if (!open) {
      lastFocusedElementRef.current?.focus();
    }
  }, []);

  const handleExport = useCallback(
    (format: 'json' | 'csv') => {
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
          selectedVenue: selectedRoute?.venue ?? 'auto',
          routeSummary:
            selectedRoute?.hops
              ?.map((hop) => `${hop.fromAsset}->${hop.toAsset}`)
              .join(' | ') ?? 'best-route',
        },
      };
      const serialized =
        format === 'json'
          ? JSON.stringify(payload, null, 2)
          : quoteExportToCsv(payload);
      const blob = new Blob([serialized], {
        type: format === 'json' ? 'application/json' : 'text/csv;charset=utf-8',
      });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = `stellarroute-quote-summary.${format}`;
      anchor.click();
      URL.revokeObjectURL(url);
      toast.success(
        t('swap.quote.exportSuccess', { format: format.toUpperCase() })
      );
    },
    [
      formattedRate,
      fromAmount,
      fromSymbol,
      quote.fee,
      quote.priceImpact,
      selectedRoute,
      slippage,
      t,
      toAmount,
      toSymbol,
    ]
  );

  return (
    <div
      data-testid="swap-card"
      className="w-full max-w-[480px] mx-auto perspective-1000"
    >
      {/* Network Mismatch Banner */}
      <NetworkMismatchBanner className="mb-4" />

      {/* Shared Quote Stale Warning */}
      {isSharedQuoteStale && refreshSharedQuote && (
        <div className="mb-4 p-3 rounded-xl border border-amber-500/50 bg-amber-500/10">
          <p className="text-xs text-amber-800 dark:text-amber-200 mb-2">
            This shared quote is outdated. Refresh to get current pricing.
          </p>
          <Button
            size="sm"
            variant="outline"
            onClick={refreshSharedQuote}
            className="h-7 text-xs"
          >
            Refresh Quote
          </Button>
        </div>
      )}

      <Card
        className={cn(
          'relative overflow-hidden border-border/40 bg-background/60 backdrop-blur-xl shadow-2xl rounded-[32px]',
          !prefersReducedMotion && 'transition-all duration-500 hover:shadow-primary/5',
          isCompact && 'rounded-2xl',
          expertMode && 'border-amber-500/30 hover:shadow-amber-500/10 shadow-amber-500/5'
        )}
      >
        {/* Animated Background Gradients */}
        <div className={cn(
          'absolute -top-24 -left-24 w-48 h-48 bg-primary/10 rounded-full blur-3xl',
          !prefersReducedMotion && 'animate-pulse'
        )} />
        <div className={cn(
          'absolute -bottom-24 -right-24 w-48 h-48 bg-blue-500/10 rounded-full blur-3xl',
          !prefersReducedMotion && 'animate-pulse delay-700'
        )} />

        <CardContent className={cn('space-y-4', isCompact ? 'p-4' : 'p-6')}>
          {/* Header */}
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-1.5">
              <h2
                className={cn(
                  'font-bold tracking-tight bg-gradient-to-br from-foreground to-foreground/60 bg-clip-text text-transparent',
                  isCompact ? 'text-lg' : 'text-xl'
                )}
              >
                Swap
              </h2>
              {expertMode && (
                <span className={cn(
                  'text-[10px] font-bold uppercase tracking-wider text-amber-500 bg-amber-500/10 px-2 py-0.5 rounded-full border border-amber-500/20',
                  !prefersReducedMotion && 'animate-pulse'
                )}>
                  Expert
                </span>
              )}
            </div>
            <div className="flex items-center gap-1">
              <QuoteStreamStatusIndicator
                status={streamStatus}
                mode={streamMode}
              />
              <Button
                variant="ghost"
                size="icon"
                onClick={toggleCompact}
                aria-label={isCompact ? 'Expand layout' : 'Compact layout'}
                className="h-9 w-9 rounded-xl hover:bg-muted/80"
              >
                {isCompact ? (
                  <Maximize2 className="h-4.5 w-4.5 text-muted-foreground" />
                ) : (
                  <Minimize2 className="h-4.5 w-4.5 text-muted-foreground" />
                )}
              </Button>
              <SettingsPanel
                slippage={slippage}
                onSlippageChange={setSlippage}
                deadline={deadline}
                onDeadlineChange={setDeadline}
                expertMode={expertMode}
                bypassConfirmation={bypassConfirmation}
                extendedRouteDetails={extendedRouteDetails}
                onExpertModeChange={updateExpertMode}
                onBypassConfirmationChange={updateBypassConfirmation}
                onExtendedRouteDetailsChange={updateExtendedRouteDetails}
                onReset={() => {
                  reset();
                  setSelectedRoute(null);
                }}
                browserNotifications={browserNotifications}
                notificationPermissionState={notificationPermissionState}
                notificationsDisabled={notificationsDisabled}
                onEnableNotifications={onEnableNotifications}
                onDisableNotifications={onDisableNotifications}
              />
              <Button
                variant="ghost"
                size="icon"
                onClick={() => quote.refresh()}
                disabled={quote.loading}
                aria-label={t('swap.card.refreshQuote')}
                className="h-9 w-9 rounded-xl hover:bg-muted/80"
              >
                <RefreshCw
                  className={cn(
                    'h-4.5 w-4.5 text-muted-foreground',
                    quote.loading && 'animate-spin'
                  )}
                />
              </Button>
            </div>
          </div>

          {/* Pay Section */}
          <div className={cn('space-y-2 group', isCompact && 'space-y-1')}>
            <div
              className={cn(
                'bg-muted/30 hover:bg-muted/40 transition-colors rounded-2xl border border-border/20 focus-within:border-primary/30 focus-within:ring-4 focus-within:ring-primary/5',
                isCompact ? 'p-3 rounded-xl' : 'p-4'
              )}
            >
              <div className="flex justify-between items-start mb-1">
                <AmountInput
                  label={t('swap.pair.youPay')}
                  value={side === 'sell' ? fromAmount : (selectedRoute?.expectedAmount ?? toAmount)}
                  onChange={(val) => {
                    if (side !== 'sell') setSide('sell');
                    setFromAmount(val);
                  }}
                  onMax={handleMax}
                  onPresetSelect={handlePresetSelect}
                  balance={`${fromBalance} ${fromSymbol}`}
                  showPresets={isConnected}
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
              className={cn(
                'absolute h-10 w-10 rounded-xl bg-background border-border/40 shadow-lg hover:shadow-primary/20 hover:border-primary/40 active:scale-95 group',
                prefersReducedMotion
                  ? 'transition-colors'
                  : 'hover:scale-110 transition-all duration-300'
              )}
            >
              <ArrowUpDown className={cn(
                'h-4 w-4 text-primary',
                !prefersReducedMotion && 'group-hover:rotate-180 transition-transform duration-500'
              )} />
            </Button>
          </div>

          {/* Receive Section */}
          <div className={cn('space-y-2', isCompact && 'space-y-1')}>
            <div
              className={cn(
                'bg-muted/30 rounded-2xl border border-border/20',
                isCompact ? 'p-3 rounded-xl' : 'p-4'
              )}
            >
              <div className="flex justify-between items-start mb-1">
                <AmountInput
                  label={t('swap.pair.youReceive')}
                  value={side === 'buy' ? fromAmount : (selectedRoute?.expectedAmount ?? toAmount)}
                  onChange={(val) => {
                    if (side !== 'buy') setSide('buy');
                    setFromAmount(val);
                  }}
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
            <div
              className={cn(
                'space-y-3',
                isCompact ? 'space-y-2 pt-1' : 'pt-2',
                !prefersReducedMotion && 'animate-in fade-in slide-in-from-bottom-2 duration-500'
              )}
            >
              <PriceInfoPanel
                rate={formattedRate}
                priceImpact={quote.priceImpact}
                midpoint={quote.data?.midpoint}
                spreadBps={quote.data?.spread_bps}
                minReceived={`${(parseFloat(toAmount || '0') * (1 - slippage / 100)).toFixed(4)} ${toSymbol}`}
                networkFee={
                  quote.fee ? `${quote.fee.toFixed(5)} XLM` : '0.00001 XLM'
                }
                isLoading={quote.loading}
                onExportJson={() => handleExport('json')}
                onExportCsv={() => handleExport('csv')}
              />
              
              <QuoteCountdownTimer
                expiresAtMs={quote.expiresAtMs}
                ttlSeconds={quote.ttlSeconds}
                onRefresh={() => quote.refresh({ force: true })}
                isLoading={quote.loading}
                className="px-1"
              />

              <RouteDisplay
                amountOut={selectedRoute?.expectedAmount ?? toAmount}
                isLoading={quote.loading}
                onSelect={setSelectedRoute}
                extendedRouteDetails={extendedRouteDetails}
                alternativeRoutes={alternativeRoutes}
                isRoutesLoading={routesLoading}
                routesError={routesError?.message ?? null}
              />
              {/* Share Quote Button */}
              <div className="flex justify-end">
                <ShareQuoteButton
                  params={{
                    from: fromToken,
                    to: toToken,
                    amount: fromAmount,
                    slippage: slippage.toString(),
                    side: side,
                  }}
                  disabled={!fromAmount || parseFloat(fromAmount) === 0}
                />
              </div>
            </div>
          )}

          {/* Stale Indicator */}
          {quote.isStale && (
            <span
              data-testid="stale-indicator"
              className="text-xs text-amber-500 font-medium"
            >
              {t('swap.card.outdated')}
            </span>
          )}
          {quote.isRecovering && (
            <div
              data-testid="recovering-indicator"
              className="flex items-center justify-between gap-3 rounded-xl border border-blue-500/20 bg-blue-500/5 px-3 py-2"
            >
              <span className="text-xs text-blue-500 font-medium">
                {quote.hasPendingRetry
                  ? t('swap.card.recoveringQuoteCountdown', {
                      seconds: Math.max(
                        1,
                        Math.ceil(quote.pendingRetryRemainingMs / 1000)
                      ),
                    })
                  : t('swap.card.recoveringQuote')}
              </span>
              {quote.hasPendingRetry && (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={quote.cancelRetry}
                  className="h-7 rounded-lg px-2 text-[11px] font-semibold text-blue-600 hover:bg-blue-500/10 hover:text-blue-700"
                >
                  {t('swap.card.cancelRetry')}
                </Button>
              )}
            </div>
          )}

          {requiresFreshQuote && (
            <span
              data-testid="recovery-refresh-indicator"
              className="text-xs font-medium text-primary"
            >
              {t('swap.card.sessionRestored')}
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
            <p className={cn(
              'text-center text-xs font-medium text-destructive',
              !prefersReducedMotion && 'animate-pulse'
            )}>
              {quote.error.message}
            </p>
          )}
        </CardContent>
      </Card>

      {/* High Impact Confirmation Modal — separate purpose: warns before the review step */}
      <HighImpactConfirmModal
        isOpen={isConfirmModalOpen}
        onClose={() => setIsConfirmModalOpen(false)}
        onConfirm={() => {
          setIsConfirmModalOpen(false);
          handleConfirm();
        }}
        priceImpact={quote.priceImpact}
        fromAmount={fromAmount}
        fromSymbol={fromSymbol}
        toAmount={toAmount}
        toSymbol={toSymbol}
      />

      {!bypassConfirmation && (
        <TransactionConfirmationModal
          isOpen={isModalOpen}
          status={optimistic.status}
          txHash={optimistic.txHash}
          errorMessage={optimistic.errorMessage}
          tradeParams={optimistic.tradeParams}
          onConfirm={() => {}}
          onCancel={() => {
            optimistic.cancel();
            setIsModalOpen(false);
          }}
          onTryAgain={() => {
            optimistic.tryAgain();
          }}
          onResubmit={() => {
            optimistic.resubmit();
          }}
          onDismiss={() => {
            optimistic.dismiss();
            setIsModalOpen(false);
          }}
          onDone={() => {
            optimistic.dismiss();
            setIsModalOpen(false);
            reset();
            setSelectedRoute(null);
          }}
        />
      )}

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
        {t('swap.card.poweredBy')}
      </p>

      <Dialog open={shortcutHelpOpen} onOpenChange={handleShortcutOpenChange}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('swap.shortcuts.title')}</DialogTitle>
          </DialogHeader>
          <ul className="space-y-3 text-sm">
            <li className="flex justify-between">
              <span>{t('swap.shortcuts.openHelp')}</span>
              <kbd className="font-mono">?</kbd>
            </li>
            <li className="flex justify-between">
              <span>{t('swap.shortcuts.closeHelp')}</span>
              <kbd className="font-mono">Esc</kbd>
            </li>
            <li className="flex justify-between">
              <span>{t('swap.shortcuts.refreshQuote')}</span>
              <kbd className="font-mono">Alt+R</kbd>
            </li>
            <li className="flex justify-between">
              <span>{t('swap.shortcuts.focusPayAmount')}</span>
              <kbd className="font-mono">Alt+1</kbd>
            </li>
            <li className="flex justify-between">
              <span>{t('swap.shortcuts.focusReceiveAmount')}</span>
              <kbd className="font-mono">Alt+2</kbd>
            </li>
            <li className="flex justify-between">
              <span>{t('swap.shortcuts.flipPair')}</span>
              <kbd className="font-mono">Shift+F</kbd>
            </li>
            <li className="flex justify-between">
              <span>{t('swap.shortcuts.maxAmount')}</span>
              <kbd className="font-mono">Shift+M</kbd>
            </li>
          </ul>
        </DialogContent>
      </Dialog>
    </div>
  );
}
