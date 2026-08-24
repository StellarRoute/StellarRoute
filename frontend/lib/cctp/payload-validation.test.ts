import { describe, expect, it } from 'vitest';
import { validatePreparedPayload } from './payload-validation';
import { SEPOLIA_CHAIN_ID } from './types';
import { SEPOLIA_CCTP_CONTRACTS } from './constants';

const baseEvmPayload = {
  type: 'evm_transaction' as const,
  chain_id: SEPOLIA_CHAIN_ID,
  to: SEPOLIA_CCTP_CONTRACTS.usdc,
  data: '0x',
  value: '0',
};

describe('validatePreparedPayload EVM bounds', () => {
  it('rejects non-zero native value', () => {
    const result = validatePreparedPayload({
      ...baseEvmPayload,
      value: '1',
    });
    expect(result.ok).toBe(false);
  });

  it('rejects oversized calldata', () => {
    const result = validatePreparedPayload({
      ...baseEvmPayload,
      data: `0x${'ab'.repeat(25_000)}`,
    });
    expect(result.ok).toBe(false);
  });

  it('rejects gas above upper bound', () => {
    const result = validatePreparedPayload({
      ...baseEvmPayload,
      gas: '3000000',
    });
    expect(result.ok).toBe(false);
  });

  it('accepts valid Sepolia payload', () => {
    const result = validatePreparedPayload(baseEvmPayload);
    expect(result.ok).toBe(true);
  });
});
