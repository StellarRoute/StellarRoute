'use client';

import { useCallback, useMemo } from 'react';
import { useTradeFormStorage } from './useTradeFormStorage';
import { useQuote } from './useQuote';

export function useSwapState() {
  const {
    amount: fromAmount,
    setAmount: setFromAmount,
    slippage,
    setSlippage,
    deadline,
    setDeadline,
    fromToken,
    setFromToken,
    toToken,
    setToToken,
    setTokenPair,
    pendingRecovery,
    restorePending,
    discardPending,
    hasRecoverableState,
    snapshotCurrent,
    reset,
  } = useTradeFormStorage();

  const parsedAmount = useMemo(() => {
    const val = parseFloat(fromAmount);
    return isFinite(val) && val > 0 ? val : undefined;
  }, [fromAmount]);

  const quote = useQuote({
    fromToken,
    toToken,
    amount: parsedAmount,
    type: 'sell',
  });

  const switchTokens = useCallback(() => {
    setTokenPair(toToken, fromToken);
    // If we have an output amount, we might want to set it as the new input amount
    if (quote.outputAmount > 0) {
      setFromAmount(quote.outputAmount.toString());
    }
  }, [fromToken, quote.outputAmount, setFromAmount, setTokenPair, toToken]);

  const formattedRate = useMemo(() => {
    if (!quote.rate) return '';
    const fromSymbol = fromToken === 'native' ? 'XLM' : fromToken.split(':')[0];
    const toSymbol = toToken === 'native' ? 'XLM' : toToken.split(':')[0];
    return `1 ${fromSymbol} = ${quote.rate.toFixed(4)} ${toSymbol}`;
  }, [quote.rate, fromToken, toToken]);

  return {
    fromToken,
    setFromToken,
    toToken,
    setToToken,
    fromAmount,
    setFromAmount,
    toAmount: quote.outputAmount > 0 ? quote.outputAmount.toString() : '',
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
  };
}
