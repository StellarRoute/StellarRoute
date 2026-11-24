'use client';

import { useMemo, useState } from 'react';
import {
  requiredUsdcForFiat,
  type TargetFiatCurrency,
} from '../../lib/card/fx';

export interface CardTargetSpendCalculatorProps {
  currentUsdc?: number;
  onBridgeRequired?: (usdcAmount: number) => void;
}

const CURRENCIES: TargetFiatCurrency[] = ['EUR', 'GBP', 'JPY'];

export function CardTargetSpendCalculator({
  currentUsdc = 0,
  onBridgeRequired,
}: CardTargetSpendCalculatorProps) {
  const [currency, setCurrency] = useState<TargetFiatCurrency>('EUR');
  const [fiatInput, setFiatInput] = useState('90');

  const targetFiat = Number(fiatInput) || 0;

  const result = useMemo(
    () =>
      requiredUsdcForFiat({
        targetFiat,
        currency,
        currentUsdc,
      }),
    [targetFiat, currency, currentUsdc],
  );

  return (
    <div className="card-target-spend-calculator" data-testid="card-target-spend">
      <h3>Target spending</h3>
      <p>Plan how much fiat you want to spend, then bridge the USDC needed.</p>

      <label>
        Currency
        <select
          value={currency}
          onChange={(e) => setCurrency(e.target.value as TargetFiatCurrency)}
          data-testid="currency-select"
        >
          {CURRENCIES.map((c) => (
            <option key={c} value={c}>
              {c}
            </option>
          ))}
        </select>
      </label>

      <label>
        Desired fiat amount
        <input
          type="number"
          min={0}
          step={currency === 'JPY' ? 1 : 0.01}
          value={fiatInput}
          onChange={(e) => setFiatInput(e.target.value)}
          data-testid="fiat-input"
        />
      </label>

      <div data-testid="required-usdc">
        Required USDC deposit: <strong>{result.requiredUsdc.toFixed(2)}</strong>
      </div>

      <span data-testid="indicative-badge" title="Using fixture rates">
        indicative: true (fixture rates)
      </span>

      <button
        type="button"
        data-testid="bridge-required-btn"
        disabled={result.requiredUsdc <= 0}
        onClick={() => onBridgeRequired?.(result.requiredUsdc)}
      >
        Bridge Required Amount
      </button>
    </div>
  );
}