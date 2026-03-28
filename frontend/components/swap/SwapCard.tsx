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
import { useState, useRef, useEffect } from 'react';
import { SwapValidationSchema } from '@/lib/swap-validation';
import { STELLAR_NATIVE_MAX_DECIMALS } from '@/lib/amount-input';

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
  const [confidenceScore, setConfidenceScore] = useState<number>(85);
  const [volatility, setVolatility] = useState<'high' | 'medium' | 'low'>('low');
  const loadingTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (loadingTimeoutRef.current) {
        clearTimeout(loadingTimeoutRef.current);
      }
    };
  }, []);

  const validation = SwapValidationSchema.validate(
    {
      amount: payAmount,
      maxDecimals: STELLAR_NATIVE_MAX_DECIMALS,
      slippage,
    },
    { mode: 'submit', requirePair: false },
  );
  const isValidAmount = validation.amountResult.status === 'ok';

  // Simulate quote fetching with confidence and volatility
  // Minimum 300ms delay before hiding skeleton to prevent flicker on fast responses
  const handlePayAmountChange = (amount: string) => {
    setPayAmount(amount);
    if (parseFloat(amount) > 0) {
      setIsLoading(true);

      if (loadingTimeoutRef.current) {
        clearTimeout(loadingTimeoutRef.current);
      }

      loadingTimeoutRef.current = setTimeout(() => {
        const amountNum = parseFloat(amount);
        setReceiveAmount((amountNum * 0.98).toFixed(4));
        // Simulate varying confidence based on amount
        const newConfidence = Math.max(50, Math.min(95, 90 - (amountNum / 100)));
        setConfidenceScore(Math.round(newConfidence));
        // Simulate volatility based on amount
        if (amountNum > 1000) {
          setVolatility('high');
        } else if (amountNum > 100) {
          setVolatility('medium');
        } else {
          setVolatility('low');
        }
        setIsLoading(false);
      }, 500); // Minimum 500ms total (300ms min + 200ms delay) guarantees skeleton shows
    } else {
      setReceiveAmount('');
      setConfidenceScore(85);
      setVolatility('low');
      setIsLoading(false);

      if (loadingTimeoutRef.current) {
        clearTimeout(loadingTimeoutRef.current);
      }
    }
  };

  const handleReset = () => {
    reset();
    setReceiveAmount('');
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
          </div>
        )}
        <SwapCTA
          validation={validation}
          isLoading={isLoading}
          onSwap={() => console.log('Swapping...')}
        />
      </CardContent>
    </Card>
  );
}
