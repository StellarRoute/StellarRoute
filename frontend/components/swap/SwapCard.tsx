'use client';

import { useState, useCallback, useMemo } from 'react';
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
import { useSwapState } from '@/hooks/useSwapState';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';

export function SwapCard() {
  const {
    fromToken,
    setFromToken,
    toToken,
    setToToken,
    fromAmount,
    setFromAmount,
    toAmount,
    slippage,
    quote,
    switchTokens,
    formattedRate,
  } = useSwapState();

  const [isConnected, setIsConnected] = useState(false);
  const [isSwapping, setIsSwapping] = useState(false);
  const [isConfirmModalOpen, setIsConfirmModalOpen] = useState(false);
  const [selectedRoute, setSelectedRoute] = useState<AlternativeRoute | null>(null);
  
  // Mock balance
  const fromBalance = "100.00"; 
  const fromSymbol = fromToken === 'native' ? 'XLM' : fromToken.split(':')[0];
  const toSymbol = toToken === 'native' ? 'XLM' : toToken.split(':')[0];

  const buttonState = useMemo<SwapButtonState>(() => {
    if (isSwapping) return "executing";
    if (!isConnected) return "no_wallet";
    if (!fromAmount || parseFloat(fromAmount) === 0) return "no_amount";
    if (parseFloat(fromAmount) > parseFloat(fromBalance)) return "insufficient_balance";
    if (quote.priceImpact > 10) return "high_impact_warning";
    if (quote.isStale) return "error";
    if (quote.error) return "error";
    return "ready";
  }, [isConnected, fromAmount, fromBalance, quote.priceImpact, quote.isStale, quote.error, isSwapping]);

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
              <SettingsPanel />
              <Button 
                variant="ghost" 
                size="icon" 
                onClick={() => quote.refresh()} 
                disabled={quote.loading}
                aria-label="Refresh quote"
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
                  label="You Pay"
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
                  label="You Receive"
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
              />
              <RouteDisplay
                amountOut={selectedRoute?.expectedAmount ?? toAmount}
                isLoading={quote.loading}
                onSelect={setSelectedRoute}
              />
            </div>
          )}

          {/* Stale / Recovering Indicators */}
          {quote.isStale && (
            <span
              data-testid="stale-indicator"
              className="text-xs text-amber-500 font-medium"
            >
              Quote outdated — refresh for latest price
            </span>
          )}
          {quote.isRecovering && (
            <span
              data-testid="recovering-indicator"
              className="text-xs text-blue-500 font-medium"
            >
              Retrying quote...
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

      {/* Footer Info */}
      <p className="text-center text-[10px] text-muted-foreground/60 mt-4 px-8 uppercase tracking-widest font-bold">
        Powered by StellarRoute Aggregator
      </p>
    </div>
  );
}

