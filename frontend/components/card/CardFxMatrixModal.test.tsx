import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within } from '@testing-library/react';

import { CardFxMatrixModal } from './CardFxMatrixModal';
import { CARD_FX_MATRIX_FIXTURE_DATE } from '@/lib/card/fx';

if (typeof global.ResizeObserver === 'undefined') {
  global.ResizeObserver = class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
}

function renderModal(usdcAmount: number, onOpenChange = vi.fn()) {
  render(
    <CardFxMatrixModal
      open
      onOpenChange={onOpenChange}
      usdcAmount={usdcAmount}
    />,
  );
  return onOpenChange;
}

describe('CardFxMatrixModal', () => {
  it('renders nothing when closed', () => {
    render(
      <CardFxMatrixModal open={false} onOpenChange={vi.fn()} usdcAmount={100} />,
    );

    expect(screen.queryByTestId('card-fx-matrix-modal')).not.toBeInTheDocument();
  });

  it('renders an accessible dialog with title and description', () => {
    renderModal(100);

    const dialog = screen.getByRole('dialog', { name: 'Currency comparison' });
    expect(dialog).toHaveAccessibleDescription(
      'What 100 USDC converts to across supported card currencies.',
    );
  });

  it('shows a row per currency with rate and converted amount', () => {
    renderModal(100);

    const expected = {
      EUR: ['1 USDC = 0.9206 EUR', '92.06'],
      GBP: ['1 USDC = 0.7895 GBP', '78.95'],
      JPY: ['1 USDC = 151.62 JPY', '15162'],
      CAD: ['1 USDC = 1.3874 CAD', '138.74'],
    };

    for (const [currency, [rate, amount]] of Object.entries(expected)) {
      const row = screen.getByTestId(`card-fx-matrix-row-${currency}`);
      expect(within(row).getByText(rate)).toBeInTheDocument();
      expect(
        screen.getByTestId(`card-fx-matrix-amount-${currency}`),
      ).toHaveTextContent(amount);
    }
    expect(screen.getAllByRole('columnheader')).toHaveLength(3);
  });

  it('shows the indicative disclaimer with the fixture date', () => {
    renderModal(100);

    expect(screen.getByTestId('card-fx-matrix-disclaimer')).toHaveTextContent(
      `indicative: true · fixture rates dated ${CARD_FX_MATRIX_FIXTURE_DATE}`,
    );
  });

  it.each([0, -50])('shows zero amounts and a hint for %p USDC', (amount) => {
    renderModal(amount);

    expect(screen.getByTestId('card-fx-matrix-empty')).toHaveTextContent(
      'Enter an amount greater than 0 USDC to compare currencies.',
    );
    expect(screen.getByTestId('card-fx-matrix-amount-JPY')).toHaveTextContent('0');
    expect(screen.getByTestId('card-fx-matrix-amount-EUR')).toHaveTextContent(
      '0.00',
    );
  });

  it('hides the hint when the amount is positive', () => {
    renderModal(10);

    expect(screen.queryByTestId('card-fx-matrix-empty')).not.toBeInTheDocument();
  });

  it('closes on Escape', () => {
    const onOpenChange = renderModal(100);

    fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });

    expect(onOpenChange).toHaveBeenCalledWith(false);
  });
});
