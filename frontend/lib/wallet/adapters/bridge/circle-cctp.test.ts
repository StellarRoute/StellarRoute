import { describe, expect, it, vi, afterEach } from 'vitest';
import { createCircleCctpBridgeProvider } from './circle-cctp';
import { resetReadinessForTests, applyReadinessCorridors } from '@/lib/cctp/readiness';
import { WalletAdapterError } from '../errors';

describe('circle-cctp bridge provider', () => {
  afterEach(() => {
    resetReadinessForTests();
    vi.restoreAllMocks();
  });

  const route = {
    sourceChain: 'evm' as const,
    destinationChain: 'stellar' as const,
  };

  it('reports unsupported when corridor not executable on API', () => {
    const provider = createCircleCctpBridgeProvider();
    const availability = provider.getAvailability(route);
    expect(availability.kind).toBe('unsupported');
  });

  it('reports supported when /api/v2 marks corridor executable', () => {
    applyReadinessCorridors([
      {
        corridor_id: 'c',
        provider: 'circle-cctp',
        direction: 'evm_to_stellar',
        source_chain_id: 'eip155:11155111',
        destination_chain_id: 'stellar:testnet',
        source_asset: { chain_id: 'eip155:11155111', asset: 'a', canonical: 'a' },
        destination_asset: { chain_id: 'stellar:testnet', asset: 'b', canonical: 'b' },
        executable: true,
      },
    ]);
    const provider = createCircleCctpBridgeProvider();
    expect(provider.getAvailability(route).kind).toBe('supported');
  });

  it('quote calls client with idempotency when executable', async () => {
    applyReadinessCorridors([
      {
        corridor_id: 'c',
        provider: 'circle-cctp',
        direction: 'evm_to_stellar',
        source_chain_id: 'eip155:11155111',
        destination_chain_id: 'stellar:testnet',
        source_asset: { chain_id: 'eip155:11155111', asset: 'a', canonical: 'a' },
        destination_asset: { chain_id: 'stellar:testnet', asset: 'b', canonical: 'b' },
        executable: true,
      },
    ]);
    const quote = vi.fn().mockResolvedValue({
      transfer_id: 't1',
      destination_amount: '9.9',
      expires_at: 9999999999,
      corridor_id: 'c',
      finality: 'standard',
    });
    const provider = createCircleCctpBridgeProvider(
      'circle-cctp',
      'Circle CCTP',
      { quote } as never,
    );
    const result = await provider.quote({
      ...route,
      amountIn: '10',
      recipient: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
    });
    expect(result.quoteId).toBe('t1');
    expect(quote).toHaveBeenCalled();
  });

  it('prepare throws — saga owns prepare with access token', async () => {
    const provider = createCircleCctpBridgeProvider();
    await expect(
      provider.prepare({
        quoteId: 'q1',
        session: {
          adapterId: 'evm-injected',
          chainFamily: 'evm',
          account: { address: '0x1' },
          network: 'eip155:11155111',
          isConnected: true,
        },
      }),
    ).rejects.toBeInstanceOf(WalletAdapterError);
  });
});
