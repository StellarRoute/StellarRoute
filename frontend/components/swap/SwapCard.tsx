'use client';

import { useState, useCallback, useEffect, useMemo, useRef } from 'react';
import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { ArrowUpDown, RefreshCw, Stethoscope } from 'lucide-react';
import { AmountInput } from './AmountInput';
import { TokenSelector } from './TokenSelector';
import { PriceInfoPanel } from './PriceInfoPanel';
import type { AlternativeRoute } from './RouteDisplay';
import RouteDisplay from './RoutePanelAsync';
import { MobileRouteBottomSheet } from './MobileRouteBottomSheet';
import { BatchSwapPreview, type BatchSwapLeg } from './BatchSwapPreview';
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
import { useWalletBalance } from '@/hooks/useWalletBalance';
import {
  DEFAULT_DEADLINE,
  DEFAULT_SLIPPAGE,
  SESSION_RECOVERY_THRESHOLD_MS,
  type TradeFormSnapshot,
} from '@/hooks/useTradeFormStorage';
import { useBatchQuote } from '@/hooks/useApi';
import { useFeatureFlag } from '@/hooks/useFeatureFlag';
import type { QuoteRequestItem } from '@/lib/api/client';
import { useOnlineStatus } from '@/hooks/useOnlineStatus';
import { useQuoteStreamStatus } from '@/hooks/useQuoteStreamStatus';
import { useCompactMode } from '@/hooks/useCompactMode';
import { useShareableQuote } from '@/hooks/useShareableQuote';
import { ShareQuoteButton } from './ShareQuoteButton';
import { NetworkMismatchBanner } from '@/components/shared/NetworkMismatchBanner';
import { DiagnosticsPanel } from '@/components/shared/DiagnosticsPanel';
import { useWallet } from '@/components/providers/wallet-provider';
import { signTransactionWithWallet } from '@/lib/wallet';
import { submitToHorizon, getNetworkPassphrase, getHorizonUrl } from '@/lib/wallet/submit';
import { buildPathPaymentXdr } from '@/lib/wallet/xdr-builder';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';
import { useSwapI18n } from '@/lib/swap-i18n';
import { useRoutes } from '@/hooks/useApi';
import { emitRouteEvent } from '@/lib/telemetry';
import { SwapWarningCenter, type SwapWarning } from './SwapWarningCenter';
import { quoteExportToCsv, type QuoteExportPayload } from '@/lib/quote-export';
import { getTraderErrorCopy, toTraderErrorLine } from '@/lib/api/trader-error-copy';
import { Maximize2, Minimize2 } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { IconographyLegend } from '@/components/shared/IconographyLegend';
import {
  getSwapCardStoryPresentation,
  type SwapCardStoryFixture,
} from './swapCardStory';

export interface SwapCardProps {
  /** Shows alternative route picker when routes beta is enabled. */
  showRoutePicker?: boolean;
  /** Ladle story fixture — drives deterministic UI states for visual review. */
  storyFixture?: SwapCardStoryFixture;
}

