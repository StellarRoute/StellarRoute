import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { CardActivityList } from './CardActivityList';
import type { CardActivityItem } from '../../lib/card/activity';

const items: CardActivityItem[] = [
  {
    id: 'd1',
    type: 'cctp_deposit',
    createdAt: '2026-10-02T12:00:00Z',
    usdcAmount: 40,
    sourceChain: 'ethereum',
    stellarTxHash: 'ABC',
    evmTxHash: '0x1',
  },
  {
    id: 'p1',
    type: 'pos_debit',
    createdAt: '2026-10-01T09:00:00Z',
    usdcAmount: -5,
    merchantName: 'Bookstore',
    fiatAmount: 4.5,
    fiatCurrency: 'EUR',
  },
];

describe('CardActivityList', () => {
  it('renders empty state', () => {
    render(<CardActivityList items={[]} />);
    expect(screen.getByTestId('card-activity-empty')).toBeTruthy();
  });

  it('shows CCTP as positive with chain badge', () => {
    render(<CardActivityList items={items} />);
    expect(screen.getByTestId('cctp-amount').textContent).toContain('+40.00');
    expect(screen.getByTestId('evm-chain-badge').textContent).toBe('ethereum');
    expect(screen.getByTestId('badge-incoming')).toBeTruthy();
  });

  it('shows POS merchant + fiat + usdc', () => {
    render(<CardActivityList items={items} />);
    expect(screen.getByTestId('merchant-name').textContent).toBe('Bookstore');
    expect(screen.getByTestId('fiat-breakdown').textContent).toContain('EUR');
    expect(screen.getByTestId('pos-usdc').textContent).toContain('-5.00');
  });

  it('links to Stellar Expert for CCTP', () => {
    render(<CardActivityList items={items} />);
    const link = screen.getByTestId('stellar-expert-link') as HTMLAnchorElement;
    expect(link.href).toContain('stellar.expert');
  });
});