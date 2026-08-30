import { describe, expect, it, vi, beforeEach } from 'vitest';
import {
  patchCctpSessionRecovery,
  setPendingEvmTx,
  loadCctpSession,
  buildCctpSessionRecord,
  clearCctpSession,
  sessionRequiresBindingRecovery,
} from './session-vault';
import { buildWalletRoleBindings } from './wallet-role-binding';

function defaultBindings() {
  return buildWalletRoleBindings({
    direction: 'evm_to_stellar',
    sourceChainId: 'eip155:11155111',
    destChainId: 'stellar:testnet',
    sender: '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0',
    recipient: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
    mintSubmitter: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
  });
}

describe('session-vault pending EVM tx', () => {
  beforeEach(() => {
    sessionStorage.clear();
  });

  it('persists pending tx hash without access token in recovery', () => {
    const record = buildCctpSessionRecord({
      transferId: 't1',
      accessToken: 'secret-token',
      idempotencyKey: 'k1',
      recovery: {
        corridorId: 'c1',
        direction: 'evm_to_stellar',
        sourceChainId: 'ethereum-sepolia',
        destChainId: 'stellar',
        amount: '10',
        recipient: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
        walletBindings: defaultBindings() ?? undefined,
      },
    });
    save(record);
    setPendingEvmTx({ txHash: '0xabc', purpose: 'burn' });
    const loaded = loadCctpSession();
    expect(loaded.ok).toBe(true);
    if (loaded.ok) {
      expect(loaded.record.recovery.pendingEvmTx?.txHash).toBe('0xabc');
      expect(loaded.record.recovery).not.toHaveProperty('accessToken');
    }
  });

  it('clears expired pending tx on load', () => {
    const record = buildCctpSessionRecord({
      transferId: 't1',
      accessToken: 'secret-token',
      idempotencyKey: 'k1',
      recovery: {
        corridorId: 'c1',
        direction: 'evm_to_stellar',
        sourceChainId: 'ethereum-sepolia',
        destChainId: 'stellar',
        amount: '10',
        recipient: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
        pendingEvmTx: {
          txHash: '0xold',
          purpose: 'burn',
          expiresAt: Date.now() - 1000,
        },
        walletBindings: defaultBindings() ?? undefined,
      },
    });
    save(record);
    const loaded = loadCctpSession();
    expect(loaded.ok).toBe(true);
    if (loaded.ok) {
      expect(loaded.record.recovery.pendingEvmTx).toBeUndefined();
    }
  });
});

function save(record: ReturnType<typeof buildCctpSessionRecord>) {
  sessionStorage.setItem('stellarroute:cctp:v1', JSON.stringify(record));
}

describe('patchCctpSessionRecovery', () => {
  beforeEach(() => {
    clearCctpSession();
    sessionStorage.clear();
  });

  it('updates burn prepare step', () => {
    const record = buildCctpSessionRecord({
      transferId: 't1',
      accessToken: 'tok',
      idempotencyKey: 'k1',
      recovery: {
        corridorId: 'c1',
        direction: 'stellar_to_evm',
        sourceChainId: 'stellar',
        destChainId: 'ethereum-sepolia',
        amount: '5',
        recipient: '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0',
        walletBindings:
          buildWalletRoleBindings({
            direction: 'stellar_to_evm',
            sourceChainId: 'stellar:testnet',
            destChainId: 'eip155:11155111',
            sender: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
            recipient: '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0',
          }) ?? undefined,
      },
    });
    save(record);
    const patched = patchCctpSessionRecovery({ burnPrepareStep: 'approval_ready' });
    expect(patched?.recovery.burnPrepareStep).toBe('approval_ready');
  });
});

describe('session-vault wallet binding migration', () => {
  beforeEach(() => {
    clearCctpSession();
    sessionStorage.clear();
  });

  it('v1 sessions load but require binding recovery', () => {
    const record = buildCctpSessionRecord({
      transferId: 't1',
      accessToken: 'tok',
      idempotencyKey: 'k1',
      recovery: {
        corridorId: 'c1',
        direction: 'evm_to_stellar',
        sourceChainId: 'ethereum-sepolia',
        destChainId: 'stellar',
        amount: '10',
        recipient: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
        walletBindings: defaultBindings() ?? undefined,
      },
    });
    record.version = 1;
    save(record);
    const loaded = loadCctpSession();
    expect(loaded.ok).toBe(true);
    if (loaded.ok) {
      expect(sessionRequiresBindingRecovery(loaded.record)).toBe(true);
    }
  });

  it('v2 sessions persist wallet bindings', () => {
    const bindings = buildWalletRoleBindings({
      direction: 'evm_to_stellar',
      sourceChainId: 'eip155:11155111',
      destChainId: 'stellar:testnet',
      sender: '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0',
      recipient: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
      mintSubmitter: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
    });
    const record = buildCctpSessionRecord({
      transferId: 't2',
      accessToken: 'tok',
      idempotencyKey: 'k2',
      recovery: {
        corridorId: 'c1',
        direction: 'evm_to_stellar',
        sourceChainId: 'ethereum-sepolia',
        destChainId: 'stellar',
        amount: '10',
        recipient: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
        walletBindings: bindings ?? undefined,
      },
    });
    save(record);
    const loaded = loadCctpSession();
    expect(loaded.ok).toBe(true);
    if (loaded.ok) {
      expect(sessionRequiresBindingRecovery(loaded.record)).toBe(false);
      expect(loaded.record.recovery.walletBindings?.sourceBurn.address).toBe(
        '0x742d35cc6634c0532925a3b844bc9e7595f0beb0',
      );
    }
  });
});
