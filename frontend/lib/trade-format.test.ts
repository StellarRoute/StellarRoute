// frontend/lib/trade-format.test.ts
import { describe, it } from 'vitest';
import * as fc from 'fast-check';
import { truncateTxHash, formatTradeTimestamp, formatTradeAmount } from './trade-format';

describe('trade-format utilities', () => {
  it('should truncate hash correctly', () => {
    fc.assert(
      fc.property(fc.string({ minLength: 13 }), (hash) => {
        const truncated = truncateTxHash(hash);
        return truncated.length === 13 && truncated.startsWith(hash.slice(0, 8)) && truncated.endsWith(hash.slice(-4));
      })
    );
  });

  it('should format timestamp to YYYY-MM-DD HH:mm UTC', () => {
    fc.assert(
      fc.property(fc.date(), (date) => {
        const formatted = formatTradeTimestamp(date);
        return /^\d{4}-\d{2}-\d{2} \d{2}:\d{2} UTC$/.test(formatted);
      })
    );
  });

  it('should format amount with max 7 decimal places and no trailing zeros', () => {
    fc.assert(
      // Switched to fc.double() to safely handle standard JavaScript numbers and the 1e14 max limit
      fc.property(fc.double({ min: 0, max: 1e14, noNaN: true, noInfinity: true }), (amount) => {
        const formatted = formatTradeAmount(amount.toString());
        const parts = formatted.split('.');
        const decimalPart = parts[1] || "";
        
        const hasValidDecimals = parts.length <= 2 && decimalPart.length <= 7;
        const noTrailingZeros = decimalPart === "" || !decimalPart.endsWith('0');
        
        return hasValidDecimals && noTrailingZeros;
      })
    );
  });
});