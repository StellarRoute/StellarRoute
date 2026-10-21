import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { readConnectedAccountBalances, renderBalanceSummary } from './balance';

const TEST_ADDRESS = 'GABCDEFGHIJKLMNOPWXYZ';
const USDC_ISSUER = 'GDJTD6CQBWST7MW4T64GQK542YDVTZUSS4WK4NPCFHCGVYUD3CJ6IETQ';

describe('read-only balance tool', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders fixture account JSON asset amounts', () => {
    const summary = renderBalanceSummary({
      balances: [
        { balance: '42.5000000', asset_type: 'native' },
        {
          balance: '250.1234567',
          asset_type: 'credit_alphanum4',
          asset_code: 'USDC',
          asset_issuer: USDC_ISSUER,
        },
      ],
      address: TEST_ADDRESS,
      network: 'testnet',
    });

    expect(summary).toContain('XLM: 42.5000000');
    expect(summary).toContain('USDC: 250.1234567');
  });

  it('returns a connect prompt and skips Horizon when no wallet is connected', async () => {
    const fetchSpy = vi.spyOn(global, 'fetch');

    const result = await readConnectedAccountBalances({
      connected: false,
      address: TEST_ADDRESS,
      network: 'testnet',
    });

    expect(result.kind).toBe('connect_prompt');
    expect(result.message).toMatch(/connect your wallet/i);
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('reads the connected account from the public Horizon URL and never hits the quote endpoint', async () => {
    const fetchSpy = vi.spyOn(global, 'fetch').mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        balances: [
          { balance: '42.5000000', asset_type: 'native' },
          {
            balance: '250.1234567',
            asset_type: 'credit_alphanum4',
            asset_code: 'USDC',
            asset_issuer: USDC_ISSUER,
          },
        ],
      }),
    } as Response);

    const result = await readConnectedAccountBalances({
      connected: true,
      address: TEST_ADDRESS,
      network: 'testnet',
    });

    expect(result.kind).toBe('balances');
    expect(fetchSpy).toHaveBeenCalledWith(
      `https://horizon-testnet.stellar.org/accounts/${encodeURIComponent(TEST_ADDRESS)}`,
      expect.objectContaining({
        headers: expect.objectContaining({ Accept: 'application/json' }),
      }),
    );
    const quoteCalls = fetchSpy.mock.calls.filter(([url]) => String(url).includes('/api/v1/quote'));
    expect(quoteCalls).toHaveLength(0);
    expect(result.summary).toContain('XLM: 42.5000000');
    expect(result.summary).toContain('USDC: 250.1234567');
  });
});
