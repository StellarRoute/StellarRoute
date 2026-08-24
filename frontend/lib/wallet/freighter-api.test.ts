import { describe, expect, it } from 'vitest';
import { resolveFreighterApi } from './freighter-api';

function makeApi() {
  return {
    isConnected: async () => ({ isConnected: true }),
    requestAccess: async () => ({ address: 'GTEST' }),
    getAddress: async () => ({ address: 'GTEST' }),
    getNetworkDetails: async () => ({ network: 'TESTNET' }),
    signTransaction: async () => ({ signedTxXdr: 'AAAA' }),
  };
}

describe('resolveFreighterApi', () => {
  it('accepts direct named-export module shapes', () => {
    const api = makeApi();
    expect(resolveFreighterApi(api).requestAccess).toBe(api.requestAccess);
  });

  it('unwraps default export used by UMD/ESM interop', () => {
    const api = makeApi();
    const resolved = resolveFreighterApi({ default: api });
    expect(resolved.requestAccess).toBe(api.requestAccess);
  });

  it('unwraps module.exports key produced by some bundlers', () => {
    const api = makeApi();
    const resolved = resolveFreighterApi({ 'module.exports': api });
    expect(resolved.signTransaction).toBe(api.signTransaction);
  });

  it('throws a clear error when no callable API is present', () => {
    expect(() => resolveFreighterApi({ requestAccess: undefined })).toThrow(
      /Freighter API failed to load/,
    );
  });
});
