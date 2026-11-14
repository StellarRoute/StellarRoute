import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { CardCategoryBreakdown } from './CardCategoryBreakdown';

describe('CardCategoryBreakdown', () => {
  it('renders empty state', () => {
    render(<CardCategoryBreakdown items={[]} />);
    expect(screen.getByTestId('category-empty')).toBeTruthy();
  });

  it('renders bar and legend for settled debits', () => {
    render(
      <CardCategoryBreakdown
        items={[
          {
            type: 'pos_debit',
            usdcAmount: -40,
            mcc: 5812,
            status: 'settled',
          },
          {
            type: 'pos_debit',
            usdcAmount: -60,
            mcc: 5411,
            status: 'settled',
          },
        ]}
      />,
    );
    expect(screen.getByTestId('category-bar')).toBeTruthy();
    expect(screen.getByTestId('segment-Dining')).toBeTruthy();
    expect(screen.getByTestId('segment-Groceries')).toBeTruthy();
  });

  it('segment title includes USDC and count (hover/focus)', () => {
    render(
      <CardCategoryBreakdown
        items={[
          {
            type: 'pos_debit',
            usdcAmount: -10,
            mcc: 5812,
            status: 'settled',
          },
        ]}
      />,
    );
    const seg = screen.getByTestId('segment-Dining');
    expect(seg.getAttribute('title')).toMatch(/USDC/);
    expect(seg.getAttribute('title')).toMatch(/tx/);
  });
});