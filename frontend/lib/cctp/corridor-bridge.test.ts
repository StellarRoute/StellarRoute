import { describe, expect, it } from 'vitest';
import { buildCctpQuoteRequest } from './corridor-bridge';

describe('buildCctpQuoteRequest finality', () => {
  it('requests Fast for evm_to_stellar', () => {
    const body = buildCctpQuoteRequest({
      sourceChainId: 'ethereum-sepolia',
      destChainId: 'stellar',
      amount: '1',
      recipient: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
      sender: '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0',
      mintSubmitter: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
    });
    expect(body?.direction).toBe('evm_to_stellar');
    expect(body?.finality).toBe('fast');
  });

  it('requests Fast for stellar_to_evm', () => {
    const body = buildCctpQuoteRequest({
      sourceChainId: 'stellar',
      destChainId: 'ethereum-sepolia',
      amount: '1',
      recipient: '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0',
      sender: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
    });
    expect(body?.direction).toBe('stellar_to_evm');
    expect(body?.finality).toBe('fast');
  });
});
