import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  fireEvent,
  render,
  screen,
  waitFor,
  cleanup,
} from '@testing-library/react';

import { useWallet } from '@/hooks/useWallet';
import { loadCardNickname, saveCardNickname } from '@/lib/card/nickname';
import { CardPageClient } from './CardPageClient';

vi.mock('@/hooks/useWallet', () => ({
  useWallet: vi.fn(),
}));

const ADDRESS = 'GABCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUV2';
const OTHER_ADDRESS = 'GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB2';

function walletFor(address: string) {
  return {
    session: {
      isConnected: true,
      address,
      network: 'testnet' as const,
      walletId: 'freighter' as const,
    },
    availableWallets: [],
    loading: false,
    error: null,
    shortAddress: `${address.slice(0, 4)}...${address.slice(-4)}`,
    connect: vi.fn(async () => {}),
    disconnect: vi.fn(),
    copyAddress: vi.fn(async () => {}),
  };
}

beforeEach(() => {
  window.localStorage.clear();
  vi.mocked(useWallet).mockReturnValue(walletFor(ADDRESS));
  vi.stubGlobal(
    'fetch',
    vi.fn().mockResolvedValue({
      ok: true,
      json: async () => [],
    })
  );
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  window.localStorage.clear();
});

describe('CardPageClient', () => {
  it('lets the connected cardholder edit and persist the displayed nickname', async () => {
    saveCardNickname(ADDRESS, 'Travel');
    render(<CardPageClient />);

    expect(await screen.findByTestId('card-nickname-title')).toHaveTextContent(
      'Travel'
    );
    fireEvent.click(screen.getByRole('button', { name: 'Edit card nickname' }));
    fireEvent.change(screen.getByRole('textbox', { name: 'Card nickname' }), {
      target: { value: 'Subscriptions' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Save card nickname' }));

    expect(screen.getByTestId('card-nickname-title')).toHaveTextContent(
      'Subscriptions'
    );
    expect(loadCardNickname(ADDRESS)).toBe('Subscriptions');
    await waitFor(() =>
      expect(screen.queryByText('Loading card activity…')).toBeNull()
    );
  });

  it('loads the nickname for the newly connected Stellar address', async () => {
    saveCardNickname(ADDRESS, 'Travel');
    saveCardNickname(OTHER_ADDRESS, 'Subscriptions');
    const { rerender } = render(<CardPageClient />);

    expect(screen.getByTestId('card-nickname-title')).toHaveTextContent(
      'Travel'
    );
    vi.mocked(useWallet).mockReturnValue(walletFor(OTHER_ADDRESS));
    rerender(<CardPageClient />);

    expect(screen.getByTestId('card-nickname-title')).toHaveTextContent(
      'Subscriptions'
    );
    await waitFor(() =>
      expect(screen.queryByText('Loading card activity…')).toBeNull()
    );
  });
});
