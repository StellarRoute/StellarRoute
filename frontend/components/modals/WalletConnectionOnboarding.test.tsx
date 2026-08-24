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

  it('lists LOBSTR and connects even when not yet detected', async () => {
    const user = userEvent.setup();
    const onConnect = vi.fn().mockResolvedValue(undefined);
    const onRefreshWallets = vi.fn().mockResolvedValue(undefined);

    render(
      <WalletConnectionOnboarding
        open
        onOpenChange={vi.fn()}
        availableWallets={[
          { id: 'freighter', label: 'Freighter', installed: false },
          { id: 'lobstr', label: 'LOBSTR', installed: false },
        ]}
        isLoading={false}
        error={null}
        onConnect={onConnect}
        appNetwork="testnet"
        walletNetwork={null}
        onRefreshWallets={onRefreshWallets}
      />,
    );

    await user.click(screen.getByRole('button', { name: /continue/i }));
    await user.click(screen.getByRole('button', { name: /testnet/i }));
    expect(screen.getByRole('heading', { name: /select your wallet/i })).toBeInTheDocument();
    expect(screen.getByText('LOBSTR')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /lobstr/i }));
    expect(onRefreshWallets).toHaveBeenCalled();
    expect(onConnect).toHaveBeenCalledWith('lobstr');
  });

  it('allows cancel while waiting for wallet approval', async () => {
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    let resolveConnect: (() => void) | undefined;
    const onConnect = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          resolveConnect = resolve;
        }),
    );

    render(
      <WalletConnectionOnboarding
        open
        onOpenChange={onOpenChange}
        availableWallets={[{ id: 'freighter', label: 'Freighter', installed: true }]}
        isLoading={false}
        error={null}
        onConnect={onConnect}
        appNetwork="testnet"
        walletNetwork={null}
      />,
    );

    await user.click(screen.getByRole('button', { name: /continue/i }));
    await user.click(screen.getByRole('button', { name: /testnet/i }));
    await user.click(screen.getByRole('button', { name: /freighter/i }));

    expect(screen.getByRole('heading', { name: /connecting freighter/i })).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: /^cancel$/i }));
    expect(screen.getByRole('heading', { name: /select your wallet/i })).toBeInTheDocument();

    resolveConnect?.();
  });

  it('closes the modal from the connecting step', async () => {
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    const onConnect = vi.fn(() => new Promise<void>(() => {}));

    render(
      <WalletConnectionOnboarding
        open
        onOpenChange={onOpenChange}
        availableWallets={[{ id: 'freighter', label: 'Freighter', installed: true }]}
        isLoading={false}
        error={null}
        onConnect={onConnect}
        appNetwork="testnet"
        walletNetwork={null}
      />,
    );

    await user.click(screen.getByRole('button', { name: /continue/i }));
    await user.click(screen.getByRole('button', { name: /testnet/i }));
    await user.click(screen.getByRole('button', { name: /freighter/i }));

    await user.click(screen.getByTestId('wallet-connect-dismiss'));
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });
});
