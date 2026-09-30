import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { PrintHeader, formatPrintTimestamp } from './PrintHeader';

describe('formatPrintTimestamp', () => {
  it('formats an ISO timestamp as YYYY-MM-DD HH:mm UTC', () => {
    expect(formatPrintTimestamp('2026-08-28T09:32:00.000Z')).toBe(
      '2026-08-28 09:32 UTC'
    );
  });

  it('pads single-digit month/day/hour/minute', () => {
    expect(formatPrintTimestamp('2026-01-02T03:04:00.000Z')).toBe(
      '2026-01-02 03:04 UTC'
    );
  });

  it('returns the original string when it cannot be parsed', () => {
    expect(formatPrintTimestamp('not-a-date')).toBe('not-a-date');
  });
});

describe('PrintHeader', () => {
  it('renders the brand, pair, and formatted timestamp', () => {
    render(
      <PrintHeader
        capturedAt="2026-08-28T09:32:00.000Z"
        fromSymbol="XLM"
        toSymbol="USDC"
      />
    );

    expect(screen.getByText('StellarRoute')).toBeInTheDocument();
    expect(screen.getByText(/XLM/)).toBeInTheDocument();
    expect(screen.getByText(/USDC/)).toBeInTheDocument();
    expect(screen.getByText('2026-08-28 09:32 UTC')).toBeInTheDocument();
  });
});
