'use client';

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { Button } from '@/components/ui/button';
import { RotateCcw } from 'lucide-react';
import { PairSelector } from './PairSelector';
import { QuoteSummary } from './QuoteSummary';
import { RouteDisplay } from './RouteDisplay';
import { SlippageControl } from './SlippageControl';
import { SwapCTA } from './SwapCTA';
import { SimulationPanel } from './SimulationPanel';
import { FeeBreakdownPanel } from './FeeBreakdownPanel';
import { useTradeFormStorage } from '@/hooks/useTradeFormStorage';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useOnlineStatus } from '@/hooks/useOnlineStatus';
import { STELLAR_NATIVE_MAX_DECIMALS } from '@/lib/amount-input';
import { SwapValidationSchema } from '@/lib/swap-validation';

export function SwapCard() {
  const {
    amount: payAmount,
    setAmount: setPayAmount,
    slippage,
    setSlippage,
    reset,
    isHydrated,
  } = useTradeFormStorage();

  const [receiveAmount, setReceiveAmount] = useState<string>('');
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [quoteError, setQuoteError] = useState<string | null>(null);
  const quoteTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const { isOnline, isOffline } = useOnlineStatus();
  const [confidenceScore, setConfidenceScore] = useState<number>(85);
  const [volatility, setVolatility] = useState<'high' | 'medium' | 'low'>('low');

  const validation = SwapValidationSchema.validate(
    {
      amount: payAmount,
      maxDecimals: STELLAR_NATIVE_MAX_DECIMALS,
      slippage,
    },
    { mode: 'submit', requirePair: false },
  );
  const isValidAmount = validation.amountResult.status === 'ok';

  const clearQuoteTimer = useCallback(() => {
    if (quoteTimerRef.current) {
      clearTimeout(quoteTimerRef.current);
      quoteTimerRef.current = null;
    }
  }, []);

  const requestQuote = useCallback((amount: string) => {
    clearQuoteTimer();
    const amountNumber = parseFloat(amount);

    if (!Number.isFinite(amountNumber) || amountNumber <= 0) {
      setIsLoading(false);
      setQuoteError(null);
      setReceiveAmount('');
      setConfidenceScore(85);
      setVolatility('low');
      return;
    }

    if (!isOnline) {
      setIsLoading(false);
      setQuoteError('You are offline. Reconnect to refresh quote.');
      setReceiveAmount('');
      return;
    }

    setIsLoading(true);
    setQuoteError(null);

    quoteTimerRef.current = setTimeout(() => {
      setReceiveAmount((amountNumber * 0.98).toFixed(4));
      const nextConfidence = Math.max(50, Math.min(95, 90 - amountNumber / 100));
      setConfidenceScore(Math.round(nextConfidence));
      if (amountNumber > 1000) {
        setVolatility('high');
      } else if (amountNumber > 100) {
        setVolatility('medium');
      } else {
        setVolatility('low');
      }
      setIsLoading(false);
    }, 500);
  }, [clearQuoteTimer, isOnline]);

  const handlePayAmountChange = (amount: string) => {
    setPayAmount(amount);
    requestQuote(amount);
  };

  const handleRetryQuote = () => {
    requestQuote(payAmount);
  };

  useEffect(() => {
    if (!isOnline) {
      clearQuoteTimer();
      // eslint-disable-next-line react-hooks/set-state-in-effect -- intentional UI transition for offline mode
      setIsLoading(false);
      if (parseFloat(payAmount) > 0) {
        setQuoteError('You are offline. Reconnect to refresh quote.');
      }
      return;
    }

    // Automatic recovery: once online, refresh the active quote.
    if (quoteError && parseFloat(payAmount) > 0) {
      requestQuote(payAmount);
    }
  }, [isOnline, payAmount, quoteError, clearQuoteTimer, requestQuote]);

  useEffect(() => {
    return () => {
      clearQuoteTimer();
    };
  }, [clearQuoteTimer]);

  const handleReset = () => {
    clearQuoteTimer();
    reset();
    setReceiveAmount('');
    setQuoteError(null);
    setIsLoading(false);
    setConfidenceScore(85);
    setVolatility('low');
  };

  // Defer render until localStorage has been read to avoid flash of default values
  if (!isHydrated) {
    return (
      <Card className="w-full border shadow-sm">
        <CardHeader className="pb-4">
          <CardTitle className="text-xl font-semibold">Swap</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="h-32 animate-pulse rounded-lg bg-muted" />
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="w-full border shadow-sm">
      <CardHeader className="pb-4">
        {isOffline && (
          <div className="mb-3 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
            You&apos;re offline. Quote refresh and swap submission are paused until
            your connection is restored.
          </div>
        )}
        <div className="flex items-center justify-between flex-row">
          <CardTitle className="text-xl font-semibold">Swap</CardTitle>
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="icon"
              className="h-11 w-11 rounded-full"
              onClick={handleReset}
              title="Clear form"
            >
              <RotateCcw className="h-4 w-4 text-muted-foreground" />
              <span className="sr-only">Clear form</span>
            </Button>
            <SlippageControl slippage={slippage} onChange={setSlippage} />
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <PairSelector
          payAmount={payAmount}
          onPayAmountChange={handlePayAmountChange}
          receiveAmount={receiveAmount}
        />
        {isValidAmount && (
          <div className="space-y-4">
            <SimulationPanel
              payAmount={payAmount}
              expectedOutput={receiveAmount}
              slippage={slippage}
              isLoading={isLoading}
            />
            <FeeBreakdownPanel
              protocolFees={[
                { name: 'Router Fee', amount: '0.001 XLM', description: 'Fee for using StellarRoute aggregator' },
                { name: 'Pool Fee', amount: '0.003%', description: 'Liquidity provider fee for AQUA pool' },
              ]}
              networkCosts={[
                { name: 'Base Fee', amount: '0.00001 XLM', description: 'Stellar network base transaction fee' },
                { name: 'Operation Fee', amount: '0.00002 XLM', description: 'Fee for path payment operations' },
              ]}
              totalFee="0.01 XLM"
              netOutput={`${(parseFloat(receiveAmount || '0') * 0.99).toFixed(4)} USDC`}
            />
            <QuoteSummary
              rate="1 XLM ≈ 0.98 USDC"
              fee="0.01 XLM"
              priceImpact="< 0.1%"
              isLoading={isLoading}
            />
            <RouteDisplay
              amountOut={receiveAmount}
              confidenceScore={confidenceScore}
              volatility={volatility}
              isLoading={isLoading}
            />
          </>
        )}
        {quoteError && isValidAmount && (
          <div className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
            <p>{quoteError}</p>
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="mt-2"
              onClick={handleRetryQuote}
              disabled={!isOnline || isLoading}
            >
              Retry quote
            </Button>
          </div>
        )}
        <SwapCTA
          validation={validation}
          isLoading={isLoading}
          isOnline={isOnline}
          onSwap={() => console.log('Swapping...')}
        />
      </CardContent>
    </Card>
  );
}
