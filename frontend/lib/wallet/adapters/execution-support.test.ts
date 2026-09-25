import { describe, expect, it } from 'vitest';
import {
  hasBackendRoute,
  resolveExecutionSupport,
} from './execution-support';

describe('execution support', () => {
  describe('stellar→stellar live state', () => {
    const stellarRoute = {
      sourceChain: 'stellar' as const,
      destinationChain: 'stellar' as const,
    };

    it('reports stellar_native when connected, matched, and able to sign', () => {
      const support = resolveExecutionSupport('stellar', stellarRoute, {
        connected: true,
        networkMatch: true,
        canSign: true,
      });
      expect(support).toMatchObject({
        kind: 'supported',
        code: 'stellar_native',
      });
    });

    it('reports not_connected when disconnected', () => {
      const support = resolveExecutionSupport('stellar', stellarRoute, {
        connected: false,
        networkMatch: true,
        canSign: true,
      });
      expect(support).toMatchObject({
        kind: 'unsupported',
        code: 'not_connected',
      });
    });

    it('reports network_mismatch when connected on the wrong network', () => {
      const support = resolveExecutionSupport('stellar', stellarRoute, {
        connected: true,
        networkMatch: false,
        canSign: true,
      });
      expect(support).toMatchObject({
        kind: 'degraded',
        code: 'network_mismatch',
      });
    });

    it('reports wallet_capability_missing when connected but cannot sign', () => {
      const support = resolveExecutionSupport('stellar', stellarRoute, {
        connected: true,
        networkMatch: true,
        canSign: false,
      });
      expect(support).toMatchObject({
        kind: 'degraded',
        code: 'wallet_capability_missing',
      });
    });
  });

  it('reports no backend route for non-stellar pairs even when signing is available', () => {
    expect(hasBackendRoute('stellar', 'evm')).toBe(true);
    expect(hasBackendRoute('evm', 'stellar')).toBe(true);
    expect(hasBackendRoute('bitcoin', 'stellar')).toBe(false);
    expect(hasBackendRoute('tron', 'bitcoin')).toBe(false);
    expect(hasBackendRoute('solana', 'solana')).toBe(false);

    for (const family of ['solana', 'bitcoin', 'tron'] as const) {
      const support = resolveExecutionSupport(
        family,
        { sourceChain: family, destinationChain: 'stellar' },
        { connected: true, networkMatch: true, canSign: true }
      );
      expect(support).toMatchObject({
        kind: 'unsupported',
        code: 'no_backend_route',
      });
    }

    const evmSupport = resolveExecutionSupport(
      'evm',
      { sourceChain: 'evm', destinationChain: 'stellar' },
      { connected: true, networkMatch: true, canSign: true }
    );
    expect(evmSupport).toMatchObject({
      kind: 'signing_only',
      code: 'chain_signing_available',
    });

    // Same-chain non-Stellar still has no backend swap route.
    const tronSame = resolveExecutionSupport(
      'tron',
      { sourceChain: 'tron', destinationChain: 'tron' },
      { connected: true, networkMatch: true, canSign: true }
    );
    expect(tronSame).toMatchObject({
      kind: 'unsupported',
      code: 'no_backend_route',
    });
  });

  it('reports not_connected when disconnected', () => {
    const support = resolveExecutionSupport(
      'solana',
      { sourceChain: 'solana', destinationChain: 'solana' },
      { connected: false }
    );
    expect(support.code).toBe('not_connected');
  });

  it('prefers network mismatch over generic no-route when both apply', () => {
    const support = resolveExecutionSupport(
      'bitcoin',
      { sourceChain: 'bitcoin', destinationChain: 'stellar' },
      { connected: true, networkMatch: false, canSign: true }
    );
    expect(support).toMatchObject({
      kind: 'degraded',
      code: 'network_mismatch',
    });
  });

  it('prefers capability gaps over generic no-route when both apply', () => {
    const support = resolveExecutionSupport(
      'evm',
      { sourceChain: 'evm', destinationChain: 'evm' },
      { connected: true, networkMatch: true, canSign: false }
    );
    expect(support).toMatchObject({
      kind: 'degraded',
      code: 'wallet_capability_missing',
    });
  });
});
