import { describe, expect, it } from 'vitest';

import { calculateQuoteRetryDelayMs } from './quote-retry';

describe('calculateQuoteRetryDelayMs', () => {
  it('caps exponential growth at the configured max delay', () => {
    expect(
      calculateQuoteRetryDelayMs(
        4,
        {
          baseDelayMs: 100,
          maxDelayMs: 350,
          jitterRatio: 0,
        },
        () => 0.5,
      ),
    ).toBe(350);
  });

  it('applies bounded jitter around the computed delay', () => {
    expect(
      calculateQuoteRetryDelayMs(
        2,
        {
          baseDelayMs: 100,
          maxDelayMs: 1_000,
          jitterRatio: 0.25,
        },
        () => 1,
      ),
    ).toBe(250);

    expect(
      calculateQuoteRetryDelayMs(
        2,
        {
          baseDelayMs: 100,
          maxDelayMs: 1_000,
          jitterRatio: 0.25,
        },
        () => 0,
      ),
    ).toBe(150);
  });
});
