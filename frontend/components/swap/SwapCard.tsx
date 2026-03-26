'use client';

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { useState } from 'react';
import { PairSelector } from './PairSelector';
import { QuoteSummary } from './QuoteSummary';
import { RouteDisplay } from './RouteDisplay';
import { SlippageControl } from './SlippageControl';
import { SwapCTA } from './SwapCTA';

export function SwapCard() {
  // Mock states for the demo
  const [payAmount, setPayAmount] = useState<string>('');
  const [receiveAmount, setReceiveAmount] = useState<string>('');
  const [slippage, setSlippage] = useState<number>(0.5);
  const [isLoading, setIsLoading] = useState<boolean>(false);

  // Derived state for the button
  const isValidAmount = parseFloat(payAmount) > 0;

  // Simulate quote fetching
  const handlePayAmountChange = (amount: string) => {
    setPayAmount(amount);
    if (parseFloat(amount) > 0) {
      setIsLoading(true);
      setTimeout(() => {
        setReceiveAmount((parseFloat(amount) * 0.98).toFixed(4));
        setIsLoading(false);
      }, 500);
    } else {
      setReceiveAmount('');
    }
  };

  return (
    <Card className="w-full border shadow-sm">
      <CardHeader className="pb-4">
        <div className="flex items-center justify-between flex-row">
          <CardTitle className="text-xl font-semibold">Swap</CardTitle>
          <SlippageControl slippage={slippage} onChange={setSlippage} />
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <PairSelector
          payAmount={payAmount}
          onPayAmountChange={handlePayAmountChange}
          receiveAmount={receiveAmount}
        />
        {isValidAmount && !isLoading && receiveAmount && (
          <>
            <SimulationPanel
              payAmount={payAmount}
              expectedOutput={receiveAmount}
              slippage={slippage}
              isLoading={isLoading}
            />
            <QuoteSummary rate="1 XLM ≈ 0.98 USDC" fee="0.01 XLM" priceImpact="< 0.1%" />
            <RouteDisplay amountOut={receiveAmount} />
          </>
        )}
        <SwapCTA
          amount={payAmount}
          isLoading={isLoading}
          hasPair={true}
          onSwap={() => console.log('Swapping...')}
        />
      </CardContent>
    </Card>
  );
}
