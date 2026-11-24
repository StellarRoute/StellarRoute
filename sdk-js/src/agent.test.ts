import { describe, expect, it, vi, afterEach } from 'vitest';
import { StellarRouteClient } from './client.js';

function ok(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

function apiError(code: string, message: string, status: number): Response {
  return new Response(JSON.stringify({ error: code, message }), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

afterEach(() => vi.restoreAllMocks());

// ── agentHealth ───────────────────────────────────────────────────────────────

describe('agentHealth', () => {
  it('returns enabled health on 200', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      ok({ data: { enabled: true } }),
    );
    const result = await new StellarRouteClient().agentHealth();
    expect(result.enabled).toBe(true);
  });

  it('404 health is disabled (does not throw)', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiError('not_found', 'agent disabled', 404),
    );
    const result = await new StellarRouteClient({ retries: 0 }).agentHealth();
    expect(result).toEqual({ enabled: false });
  });

  it('calls the correct endpoint', async () => {
    const spy = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValueOnce(ok({ enabled: true }));
    await new StellarRouteClient({
      baseUrl: 'https://api.example.com',
    }).agentHealth();
    expect(spy.mock.calls[0]?.[0]).toBe(
      'https://api.example.com/api/v1/agent/health',
    );
  });
});

// ── agentTools ────────────────────────────────────────────────────────────────

describe('agentTools', () => {
  it('returns tools catalog on 200', async () => {
    const catalog = {
      data: {
        tools: [
          { name: 'get_quote', description: 'Fetch a price quote' },
          { name: 'execute_swap', description: 'Execute a swap' },
        ],
      },
    };
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(catalog));
    const result = await new StellarRouteClient().agentTools();
    expect(result).not.toHaveProperty('enabled', false);
    if ('tools' in result) {
      expect(result.tools).toHaveLength(2);
      expect(result.tools[0]?.name).toBe('get_quote');
      expect(result.tools[1]?.description).toBe('Execute a swap');
    }
  });

  it('404 returns disabled result (does not throw)', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiError('not_found', 'agent disabled', 404),
    );
    const result = await new StellarRouteClient({ retries: 0 }).agentTools();
    expect(result).toEqual({ enabled: false });
  });

  it('calls the correct endpoint', async () => {
    const spy = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValueOnce(ok({ data: { tools: [] } }));
    await new StellarRouteClient({
      baseUrl: 'https://api.example.com',
    }).agentTools();
    expect(spy.mock.calls[0]?.[0]).toBe(
      'https://api.example.com/api/v1/agent/catalog',
    );
  });
});

// ── validateAgentIntent ───────────────────────────────────────────────────────

describe('validateAgentIntent', () => {
  it('returns validated intent on 200', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      ok({ data: { amount: '100.0', type: 'swap' } }),
    );
    const result = await new StellarRouteClient().validateAgentIntent({
      type: 'swap',
      amount: '100.0',
      asset: 'USDC',
    });
    expect(result).not.toHaveProperty('enabled', false);
    if ('amount' in result) {
      expect(result.amount).toBe('100.0');
      expect(result.type).toBe('swap');
    }
  });

  it('404 returns disabled result (does not throw)', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiError('not_found', 'agent disabled', 404),
    );
    const result = await new StellarRouteClient({ retries: 0 })
      .validateAgentIntent({
        type: 'swap',
        amount: '100.0',
        asset: 'USDC',
      });
    expect(result).toEqual({ enabled: false });
  });

  it('POSTs to the correct endpoint', async () => {
    const spy = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValueOnce(ok({ data: { amount: '50.0', type: 'send' } }));
    await new StellarRouteClient({
      baseUrl: 'https://api.example.com',
    }).validateAgentIntent({
      type: 'send',
      amount: '50.0',
      asset: 'XLM',
      recipient: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
    });
    const url = spy.mock.calls[0]?.[0] as string;
    expect(url).toBe('https://api.example.com/api/v1/agent/intents/validate');
    const init = spy.mock.calls[0]?.[1] as RequestInit;
    expect(init.method).toBe('POST');
    const body = JSON.parse(init.body as string);
    expect(body).toMatchObject({
      type: 'send',
      amount: '50.0',
      asset: 'XLM',
      recipient: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
    });
  });

  it('re-throws non-404 errors', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiError('overloaded', 'Service overloaded', 503),
    );
    await expect(
      new StellarRouteClient({ retries: 0 }).validateAgentIntent({
        type: 'swap',
        amount: '100.0',
        asset: 'USDC',
      }),
    ).rejects.toThrow();
  });
});