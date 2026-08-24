import { describe, expect, it } from 'vitest';
import {
  assertSepoliaCaip,
  caip2EvmToChainIdHex,
  caip2FromChainIdHex,
} from './caip-evm';

describe('caip-evm', () => {
  it('converts Sepolia CAIP-2 to EIP-1193 hex', () => {
    const result = caip2EvmToChainIdHex('eip155:11155111');
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.chainIdHex).toBe('0xaa36a7');
    }
  });

  it('rejects non-eip155 namespaces', () => {
    const result = caip2EvmToChainIdHex('cosmos:osmosis-1');
    expect(result.ok).toBe(false);
  });

  it('rejects overflow references', () => {
    const result = caip2EvmToChainIdHex(`eip155:${'9'.repeat(400)}`);
    expect(result.ok).toBe(false);
  });

  it('assertSepoliaCaip enforces testnet reference', () => {
    expect(assertSepoliaCaip('eip155:1').ok).toBe(false);
    expect(assertSepoliaCaip('eip155:11155111').ok).toBe(true);
  });

  it('round-trips hex to CAIP-2', () => {
    expect(caip2FromChainIdHex('0xaa36a7')).toBe('eip155:11155111');
  });
});