export function SwapCard({ storyFixture, showRoutePicker = false }: SwapCardProps = {}) {
  const storyPresentation = storyFixture
    ? getSwapCardStoryPresentation(storyFixture)
    : null;
  const { t } = useSwapI18n();
  const { isCompact, toggleCompact } = useCompactMode();
  const tradingPairContext = useOptionalTradingPair();
  const { enabled: realXdrEnabled } = useFeatureFlag('real_xdr');

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
  } catch {
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
    side,
    setSide,
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

  // Fetch ranked routes from /api/v1/routes
  const routesState = useRoutes(
    fromToken,
    toToken,
    parseFloat(fromAmount) || undefined
  );

  // Merge quote alternatives and routes endpoint candidates
  const mergedAlternativeRoutes = useMemo(() => {
    const list: AlternativeRoute[] = [];

    // 1. Add any alternative routes embedded in the quote
    if (quote.data?.alternativeRoutes) {
      quote.data.alternativeRoutes.forEach((alt) => {
        list.push({
          id: alt.id,
          venue: alt.venue,
          expectedAmount: alt.expectedAmount.startsWith('≈') ? alt.expectedAmount : `≈ ${alt.expectedAmount}`,
          hops: [],
        });
      });
    }

    // 2. Add routes from the /api/v1/routes endpoint
    if (routesState.data?.routes) {
      routesState.data.routes.forEach((candidate, index) => {
        const hopVenues = candidate.path.map((hop) => {
          const source = hop.source;
          if (source === 'sdex') return 'SDEX';
          if (source.startsWith('amm:')) {
            const name = source.substring(4);
            if (name.toLowerCase() === 'aqua') return 'AQUA Pool';
            if (name.toLowerCase() === 'phoenix') return 'Phoenix AMM';
            if (name.toLowerCase() === 'blend') return 'Blend Pool';
            return name.charAt(0).toUpperCase() + name.slice(1);
          }
          return source;
        });
        const uniqueHopVenues = hopVenues.filter((v, i) => i === 0 || v !== hopVenues[i - 1]);
        const venueName = uniqueHopVenues.join(' + ');

        const hops = candidate.path.map((hop, hopIndex) => {
          const fromSymbol = hop.from_asset.asset_type === 'native' ? 'XLM' : (hop.from_asset.asset_code || 'UNK');
          const toSymbol = hop.to_asset.asset_type === 'native' ? 'XLM' : (hop.to_asset.asset_code || 'UNK');
          
          let sourceName = hop.source;
          if (sourceName === 'sdex') sourceName = 'SDEX';
          else if (sourceName.startsWith('amm:')) {
            const name = sourceName.substring(4);
            if (name.toLowerCase() === 'aqua') sourceName = 'AQUA Pool';
            else if (name.toLowerCase() === 'phoenix') sourceName = 'Phoenix AMM';
            else if (name.toLowerCase() === 'blend') sourceName = 'Blend Pool';
            else sourceName = name.charAt(0).toUpperCase() + sourceName.slice(1);
          }

          const feeXLM = ((hop.fee_bps || 30) / 100000).toFixed(5) + ' XLM';

          return {
            id: `candidate-${index}-hop-${hopIndex}`,
            fromAsset: fromSymbol,
            toAsset: toSymbol,
            venue: sourceName,
            fee: feeXLM,
          };
        });

        // Avoid adding duplicates of the same venue name
        const isDuplicate = list.some((item) => item.venue === venueName);
        if (!isDuplicate) {
          list.push({
            id: `route-api-${index}`,
            venue: venueName,
            expectedAmount: `≈ ${parseFloat(candidate.estimated_output).toFixed(4)}`,
            hops,
            rawPath: candidate.path,
            priceImpact: candidate.impact_bps / 100,
          });
        }
      });
    }

    return list;
  }, [quote.data, routesState.data]);

  const [selectedRoute, setSelectedRoute] = useState<AlternativeRoute | null>(
    null
  );

  const handleRouteSelect = useCallback((route: AlternativeRoute) => {
    setSelectedRoute(route);
    // Trigger re-quote
    quote.refresh();
    
    const hopCount = route.rawPath ? route.rawPath.length : (quote.data?.path.length ?? 1);
    emitRouteEvent(route.venue, hopCount);
  }, [quote]);

  const isRoutesLoading = quote.loading || routesState.loading;

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
  const { enabled: batchSwapsEnabled } = useFeatureFlag('batch_swaps');
  const batchRequests = useMemo<QuoteRequestItem[]>(() => {
    const amount = Number.parseFloat(fromAmount);
    if (
      !batchSwapsEnabled ||
      !Number.isFinite(amount) ||
      amount <= 0 ||
      !fromToken ||
      !toToken ||
      fromToken === toToken
    ) {
      return [];
    }

    const firstLegAmount = Number((amount / 2).toFixed(7));
    const secondLegAmount = Number((amount - firstLegAmount).toFixed(7));
    return [firstLegAmount, secondLegAmount]
      .filter((legAmount) => legAmount > 0)
      .map((legAmount) => ({
        base: fromToken,
        quote: toToken,
        amount: legAmount,
        quote_type: 'sell',
      }));
  }, [batchSwapsEnabled, fromAmount, fromToken, toToken]);
  const batchQuote = useBatchQuote(
    batchRequests,
    !batchSwapsEnabled || batchRequests.length === 0
  );
  const batchLegs = useMemo<BatchSwapLeg[]>(
    () =>
      batchQuote.data?.quotes.map((legQuote, index) => ({
        id: `batch-leg-${index}`,
        fromAsset:
          legQuote.base_asset.asset_code ??
          (legQuote.base_asset.asset_type === 'native'
            ? 'XLM'
            : fromToken.split(':')[0]),
        toAsset:
          legQuote.quote_asset.asset_code ??
          (legQuote.quote_asset.asset_type === 'native'
            ? 'XLM'
            : toToken.split(':')[0]),
        fromAmount: legQuote.amount,
        toAmount: legQuote.total,
        price: legQuote.price,
        priceImpact: legQuote.price_impact ?? legQuote.priceImpact,
      })) ?? [],
    [batchQuote.data, fromToken, toToken]
  );

  const {
    address: walletAddress,
    isConnected,
    walletId,
    network: walletAppNetwork,
    networkMismatch,
    connect,
  } = useWallet();

  // Fetch real wallet balance for the selected from-asset
  const balanceState = useWalletBalance({
    address: walletAddress,
    asset: fromToken,
    isConnected,
    network: walletAppNetwork,
  });

  // --- Issue #506: Memo State Management ---
  const [showMemoField, setShowMemoField] = useState(false);
  const [memoType, setMemoType] = useState<'text' | 'hash'>('text');
  const [memoValue, setMemoValue] = useState('');
  const memoError = useMemo(() => {
    if (!memoValue) return null;
    if (memoType === 'text') {
      const byteLength = new TextEncoder().encode(memoValue).length;
      if (byteLength > 28) {
        return `Memo text exceeds 28 bytes (currently ${byteLength} bytes)`;
      }
    }
    if (memoType === 'hash') {
      const hexRegex = /^[0-9a-fA-F]{64}$/;
      if (!hexRegex.test(memoValue)) {
        return 'Hash memo must be exactly 64 hexadecimal characters';
      }
    }
    return null;
  }, [memoValue, memoType]);
  const [isConfirmModalOpen, setIsConfirmModalOpen] = useState(false);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [wakeSnapshot, setWakeSnapshot] = useState<TradeFormSnapshot | null>(
    null
  );
  const [wakeRecoveryOpen, setWakeRecoveryOpen] = useState(false);
  const [recoveryRequestedAt, setRecoveryRequestedAt] = useState<number | null>(
    null
  );
  const [isRecoveringSession, setIsRecoveringSession] = useState(false);
  const [shortcutHelpOpen, setShortcutHelpOpen] = useState(false);
  const [diagnosticsOpen, setDiagnosticsOpen] = useState(false);
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

  // --- Issue #745: Swap Warning Center Logic ---
  const [warnings, setWarnings] = useState<SwapWarning[]>([]);
  const [dismissedWarningIds, setDismissedWarningIds] = useState<Set<string>>(
    new Set()
  );

  const handleRemoveWarning = useCallback((id: string) => {
    setDismissedWarningIds((prev) => {
      const next = new Set(prev);
      next.add(id);
      return next;
    });
  }, []);

  useEffect(() => {
    const checkWarnings = () => {
      const list: SwapWarning[] = [];

      // 1. Low slippage (<0.5%) warn (warning level, dismissible)
      if (slippage > 0 && slippage < 0.5) {
        const id = 'low_slippage';
        if (!dismissedWarningIds.has(id)) {
          list.push({
            id,
            type: 'warning',
            title: 'Low Slippage Tolerance',
            message: `Your transaction may fail if the price moves unfavorably by more than the set limit of ${slippage}%.`,
            timestamp: Date.now(),
            dismissible: true,
          });
        }
      }

      // 2. High slippage (>5.0%) warn (error level, not dismissible)
      if (slippage > 5.0) {
        const id = 'high_slippage';
        list.push({
          id,
          type: 'error',
          title: 'High Slippage Risk',
          message:
            'High slippage increases the risk of frontrunning and getting a significantly worse price.',
          timestamp: Date.now(),
          dismissible: false,
        });
      }

      // 3. Stale quote warning when timestamp exceeds 60s
      if (quote.lastQuotedAtMs && Date.now() - quote.lastQuotedAtMs > 60000) {
        const id = 'stale_quote';
        list.push({
          id,
          type: 'warning',
          title: 'Stale Quote',
          message:
            'This quote is more than 60 seconds old. Please refresh for accurate pricing.',
          timestamp: Date.now(),
          dismissible: false,
        });
      }

      // 4. Quote error response from API
      if (quote.error) {
        const copy = getTraderErrorCopy(quote.error);
        const id = `quote_error_${quote.error.message || 'unknown'}`;
        if (!dismissedWarningIds.has(id)) {
          list.push({
            id,
            type: 'error',
            title: copy.headline,
            message: `${copy.explanation} ${copy.recoveryAction}`,
            timestamp: Date.now(),
            dismissible: true,
          });
        }
      }

      setWarnings((prev) => {
        const prevIds = prev.map((w) => w.id).join(',');
        const currIds = list.map((w) => w.id).join(',');
        if (prevIds === currIds) return prev;
        return list;
      });
    };

    checkWarnings();
    const interval = setInterval(checkWarnings, 1000);
    return () => clearInterval(interval);
  }, [slippage, quote.lastQuotedAtMs, quote.error, dismissedWarningIds]);

  // Connection status indicator
  const { isOnline } = useOnlineStatus();
  const { status: streamStatus, mode: streamMode } = useQuoteStreamStatus({
    isRecovering: quote.isRecovering,
    error: quote.error,
    isOnline,
  });

  const optimistic = useOptimisticSwap({
    signTransaction: walletId
      ? (xdr) =>
          signTransactionWithWallet(
            xdr,
            walletId,
            getNetworkPassphrase(walletAppNetwork)
          )
      : undefined,
    submitTransaction: (signedXdr) =>
      submitToHorizon(signedXdr, walletAppNetwork),
    // Build real Stellar path-payment XDR when the integration flag is enabled.
    // Falls back to "mock_xdr" stub when flag is off (default during development).
    buildXdr: realXdrEnabled && walletAddress
      ? (params) =>
          buildPathPaymentXdr({
            walletAddress: params.walletAddress || walletAddress,
            fromAsset: params.fromAsset,
            fromAmount: params.fromAmount,
            toAsset: params.toAsset,
            minReceived: params.minReceived,
            routePath: params.routePath,
            networkPassphrase: getNetworkPassphrase(walletAppNetwork),
            horizonUrl: getHorizonUrl(walletAppNetwork),
          })
      : undefined,
    rollbackTarget: {
      setFromToken,
      setToToken,
      setFromAmount,
      setSlippage,
      setSelectedRoute: (id) =>
        setSelectedRoute(id ? { id, venue: '', expectedAmount: '' } : null),
      refreshQuote: quote.refresh,
    },
  });

  // Handle background transaction toasts when bypassConfirmation is enabled
  useEffect(() => {
    if (!bypassConfirmation || !isModalOpen) return;

    if (optimistic.status === 'pending') {
      toast.loading('Signing transaction...', { id: 'swap-toast' });
    } else if (optimistic.status === 'submitted') {
      toast.loading('Transaction submitted, awaiting confirmation...', {
        id: 'swap-toast',
      });
    } else if (optimistic.status === 'confirmed') {
      toast.success('Swap confirmed successfully!', { id: 'swap-toast' });
      setIsModalOpen(false);
      reset();
      setSelectedRoute(null);
    } else if (optimistic.status === 'failed') {
      const errorObj = optimistic.errorMessage ? new Error(optimistic.errorMessage) : new Error('Unknown error');
      const copy = getTraderErrorCopy(errorObj);
      toast.error(toTraderErrorLine(copy), {
        id: 'swap-toast',
      });
      setIsModalOpen(false);
    } else if (optimistic.status === 'dropped') {
      toast.error('Transaction timed out.', { id: 'swap-toast' });
      setIsModalOpen(false);
    }
  }, [
    optimistic.status,
    optimistic.errorMessage,
    bypassConfirmation,
    isModalOpen,
    reset,
    setSelectedRoute,
  ]);

  // Replace hardcoded balance with real wallet balance
  const fromBalance = balanceState.balance ?? '0';
  const fromSymbol = fromToken === 'native' ? 'XLM' : fromToken.split(':')[0];
  const toSymbol = toToken === 'native' ? 'XLM' : toToken.split(':')[0];

  const buttonState = useMemo<SwapButtonState>(() => {
    if (optimistic.submitLock) return 'executing';
    if (!isConnected) return 'no_wallet';
    if (networkMismatch) return 'no_wallet'; // Swap disabled while network mismatch
    if (memoError) return 'error'; // Block swap if there is a memo validation error
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
    networkMismatch,
    optimistic.submitLock,
    quote.error,
    quote.isStale,
    quote.loading,
    quote.priceImpact,
    requiresFreshQuote,
    memoError,
  ]);

  const displayButtonState = storyPresentation?.buttonState ?? buttonState;
  const displayQuoteLoading = storyPresentation?.quoteLoading ?? quote.loading;
  const displayQuoteStale = storyPresentation?.quoteStale ?? quote.isStale;
  const displayQuoteError = storyPresentation?.quoteError ?? quote.error;
  const displayQuotePriceImpact =
    storyPresentation?.quotePriceImpact ?? quote.priceImpact;
  const displayToAmount = storyPresentation?.toAmount ?? toAmount;
  const displayFormattedRate =
    storyPresentation?.formattedRate ?? formattedRate;
  const displayIsModalOpen = storyPresentation?.confirmModalOpen ?? isModalOpen;
  const displayOptimisticStatus =
    storyPresentation?.optimisticStatus ?? optimistic.status;
  const displayTradeParams =
    storyPresentation?.tradeParams ?? optimistic.tradeParams;

  useEffect(() => {
    if (!storyPresentation?.seedFromAmount) return;
    setFromAmount(storyPresentation.seedFromAmount);
  }, [storyPresentation?.seedFromAmount, setFromAmount]);

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

  // Handle "Swap Again" action: close modal but keep form state intact
  const handleSwapAgain = useCallback(() => {
    setIsModalOpen(false);
    // keep tokens/amounts as-is so user can quickly modify and swap again
  }, []);

  const handleConfirm = useCallback(() => {
    const snap: PreSubmitSnapshot = {
      fromToken,
      toToken,
      fromAmount,
      slippage,
      selectedRouteId: selectedRoute?.id ?? null,
    };
    setIsModalOpen(true);
    const finalToAmount = selectedRoute?.expectedAmount
      ? selectedRoute.expectedAmount.replace('≈ ', '')
      : toAmount;
    optimistic.initiateSwap({
      fromAsset: fromToken,
      fromAmount,
      toAsset: toToken,
      toAmount: finalToAmount,
      exchangeRate: formattedRate,
      priceImpact: quote.priceImpact.toString(),
      minReceived: `${(parseFloat(finalToAmount || '0') * (1 - slippage / 100)).toFixed(4)} ${toSymbol}`,
      networkFee: quote.fee ? `${quote.fee.toFixed(5)} XLM` : '0.00001 XLM',
      routePath: selectedRoute?.rawPath ?? (quote.data?.path || []),
      walletAddress: walletAddress ?? '',
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
    walletAddress,
  ]);

  const handleSwap = useCallback(() => {
    if (!bypassConfirmation && quote.priceImpact > 5) {
      setIsConfirmModalOpen(true);
      return;
    }
    handleConfirm();
  }, [bypassConfirmation, quote.priceImpact, handleConfirm]);

  const handleSettingsReset = useCallback(() => {
    setSlippage(DEFAULT_SLIPPAGE);
    setDeadline(DEFAULT_DEADLINE);
    updateExpertMode(false);
  }, [setDeadline, setSlippage, updateExpertMode]);

  const handleMax = useCallback(() => {
    // Use spendableBalance for XLM (accounts for base reserve)
    // Use regular balance for other assets
    const maxAmount =
      fromToken === 'native' ? balanceState.spendableBalance : fromBalance;
    setFromAmount(maxAmount ?? '0');
  }, [fromToken, balanceState.spendableBalance, fromBalance, setFromAmount]);

  const handlePresetSelect = useCallback(
    (percentage: number) => {
      const balanceNum = parseFloat(fromBalance);
      if (isNaN(balanceNum) || balanceNum === 0) return;

      // For native assets, respect the spendable balance limit
      const maxSpendable =
        fromToken === 'native'
          ? parseFloat(balanceState.spendableBalance ?? '0')
          : balanceNum;

      const amount = maxSpendable * percentage;
      // Round to 7 decimals to respect asset precision
      const rounded = Math.floor(amount * 10000000) / 10000000;
      setFromAmount(rounded.toString());
    },
    [fromBalance, fromToken, balanceState.spendableBalance, setFromAmount]
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
    };

    window.addEventListener('keydown', onKeydown);
    return () => window.removeEventListener('keydown', onKeydown);
  }, [quote]);

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
          'relative overflow-hidden border-border/40 bg-background/60 backdrop-blur-xl shadow-2xl rounded-[32px] transition-all duration-500 hover:shadow-primary/5',
          isCompact && 'rounded-2xl',
          expertMode &&
            'border-amber-500/30 hover:shadow-amber-500/10 shadow-amber-500/5'
        )}
      >
        {/* Animated Background Gradients */}
        <div className="absolute -top-24 -left-24 w-48 h-48 bg-primary/10 rounded-full blur-3xl animate-pulse" />
        <div className="absolute -bottom-24 -right-24 w-48 h-48 bg-blue-500/10 rounded-full blur-3xl animate-pulse delay-700" />

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
                <span className="text-[10px] font-bold uppercase tracking-wider text-amber-500 bg-amber-500/10 px-2 py-0.5 rounded-full border border-amber-500/20 animate-pulse">
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
                deadline={deadline}
                expertMode={expertMode}
                bypassConfirmation={bypassConfirmation}
                extendedRouteDetails={extendedRouteDetails}
                onSlippageChange={setSlippage}
                onDeadlineChange={setDeadline}
                onExpertModeChange={updateExpertMode}
                onBypassConfirmationChange={updateBypassConfirmation}
                onExtendedRouteDetailsChange={updateExtendedRouteDetails}
                onReset={handleSettingsReset}
              />
              <Button
                variant="ghost"
                size="icon"
                onClick={() => setDiagnosticsOpen(true)}
                aria-label={t('swap.card.diagnostics')}
                className="h-9 w-9 rounded-xl hover:bg-muted/80"
              >
                <Stethoscope className="h-4.5 w-4.5 text-muted-foreground" />
              </Button>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => quote.refresh()}
                disabled={displayQuoteLoading}
                aria-label={t('swap.card.refreshQuote')}
                className="h-9 w-9 rounded-xl hover:bg-muted/80"
              >
                <RefreshCw
                  className={cn(
                    'h-4.5 w-4.5 text-muted-foreground',
                    displayQuoteLoading && 'animate-spin'
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
                  value={fromAmount}
                  onChange={setFromAmount}
                  onMax={handleMax}
                  onPresetSelect={handlePresetSelect}
                  balance={`${fromBalance} ${fromSymbol}`}
                  balanceLoading={balanceState.loading}
                  balanceError={!!balanceState.error}
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
              className="absolute h-10 w-10 rounded-xl bg-background border-border/40 shadow-lg hover:shadow-primary/20 hover:border-primary/40 hover:scale-110 active:scale-95 transition-all duration-300 group"
            >
              <ArrowUpDown className="h-4 w-4 text-primary group-hover:rotate-180 transition-transform duration-500" />
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
                  value={selectedRoute?.expectedAmount ?? displayToAmount}
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
          {(parseFloat(fromAmount) > 0 ||
            storyPresentation?.seedFromAmount) && (
            <div
              className={cn(
                'space-y-3 animate-in fade-in slide-in-from-bottom-2 duration-500',
                isCompact ? 'space-y-2 pt-1' : 'pt-2'
              )}
            >
              <PriceInfoPanel
                rate={displayFormattedRate}
                priceImpact={displayQuotePriceImpact}
                minReceived={`${(parseFloat(displayToAmount || '0') * (1 - slippage / 100)).toFixed(4)} ${toSymbol}`}
                networkFee={
                  quote.fee ? `${quote.fee.toFixed(5)} XLM` : '0.00001 XLM'
                }
                isLoading={displayQuoteLoading}
                onExportJson={() => handleExport('json')}
                onExportCsv={() => handleExport('csv')}
              />
              <MobileRouteBottomSheet
                quote={quote.data ?? null}
                amountOut={selectedRoute?.expectedAmount ?? displayToAmount}
                isLoading={displayQuoteLoading}
              />
              {showRoutePicker && (
                <RouteDisplay
                  amountOut={selectedRoute?.expectedAmount ?? displayToAmount}
                  isLoading={displayQuoteLoading}
                  onSelect={setSelectedRoute}
                />
              )}
              {batchSwapsEnabled && (
                <BatchSwapPreview
                  legs={batchLegs}
                  isLoading={batchQuote.loading}
                  error={batchQuote.error?.message}
                  onRetry={batchQuote.refresh}
                />
              )}
              {/* Share Quote Button */}
              <div className="flex justify-end">
                <ShareQuoteButton
                  params={{
                    from: fromToken,
                    to: toToken,
                    amount: fromAmount,
                    slippage: slippage.toString(),
                  }}
                  disabled={!fromAmount || parseFloat(fromAmount) === 0}
                />
              </div>
            </div>
          )}

          {/* Stale Indicator */}
          {displayQuoteStale && (
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

          {/* --- Issue #506: Optional Memo Interface Module --- */}
          <div className="space-y-2 pt-1">
            <button
              type="button"
              onClick={() => {
                setShowMemoField(!showMemoField);
                if (!showMemoField) setMemoValue('');
              }}
              className="text-xs font-semibold text-primary/80 hover:text-primary transition-colors flex items-center gap-1.5 focus:outline-none"
            >
              <span>
                {showMemoField ? '✕ Remove Memo' : '+ Add Optional Memo'}
              </span>
            </button>

            {showMemoField && (
              <div className="p-3 bg-muted/20 border border-border/20 rounded-2xl space-y-3">
                <div className="flex gap-2">
                  <Button
                    type="button"
                    variant={memoType === 'text' ? 'default' : 'outline'}
                    size="sm"
                    className="h-7 text-xs rounded-lg flex-1"
                    onClick={() => {
                      setMemoType('text');
                      setMemoValue('');
                    }}
                  >
                    Text Memo
                  </Button>
                  <Button
                    type="button"
                    variant={memoType === 'hash' ? 'default' : 'outline'}
                    size="sm"
                    className="h-7 text-xs rounded-lg flex-1"
                    onClick={() => {
                      setMemoType('hash');
                      setMemoValue('');
                    }}
                  >
                    Hash Memo
                  </Button>
                </div>

                <div className="space-y-1">
                  <input
                    type="text"
                    value={memoValue}
                    onChange={(e) => setMemoValue(e.target.value)}
                    placeholder={
                      memoType === 'text'
                        ? 'Enter text reference (max 28 bytes)'
                        : 'Enter 64-char hex string'
                    }
                    className={cn(
                      'w-full bg-background/50 border rounded-xl px-3 py-1.5 text-xs font-mono focus:outline-none focus:ring-2 focus:ring-primary/20',
                      memoError
                        ? 'border-destructive focus:ring-destructive/20'
                        : 'border-border/60 focus:border-primary/40'
                    )}
                  />
                  {memoError && (
                    <p className="text-[11px] font-medium text-destructive px-1">
                      {memoError}
                    </p>
                  )}
                </div>
              </div>
            )}
          </div>

          {/* Warnings Panel */}
          <SwapWarningCenter
            warnings={warnings}
            onRemoveWarning={handleRemoveWarning}
            className="mb-2"
          />

          {/* Assistive Screen Reader Region */}
          <div className="sr-only" aria-live="assertive" role="status">
            {warnings
              .filter((w) => w.type === 'error')
              .map((w) => `${w.title}: ${w.message}`)
              .join('. ')}
          </div>

          {/* Action Button */}
          <div className="pt-2">
            <SwapButton
              state={displayButtonState}
              onSwap={handleSwap}
              onConnectWallet={() => connect('freighter')} // Connection managed by WalletProvider
              isLoading={displayQuoteLoading}
            />
          </div>

          {/* Status/Error Messages */}
          {displayQuoteError && (
            <p className="text-center text-xs font-medium text-destructive animate-pulse">
              {toTraderErrorLine(getTraderErrorCopy(displayQuoteError))}
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
          isOpen={displayIsModalOpen}
          status={displayOptimisticStatus}
          txHash={optimistic.txHash}
          errorMessage={optimistic.errorMessage}
          tradeParams={displayTradeParams}
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
          onSwapAgain={handleSwapAgain}
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

      <DiagnosticsPanel
        quote={quote.data}
        requestId={quote.requestId}
        lastQuotedAtMs={quote.lastQuotedAtMs}
        isOpen={diagnosticsOpen}
        onOpenChange={setDiagnosticsOpen}
      />

      {/* Footer Info */}
      <p className="text-center text-[10px] text-muted-foreground/60 mt-4 px-8 uppercase tracking-widest font-bold">
        {t('swap.card.poweredBy')}
      </p>

      <Dialog open={shortcutHelpOpen} onOpenChange={handleShortcutOpenChange}>
        <DialogContent className="max-w-lg max-h-[85vh] overflow-y-auto">
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
          </ul>
          <IconographyLegend embedded className="mt-4" />
        </DialogContent>
      </Dialog>
    </div>
  );
}
