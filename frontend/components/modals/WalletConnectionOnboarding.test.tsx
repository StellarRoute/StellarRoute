import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { WalletConnectionOnboarding } from './WalletConnectionOnboarding';

vi.mock('@/lib/network-policy', async () => {
  const actual = await vi.importActual<typeof import('@/lib/network-policy')>(
    '@/lib/network-policy',
  );
  return {
    ...actual,
    getAllowedNetworks: vi.fn(() => ['testnet', 'mainnet']),
    isNetworkAllowed: vi.fn(() => true),
  };
});

describe('WalletConnectionOnboarding', () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('shows network selection when multiple networks are allowed', async () => {
    const user = userEvent.setup();
    const onNetworkSelection = vi.fn();

    render(
      <WalletConnectionOnboarding
        open
        onOpenChange={vi.fn()}
        availableWallets={[]}
        isLoading={false}
        error={null}
        onConnect={vi.fn()}
        appNetwork="testnet"
        walletNetwork={null}
        onNetworkSelection={onNetworkSelection}
      />,
    );

    await user.click(screen.getByRole('button', { name: /continue/i }));

    expect(screen.getByRole('heading', { name: /select network/i })).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: /mainnet/i }));
    expect(onNetworkSelection).toHaveBeenCalledWith('mainnet');
  });

  it('wallet connect dialog uses mobile clip-safe layout classes (#1006)', () => {
    render(
      <WalletConnectionOnboarding
        open
        onOpenChange={vi.fn()}
        availableWallets={[]}
        isLoading={false}
        error={null}
        onConnect={vi.fn()}
        appNetwork="testnet"
        walletNetwork={null}
      />,
    );

    const dialog = screen.getByTestId('wallet-connect-dialog');
    expect(dialog.className).toMatch(/90vw/);
    expect(dialog.className).toMatch(/90dvh|90vh/);
    expect(dialog.className).toMatch(/overflow-hidden/);
  });
});
