import { describe, expect, it, vi, afterEach } from 'vitest';
import {
  StellarRouteClient,
  StellarRouteApiError,
  isStellarRouteApiError,
  parseApiErrorBody,
} from './client.js';
import {
  API_ERROR_CODES,
  CCTP_PROVIDER_ID,
  CCTP_TESTNET_CORRIDOR_ID,
  type CctpQuoteRequest,
} from './types.js';

const TRANSFER_ID = '550e8400-e29b-41d4-a716-446655440000';
const VALID_EVM_TX_HASH =
  '0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef';
const VALID_STELLAR_TX_HASH =
  '1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef';

const sampleQuoteRequest: CctpQuoteRequest = {
  corridor_id: CCTP_TESTNET_CORRIDOR_ID,
  provider: CCTP_PROVIDER_ID,
  direction: 'evm_to_stellar',
  source_chain_id: 'eip155:11155111',
  destination_chain_id: 'stellar:testnet',
  source_asset: {
    chain_id: 'eip155:11155111',
    asset: 'erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238',
    canonical:
      'eip155:11155111/erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238',
    symbol: 'USDC',
  },
  destination_asset: {
    chain_id: 'stellar:testnet',
    asset: 'erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA',
    canonical:
      'stellar:testnet/erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA',
    symbol: 'USDC',
  },
  amount: '100.000000',
  recipient: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
  mint_submitter: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
  finality: 'standard',
};

function envelopeError(code: string, message: string, status: number): Response {
  return new Response(
    JSON.stringify({
      v: 2,
      request_id: 'req-test',
      data: { error: code, message },
    }),
    { status, headers: { 'Content-Type': 'application/json' } },
  );
}

