import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clearAdaptersForTests, getAdapter } from '../registry';
import { WalletAdapterError } from '../errors';
import { createInjectedEvmAdapter } from './injected';

type MockProvider = {
  request: ReturnType<typeof vi.fn>;
  isMetaMask?: boolean;
};

function installProvider(provider: MockProvider | null) {
  const win = window as unknown as Record<string, unknown>;
  if (provider) {
    win.ethereum = provider;
  } else {
    delete win.ethereum;
  }
}

describe('injected EVM adapter', () => {
  beforeEach(() => {
    clearAdaptersForTests();
    installProvider(null);
  });

  afterEach(() => {
    installProvider(null);
    clearAdaptersForTests();
  });

  it('detects missing provider as not installed', async () => {
    const adapter = createInjectedEvmAdapter();
    await expect(adapter.detectInstalled()).resolves.toBe(false);
  });

  it('connects via eth_requestAccounts and reports CAIP-2 network', async () => {
    const request = vi.fn(async ({ method }: { method: string }) => {
      if (method === 'eth_requestAccounts') return ['0xabc'];
      if (method === 'eth_chainId') return '0xaa36a7'; // sepolia
      if (method === 'eth_accounts') return ['0xabc'];
      return null;
    });
    installProvider({ request, isMetaMask: true });

    const adapter = createInjectedEvmAdapter();
    const session = await adapter.connect('eip155:11155111');

    expect(session).toMatchObject({
      adapterId: 'evm-injected',
      chainFamily: 'evm',
      account: { address: '0xabc' },
      network: 'eip155:11155111',
      isConnected: true,
    });
  });

  it('soft-connects on mismatch and attempts switch without failing connect', async () => {
    const chainId = '0x1';
    const request = vi.fn(async ({ method }: { method: string }) => {
      if (method === 'eth_requestAccounts') return ['0xabc'];
      if (method === 'eth_accounts') return ['0xabc'];
      if (method === 'eth_chainId') return chainId;
      if (method === 'wallet_switchEthereumChain') {
        throw Object.assign(new Error('Unrecognized chain'), { code: 4902 });
      }
      return null;
    });
    installProvider({ request });

    const adapter = createInjectedEvmAdapter();
    const session = await adapter.connect('eip155:11155111');
    expect(session.isConnected).toBe(true);
    expect(session.network).toBe('eip155:1');
    expect((await adapter.getNetwork('eip155:11155111')).matchesExpected).toBe(
      false
    );
    expect(adapter.getExecutionSupport().code).toBe('network_mismatch');
  });

  it('sends transactions through eth_sendTransaction', async () => {
    const request = vi.fn(async ({ method }: { method: string }) => {
      if (method === 'eth_requestAccounts') return ['0xabc'];
      if (method === 'eth_accounts') return ['0xabc'];
      if (method === 'eth_chainId') return '0xaa36a7';
      if (method === 'eth_sendTransaction') return '0xdead';
      return null;
    });
    installProvider({ request });

    const adapter = createInjectedEvmAdapter();
    await adapter.connect('eip155:11155111');
    const result = await adapter.sendTransaction?.({
      kind: 'evm_transaction',
      transaction: { to: '0xdef', value: '0x1' },
    });

    expect(result).toEqual({ kind: 'evm_transaction', hash: '0xdead' });
    expect(request).toHaveBeenCalledWith({
      method: 'eth_sendTransaction',
      params: [{ to: '0xdef', value: '0x1', from: '0xabc' }],
    });
  });

  it('normalizes user rejection on personal_sign', async () => {
    const request = vi.fn(async ({ method }: { method: string }) => {
      if (method === 'eth_requestAccounts') return ['0xabc'];
      if (method === 'eth_accounts') return ['0xabc'];
      if (method === 'eth_chainId') return '0xaa36a7';
      if (method === 'personal_sign') {
        const err = new Error('User rejected the request');
        (err as { code?: number }).code = 4001;
        throw err;
      }
      return null;
    });
    installProvider({ request });

    const adapter = createInjectedEvmAdapter();
    await adapter.connect('eip155:11155111');
    await expect(
      adapter.signMessage({ kind: 'message', message: 'hi' })
    ).rejects.toMatchObject({
      code: 'user_rejected',
    } satisfies Partial<WalletAdapterError>);
  });

  it('maps eth_signTransaction method-not-found to unsupported_capability', async () => {
    const request = vi.fn(async ({ method }: { method: string }) => {
      if (method === 'eth_requestAccounts') return ['0xabc'];
      if (method === 'eth_accounts') return ['0xabc'];
      if (method === 'eth_chainId') return '0xaa36a7';
      if (method === 'eth_signTransaction') {
        throw Object.assign(new Error('Method not found'), { code: -32601 });
      }
      return null;
    });
    installProvider({ request });

    const adapter = createInjectedEvmAdapter();
    await adapter.connect('eip155:11155111');
    await expect(
      adapter.signTransaction({
        kind: 'evm_transaction',
        transaction: { to: '0x1' },
      })
    ).rejects.toMatchObject({ code: 'unsupported_capability' });
  });

  it('preserves non-method RPC errors from eth_signTransaction', async () => {
    const request = vi.fn(async ({ method }: { method: string }) => {
      if (method === 'eth_requestAccounts') return ['0xabc'];
      if (method === 'eth_accounts') return ['0xabc'];
      if (method === 'eth_chainId') return '0xaa36a7';
      if (method === 'eth_signTransaction') {
        throw new Error('nonce too low');
      }
      return null;
    });
    installProvider({ request });

    const adapter = createInjectedEvmAdapter();
    await adapter.connect('eip155:11155111');
    await expect(
      adapter.signTransaction({
        kind: 'evm_transaction',
        transaction: { to: '0x1' },
      })
    ).rejects.toMatchObject({
      code: 'provider_error',
      message: 'nonce too low',
    });
  });

  it('registers in the default adapter registry', () => {
    const adapter = getAdapter('evm-injected');
    expect(adapter?.chainFamily).toBe('evm');
  });

  it('reports not_connected until a live session exists', () => {
    const adapter = createInjectedEvmAdapter();
    expect(
      adapter.getExecutionSupport({
        sourceChain: 'evm',
        destinationChain: 'stellar',
      }).code
    ).toBe('not_connected');
  });

  it('reports no_backend_route after connect when signing is available', async () => {
    const request = vi.fn(async ({ method }: { method: string }) => {
      if (method === 'eth_requestAccounts') return ['0xabc'];
      if (method === 'eth_accounts') return ['0xabc'];
      if (method === 'eth_chainId') return '0xaa36a7';
      return null;
    });
    installProvider({ request });
    const adapter = createInjectedEvmAdapter();
    await adapter.connect('eip155:11155111');
    expect(
      adapter.getExecutionSupport({
        sourceChain: 'evm',
        destinationChain: 'stellar',
      }).code
    ).toBe('no_backend_route');
  });
});
