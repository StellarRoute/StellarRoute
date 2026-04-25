'use client';

import { useState, useCallback, useMemo } from 'react';
import { useTradeFormStorage } from './useTradeFormStorage';
import { useQuote } from './useQuote';

const DEFAULT_FROM_TOKEN = 'native';
const DEFAULT_TO_TOKEN = 'USDC:GA5ZSEJYB37JRC5AVCIAZDL2Y343IFRMA2EO3HJWV2XG7H5V5CQRUP7W';

export function useSwapState() {
  const { amount: fromAmount, setAmount: setFromAmount, slippage, setSlippage, reset } = useTradeFormStorage();
  
  const [fromToken, setFromToken] = useState(DEFAULT_FROM_TOKEN);
  const [toToken, setToToken] = useState(DEFAULT_TO_TOKEN);
  
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
    setFromToken(toToken);
    setToToken(fromToken);
    // If we have an output amount, we might want to set it as the new input amount
    if (quote.outputAmount > 0) {
      setFromAmount(quote.outputAmount.toString());
    }
  }, [fromToken, toToken, quote.outputAmount, setFromAmount]);

  const isReverseRate = false; // Could be state to toggle rate display (e.g. 1 XLM = 0.1 USDC vs 1 USDC = 10 XLM)
  
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
    quote,
    switchTokens,
    formattedRate,
    reset,
  };
}