describe('CCTP SDK contract', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('API_ERROR_CODES includes CCTP taxonomy entries', () => {
    for (const code of [
      'cctp_not_enabled',
      'unsupported_corridor',
      'invalid_finality',
      'invalid_recipient',
      'fee_quote_unavailable',
      'attestation_pending',
      'attestation_expired',
      'mint_retryable',
      'transfer_not_found',
      'provider_killed',
    ]) {
      expect(API_ERROR_CODES).toContain(code);
    }
  });

  it('parseApiErrorBody reads nested envelope cctp_not_enabled', () => {
    const parsed = parseApiErrorBody({
      v: 2,
      data: {
        error: 'cctp_not_enabled',
        message: 'disabled',
      },
    });
    expect(parsed.error).toBe('cctp_not_enabled');
    expect(parsed.message).toBe('disabled');
  });

  it('getApiV2Info GETs /api/v2 and unwraps supported_corridors', async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          v: 2,
          data: {
            version: 2,
            chain_aware_assets: true,
            bridge_venues_metadata_only: true,
            bridge_settlement_executable: false,
            supported_chain_namespaces: ['stellar', 'eip155'],
            supported_corridors: [],
          },
        }),
        { status: 200, headers: { 'Content-Type': 'application/json' } },
      ),
    );
    vi.stubGlobal('fetch', fetchMock);

    const client = new StellarRouteClient({ baseUrl: 'http://api.test', retries: 0 });
    const info = await client.getApiV2Info();
    expect(info.bridge_settlement_executable).toBe(false);
    expect(info.supported_corridors).toEqual([]);

    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe('http://api.test/api/v2');
    expect(init.method).toBe('GET');
  });

  it('cctpQuote propagates idempotency and access headers on happy path', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            v: 2,
            data: {
              transfer_id: TRANSFER_ID,
              access_token: 'token-a',
              corridor_id: CCTP_TESTNET_CORRIDOR_ID,
              provider: CCTP_PROVIDER_ID,
              direction: 'evm_to_stellar',
              source_amount: '1',
              destination_amount: '1',
              fee_quote: {},
              expires_at: 1,
              finality: 'standard',
            },
          }),
          { status: 200, headers: { 'Content-Type': 'application/json' } },
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            v: 2,
            data: {
              transfer_id: TRANSFER_ID,
              access_token: 'token-a',
              corridor_id: CCTP_TESTNET_CORRIDOR_ID,
              provider: CCTP_PROVIDER_ID,
              direction: 'evm_to_stellar',
              source_amount: '1',
              destination_amount: '1',
              fee_quote: {},
              expires_at: 1,
              finality: 'standard',
            },
          }),
          { status: 200, headers: { 'Content-Type': 'application/json' } },
        ),
      );
    vi.stubGlobal('fetch', fetchMock);

    const client = new StellarRouteClient({ baseUrl: 'http://api.test', retries: 0 });
    const opts = {
      idempotencyKey: 'idem-sdk-1',
      accessToken: 'token-a',
    };
    await client.cctpQuote(sampleQuoteRequest, opts);
    await client.cctpGetTransfer(TRANSFER_ID, opts);

    const quoteInit = fetchMock.mock.calls[0][1] as RequestInit;
    expect(quoteInit.headers).toMatchObject({
      'idempotency-key': 'idem-sdk-1',
    });
    const statusInit = fetchMock.mock.calls[1][1] as RequestInit;
    expect(statusInit.headers).toMatchObject({
      'x-cctp-transfer-access': 'token-a',
    });
    expect(JSON.stringify(fetchMock.mock.results)).not.toContain('token-a');
  });

  it('cctpQuote POSTs quote body and surfaces nested 503 error', async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      envelopeError('cctp_not_enabled', 'not enabled', 503),
    );
    vi.stubGlobal('fetch', fetchMock);

    const client = new StellarRouteClient({ baseUrl: 'http://api.test', retries: 0 });
    await expect(client.cctpQuote(sampleQuoteRequest)).rejects.toSatisfy(
      (err: unknown) =>
        isStellarRouteApiError(err) &&
        err.code === 'cctp_not_enabled' &&
        err.status === 503,
    );

    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe('http://api.test/api/v2/bridge/cctp/quote');
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body as string)).toEqual(sampleQuoteRequest);
  });

  it('cctpPrepareBurn POSTs encoded transfer path with empty body', async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      envelopeError('cctp_not_enabled', 'not enabled', 503),
    );
    vi.stubGlobal('fetch', fetchMock);

    const client = new StellarRouteClient({ baseUrl: 'http://api.test', retries: 0 });
    await expect(client.cctpPrepareBurn(TRANSFER_ID)).rejects.toBeInstanceOf(
      StellarRouteApiError,
    );

    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe(
      `http://api.test/api/v2/bridge/cctp/${TRANSFER_ID}/prepare-burn`,
    );
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body as string)).toEqual({});
  });

  it('cctpGetTransfer GETs encoded transfer path', async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      envelopeError('cctp_not_enabled', 'not enabled', 503),
    );
    vi.stubGlobal('fetch', fetchMock);

    const client = new StellarRouteClient({ baseUrl: 'http://api.test', retries: 0 });
    await expect(client.cctpGetTransfer(TRANSFER_ID)).rejects.toBeInstanceOf(
      StellarRouteApiError,
    );

    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe(`http://api.test/api/v2/bridge/cctp/${TRANSFER_ID}`);
    expect(init.method).toBe('GET');
  });

  it('cctpPrepareMint POSTs encoded transfer path', async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      envelopeError('cctp_not_enabled', 'not enabled', 503),
    );
    vi.stubGlobal('fetch', fetchMock);

    const client = new StellarRouteClient({ baseUrl: 'http://api.test', retries: 0 });
    await expect(client.cctpPrepareMint(TRANSFER_ID)).rejects.toBeInstanceOf(
      StellarRouteApiError,
    );

    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe(
      `http://api.test/api/v2/bridge/cctp/${TRANSFER_ID}/prepare-mint`,
    );
    expect(init.method).toBe('POST');
  });

  it('cctpSubmitBurn serializes tx_hash acknowledgement body', async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      envelopeError('cctp_not_enabled', 'not enabled', 503),
    );
    vi.stubGlobal('fetch', fetchMock);

    const client = new StellarRouteClient({ baseUrl: 'http://api.test', retries: 0 });
    await expect(
      client.cctpSubmitBurn(TRANSFER_ID, { tx_hash: VALID_EVM_TX_HASH }),
    ).rejects.toSatisfy(
      (err: unknown) =>
        isStellarRouteApiError(err) && err.code === 'cctp_not_enabled',
    );

    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe(
      `http://api.test/api/v2/bridge/cctp/${TRANSFER_ID}/submit-burn`,
    );
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body as string)).toEqual({
      tx_hash: VALID_EVM_TX_HASH,
    });
  });

  it('cctpSubmitMint serializes tx_hash acknowledgement body', async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      envelopeError('cctp_not_enabled', 'not enabled', 503),
    );
    vi.stubGlobal('fetch', fetchMock);

    const client = new StellarRouteClient({ baseUrl: 'http://api.test', retries: 0 });
    await expect(
      client.cctpSubmitMint(TRANSFER_ID, { tx_hash: VALID_STELLAR_TX_HASH }),
    ).rejects.toBeInstanceOf(StellarRouteApiError);

    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe(
      `http://api.test/api/v2/bridge/cctp/${TRANSFER_ID}/submit-mint`,
    );
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body as string)).toEqual({
      tx_hash: VALID_STELLAR_TX_HASH,
    });
  });

  it('cctpReattest POSTs encoded transfer path with empty body', async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      envelopeError('cctp_not_enabled', 'not enabled', 503),
    );
    vi.stubGlobal('fetch', fetchMock);

    const client = new StellarRouteClient({ baseUrl: 'http://api.test', retries: 0 });
    await expect(client.cctpReattest(TRANSFER_ID)).rejects.toBeInstanceOf(
      StellarRouteApiError,
    );

    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe(
      `http://api.test/api/v2/bridge/cctp/${TRANSFER_ID}/reattest`,
    );
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body as string)).toEqual({});
  });
});
