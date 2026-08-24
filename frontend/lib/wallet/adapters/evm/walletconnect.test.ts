import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clearAdaptersForTests, getAdapter } from '../registry';
import { WalletAdapterError } from '../errors';
import {
  createWalletConnectEvmAdapter,
  resetWalletConnectProviderForTests,
} from './walletconnect';

const connect = vi.fn(async (_opts?: unknown) => undefined);
const disconnect = vi.fn(async () => undefined);
const request = vi.fn(async ({ method }: { method: string }) => {
  if (method === 'eth_requestAccounts' || method === 'eth_accounts') {
    return ['0xwc'];
  }
  if (method === 'eth_chainId') return '0xaa36a7';
  if (method === 'eth_sendTransaction') return '0xhash';
  return null;
});

const providerState = {
  accounts: [] as string[],
  session: undefined as { topic: string } | undefined,
};

vi.mock('@walletconnect/ethereum-provider', () => ({
  EthereumProvider: {
    init: vi.fn(async () => ({
      get accounts() {
        return providerState.accounts;
      },
      get session() {
        return providerState.session;
      },
      connect: async (opts?: unknown) => {
        await connect(opts);
        providerState.accounts = ['0xwc'];
        providerState.session = { topic: 't' };
      },
      disconnect: async () => {
        await disconnect();
        providerState.accounts = [];
        providerState.session = undefined;
      },
      request,
    })),
  },
}));

describe('WalletConnect EVM adapter', () => {
  beforeEach(() => {
    clearAdaptersForTests();
    resetWalletConnectProviderForTests();
    providerState.accounts = [];
    providerState.session = undefined;
    connect.mockClear();
    disconnect.mockClear();
    request.mockClear();
    vi.stubEnv('NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID', 'test-project-id');
  });

  afterEach(() => {
    resetWalletConnectProviderForTests();
    clearAdaptersForTests();
    vi.unstubAllEnvs();
  });

  it('reports installed when project id is configured', async () => {
    const adapter = createWalletConnectEvmAdapter();
    await expect(adapter.detectInstalled()).resolves.toBe(true);
  });

  it('reports not installed without project id', async () => {
    vi.stubEnv('NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID', '');
    const adapter = createWalletConnectEvmAdapter();
    await expect(adapter.detectInstalled()).resolves.toBe(false);
    await expect(adapter.connect('eip155:11155111')).rejects.toBeInstanceOf(
      WalletAdapterError
    );
  });

  it('connects via WalletConnect provider and returns CAIP-2 network', async () => {
    const adapter = createWalletConnectEvmAdapter();
    const session = await adapter.connect('eip155:11155111');

    expect(connect).toHaveBeenCalled();
    expect(session).toMatchObject({
      adapterId: 'evm-walletconnect',
      chainFamily: 'evm',
      account: { address: '0xwc' },
      network: 'eip155:11155111',
      isConnected: true,
    });
  });

  it('reuses a persisted WalletConnect session without opening QR again', async () => {
    providerState.accounts = ['0xwc'];
    providerState.session = { topic: 'existing' };
    const adapter = createWalletConnectEvmAdapter();
    const session = await adapter.connect('eip155:11155111');
    expect(connect).not.toHaveBeenCalled();
    expect(session.account.address).toBe('0xwc');
  });

  it('sends transactions through the WC EIP-1193 provider', async () => {
    const adapter = createWalletConnectEvmAdapter();
    await adapter.connect('eip155:11155111');
    const result = await adapter.sendTransaction?.({
      kind: 'evm_transaction',
      transaction: { to: '0xabc', value: '0x0' },
    });
    expect(result).toEqual({ kind: 'evm_transaction', hash: '0xhash' });
    expect(request).toHaveBeenCalledWith({
      method: 'eth_sendTransaction',
      params: [expect.objectContaining({ to: '0xabc', from: '0xwc' })],
    });
  });

  it('disconnects the WalletConnect session', async () => {
    const adapter = createWalletConnectEvmAdapter();
    await adapter.connect('eip155:11155111');
    await adapter.disconnect();
    expect(disconnect).toHaveBeenCalled();
  });

  it('is registered as evm-walletconnect by default', () => {
    expect(getAdapter('evm-walletconnect')?.label).toBe('WalletConnect');
  });
});
