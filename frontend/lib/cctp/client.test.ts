import { describe, expect, it, vi, afterEach } from 'vitest';
import { CctpApiClient, resetCctpApiClientForTests } from './client';
import {
  buildCctpSessionRecord,
  clearCctpSession,
  loadCctpSession,
  redactSecretsForLogs,
  saveCctpSession,
} from './session-vault';
import { buildWalletRoleBindings } from './wallet-role-binding';
import { CCTP_IDEMPOTENCY_HEADER, CCTP_TRANSFER_ACCESS_HEADER } from './types';

describe('CctpApiClient', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    resetCctpApiClientForTests();
  });

  it('sends idempotency and access headers on quote and transfer', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            data: {
              transfer_id: 't1',
              access_token: 'secret-token',
              corridor_id: 'c',
              provider: 'circle-cctp',
              direction: 'evm_to_stellar',
              source_amount: '1',
              destination_amount: '1',
              fee_quote: {},
              expires_at: 9999999999,
              finality: 'standard',
            },
          }),
          { status: 200, headers: { 'Content-Type': 'application/json' } },
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            data: {
              transfer_id: 't1',
              status: 'created',
              corridor_id: 'c',
              provider: 'circle-cctp',
              direction: 'evm_to_stellar',
              retryable: false,
            },
          }),
          { status: 200, headers: { 'Content-Type': 'application/json' } },
        ),
      );
    vi.stubGlobal('fetch', fetchMock);

    const client = new CctpApiClient('http://api.test');
    await client.quote(
      {
        corridor_id: 'c',
        provider: 'circle-cctp',
        direction: 'evm_to_stellar',
        source_chain_id: 'eip155:11155111',
        destination_chain_id: 'stellar:testnet',
        source_asset: {
          chain_id: 'eip155:11155111',
          asset: 'a',
          canonical: 'a',
        },
        destination_asset: {
          chain_id: 'stellar:testnet',
          asset: 'b',
          canonical: 'b',
        },
        amount: '10',
        recipient: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
        finality: 'standard',
      },
      { idempotencyKey: 'idem-1' },
    );
    await client.getTransfer('t1', { accessToken: 'secret-token' });

    const quoteHeaders = (fetchMock.mock.calls[0][1] as RequestInit)
      .headers as Record<string, string>;
    expect(quoteHeaders[CCTP_IDEMPOTENCY_HEADER.toLowerCase()] ?? quoteHeaders['Idempotency-Key'] ?? quoteHeaders['idempotency-key']).toBe('idem-1');

    const statusHeaders = (fetchMock.mock.calls[1][1] as RequestInit)
      .headers as Record<string, string>;
    expect(statusHeaders[CCTP_TRANSFER_ACCESS_HEADER]).toBe('secret-token');
    expect(JSON.stringify(fetchMock.mock.results)).not.toContain('secret-token');
  });

  it('retries quote on HTTP 425 with same idempotency key', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ data: { error: 'in_progress', message: 'wait' } }), {
          status: 425,
          headers: { 'Content-Type': 'application/json' },
        }),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            data: {
              transfer_id: 't1',
              access_token: 'tok',
              corridor_id: 'c',
              provider: 'circle-cctp',
              direction: 'evm_to_stellar',
              source_amount: '1',
              destination_amount: '1',
              fee_quote: {},
              expires_at: 9999999999,
              finality: 'standard',
            },
          }),
          { status: 200, headers: { 'Content-Type': 'application/json' } },
        ),
      );
    vi.stubGlobal('fetch', fetchMock);

    const client = new CctpApiClient('http://api.test');
    const result = await client.quote(
      {
        corridor_id: 'c',
        provider: 'circle-cctp',
        direction: 'evm_to_stellar',
        source_chain_id: 'eip155:11155111',
        destination_chain_id: 'stellar:testnet',
        source_asset: { chain_id: 'eip155:11155111', asset: 'a', canonical: 'a' },
        destination_asset: { chain_id: 'stellar:testnet', asset: 'b', canonical: 'b' },
        amount: '1',
        recipient: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
        finality: 'standard',
      },
      { idempotencyKey: 'idem-retry' },
    );
    expect(result.transfer_id).toBe('t1');
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });
});

describe('session vault', () => {
  afterEach(() => {
    clearCctpSession();
  });

  it('stores token only in sessionStorage and redacts logs', () => {
    const record = buildCctpSessionRecord({
      transferId: 't1',
      accessToken: 'super-secret',
      idempotencyKey: 'idem',
      recovery: {
        corridorId: 'c',
        direction: 'evm_to_stellar',
        sourceChainId: 'ethereum-sepolia',
        destChainId: 'stellar',
        amount: '1',
        recipient: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
        walletBindings:
          buildWalletRoleBindings({
            direction: 'evm_to_stellar',
            sourceChainId: 'eip155:11155111',
            destChainId: 'stellar:testnet',
            sender: '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0',
            recipient: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
            mintSubmitter: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
          }) ?? undefined,
      },
    });
    saveCctpSession(record);
    const loaded = loadCctpSession();
    expect(loaded.ok).toBe(true);
    if (loaded.ok) {
      expect(loaded.record.accessToken).toBe('super-secret');
    }
    expect(redactSecretsForLogs(record)).not.toContain('super-secret');
  });
});
