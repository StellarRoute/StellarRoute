import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, fireEvent, render, screen } from '@testing-library/react';
import { useState, type ComponentProps } from 'react';

import { CardFxFreshnessBadge } from './CardFxFreshnessBadge';
import {
  computeCardFxQuotes,
  formatCardFxRate,
  getCardFxQuote,
} from '@/lib/card/fx';

const RATE = '1 USDC = 1538.46 NGN';

function renderBadge(
  overrides: Partial<ComponentProps<typeof CardFxFreshnessBadge>> = {},
) {
  const onRefresh = vi.fn();
  const result = render(
    <CardFxFreshnessBadge
      rate={RATE}
      retrievedAtMs={1_000_000}
      onRefresh={onRefresh}
      {...overrides}
    />,
  );
  return { ...result, onRefresh };
}

function countdownLabel() {
  return screen.getByTestId('card-fx-countdown-label');
}

function setVisibility(visibilityState: 'visible' | 'hidden') {
  Object.defineProperty(document, 'visibilityState', {
    configurable: true,
    value: visibilityState,
  });
  document.dispatchEvent(new Event('visibilitychange'));
}

function tick(seconds: number) {
  act(() => {
    vi.advanceTimersByTime(seconds * 1000);
  });
}

describe('CardFxFreshnessBadge', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    setVisibility('visible');
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
    delete (document as unknown as { visibilityState?: string }).visibilityState;
  });

  describe('countdown', () => {
    it('initializes the countdown to the 60 second TTL', () => {
      renderBadge();

      expect(countdownLabel()).toHaveTextContent('Quote valid for 60s');
    });

    it('decrements once per second', () => {
      renderBadge();

      tick(1);
      expect(countdownLabel()).toHaveTextContent('Quote valid for 59s');

      tick(5);
      expect(countdownLabel()).toHaveTextContent('Quote valid for 54s');
    });

    it('honours a custom TTL', () => {
      renderBadge({ ttlSeconds: 5 });

      expect(countdownLabel()).toHaveTextContent('Quote valid for 5s');
      tick(5);
      expect(countdownLabel()).toHaveTextContent('Quote Stale');
    });

    it('shrinks the countdown ring as the quote ages', () => {
      const { container } = renderBadge();
      const ring = container.querySelectorAll('circle')[1];
      const circumference = Number(ring.getAttribute('stroke-dasharray'));

      expect(Number(ring.getAttribute('stroke-dashoffset'))).toBeCloseTo(0, 3);

      tick(30);
      expect(Number(ring.getAttribute('stroke-dashoffset'))).toBeCloseTo(
        circumference * 0.5,
        3,
      );
    });
  });

  describe('expiration', () => {
    it('marks the quote as stale when the timer reaches zero', () => {
      renderBadge();

      tick(59);
      expect(countdownLabel()).toHaveTextContent('Quote valid for 1s');

      tick(1);
      expect(countdownLabel()).toHaveTextContent(
        'Quote Stale — click to update',
      );
    });

    it('applies the amber warning style and dims the indicative rate', () => {
      renderBadge();

      tick(60);

      expect(screen.getByTestId('card-fx-freshness-badge')).toHaveClass(
        'border-amber-300',
      );
      expect(screen.getByTestId('card-fx-indicative-rate')).toHaveClass(
        'opacity-50',
      );
      expect(screen.getByTestId('card-fx-indicative-rate')).toHaveTextContent(
        RATE,
      );
    });

    it('never removes the indicative disclaimer, fresh or stale', () => {
      renderBadge();

      expect(
        screen.getByTestId('card-fx-indicative-disclaimer'),
      ).toHaveTextContent('indicative: true');

      tick(60);

      expect(
        screen.getByTestId('card-fx-indicative-disclaimer'),
      ).toHaveTextContent('indicative: true');
    });

    it('stays at zero once expired', () => {
      renderBadge();

      tick(75);

      expect(countdownLabel()).toHaveTextContent(
        'Quote Stale — click to update',
      );
    });
  });

  describe('manual refresh', () => {
    it('resets the countdown and notifies the caller', () => {
      const { onRefresh } = renderBadge();

      tick(45);
      fireEvent.click(screen.getByRole('button', { name: /refresh rate/i }));

      expect(onRefresh).toHaveBeenCalledTimes(1);
      expect(countdownLabel()).toHaveTextContent('Quote valid for 60s');
    });

    it('clears the stale state when refreshed after expiry', () => {
      renderBadge();

      tick(60);
      fireEvent.click(screen.getByRole('button', { name: /refresh rate/i }));

      expect(countdownLabel()).toHaveTextContent('Quote valid for 60s');
      expect(screen.getByTestId('card-fx-freshness-badge')).not.toHaveClass(
        'border-amber-300',
      );
    });

    it('re-computes the fixture quote when the dashboard handles the refresh', () => {
      function Dashboard() {
        const [snapshot, setSnapshot] = useState(() =>
          computeCardFxQuotes(0),
        );
        return (
          <CardFxFreshnessBadge
            rate={formatCardFxRate(getCardFxQuote(snapshot, 'NGN'))}
            retrievedAtMs={snapshot.retrievedAtMs}
            onRefresh={() =>
              setSnapshot(
                computeCardFxQuotes(Date.now(), snapshot.revision + 1),
              )
            }
          />
        );
      }

      render(<Dashboard />);

      expect(screen.getByTestId('card-fx-indicative-rate')).toHaveTextContent(
        '1 USDC = 1538.46 NGN',
      );

      tick(20);
      fireEvent.click(screen.getByRole('button', { name: /refresh rate/i }));

      expect(countdownLabel()).toHaveTextContent('Quote valid for 60s');
      expect(screen.getByTestId('card-fx-indicative-rate')).not.toHaveTextContent(
        '1 USDC = 1538.46 NGN',
      );
    });

    it('restarts the countdown when a newer quote is supplied', () => {
      const { rerender } = render(
        <CardFxFreshnessBadge
          rate={RATE}
          retrievedAtMs={1_000_000}
          onRefresh={vi.fn()}
        />,
      );

      tick(50);
      expect(countdownLabel()).toHaveTextContent('Quote valid for 10s');

      rerender(
        <CardFxFreshnessBadge
          rate={RATE}
          retrievedAtMs={1_060_000}
          onRefresh={vi.fn()}
        />,
      );
      expect(countdownLabel()).toHaveTextContent('Quote valid for 60s');
    });
  });

  describe('backgrounded tab', () => {
    it('pauses the countdown while hidden and resumes when visible', () => {
      renderBadge();

      setVisibility('hidden');
      tick(30);
      expect(countdownLabel()).toHaveTextContent('Quote valid for 60s');

      setVisibility('visible');
      tick(1);
      expect(countdownLabel()).toHaveTextContent('Quote valid for 59s');
    });

    it('does not expire a quote while the tab stays backgrounded', () => {
      renderBadge();

      setVisibility('hidden');
      tick(120);

      expect(countdownLabel()).toHaveTextContent('Quote valid for 60s');
    });

    it('keeps the paused countdown when refreshed while hidden', () => {
      const { onRefresh } = renderBadge();

      tick(10);
      setVisibility('hidden');
      fireEvent.click(screen.getByRole('button', { name: /refresh rate/i }));

      expect(onRefresh).toHaveBeenCalledTimes(1);
      expect(countdownLabel()).toHaveTextContent('Quote valid for 60s');
    });
  });

  describe('accessibility', () => {
    it('uses a non-submitting, labelled refresh button', () => {
      renderBadge();

      const button = screen.getByRole('button', { name: /refresh rate/i });
      expect(button).toHaveAttribute('type', 'button');
    });

    it('hides the decorative ring from assistive tech', () => {
      const { container } = renderBadge();

      expect(
        container.querySelector('[data-testid="card-fx-countdown-ring"]'),
      ).toHaveAttribute('aria-hidden', 'true');
    });
  });
});
