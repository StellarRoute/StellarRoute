import React from 'react';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { HistoryPageClient, recordsToCsv, triggerCsvDownload } from './HistoryPageClient';

// ── Feature flag: on by default for these tests ───────────────────────────────
vi.mock('@/hooks/useFeatureFlag', () => ({
  useFeatureFlag: vi.fn((flag: string) => ({
    enabled: flag === 'history_csv_export',
    loading: false,
  })),
}));

// ── StellarRoute client mock ──────────────────────────────────────────────────
const getSwapActivity = vi.fn();
vi.mock('@/hooks/useStellarRouteClient', () => ({
  useStellarRouteClient: () => ({ getSwapActivity }),
}));

// ── URL API stubs ─────────────────────────────────────────────────────────────
const revokeObjectURL = vi.fn();
const createObjectURL = vi.fn(() => 'blob:test');
Object.defineProperty(URL, 'createObjectURL', { value: createObjectURL, writable: true, configurable: true });
Object.defineProperty(URL, 'revokeObjectURL', { value: revokeObjectURL, writable: true, configurable: true });

const ADDRESS = 'GBRPDEJSTXWHLT2YTIU6X7E3E5B5O3N4CUXOAT76O4Q4WUPTFBJMDSZH';

function swap(overrides: Record<string, unknown> = {}) {
  return {
    event_id: 'evt-1',
    contract_id: 'c-1',
    ledger: 100,
    ledger_closed_at: '2026-01-01T00:00:00Z',
    paging_token: 'abc123',
    sender: ADDRESS,
    amount_in: '100.0000000',
    amount_out: '10.5000000',
    fee_amount: '0.0000100',
    route: null,
    source_asset: 'XLM',
    destination_asset: 'USDC',
    ...overrides,
  };
}

describe('History CSV export (flag on)', () => {
  beforeEach(() => { vi.clearAllMocks(); });
  afterEach(() => { cleanup(); });

  it('renders the Export CSV button when records are present', async () => {
    getSwapActivity.mockResolvedValue({ swaps: [swap()] });
    render(<HistoryPageClient address={ADDRESS} />);

    await waitFor(() =>
      expect(screen.getByTestId('history-csv-export')).toBeInTheDocument()
    );
  });

  it('does not render Export CSV button when there are no records', async () => {
    getSwapActivity.mockResolvedValue({ swaps: [] });
    render(<HistoryPageClient address={ADDRESS} />);

    await waitFor(() =>
      expect(screen.getByText('No transactions yet')).toBeInTheDocument()
    );
    expect(screen.queryByTestId('history-csv-export')).not.toBeInTheDocument();
  });

  it('clicking Export CSV triggers a download', async () => {
    const user = userEvent.setup();
    getSwapActivity.mockResolvedValue({ swaps: [swap()] });

    // Spy on anchor click so we don't navigate
    const clickSpy = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});

    render(<HistoryPageClient address={ADDRESS} />);

    const btn = await screen.findByTestId('history-csv-export');
    await user.click(btn);

    expect(createObjectURL).toHaveBeenCalledWith(expect.any(Blob));
    expect(clickSpy).toHaveBeenCalledTimes(1);
    expect(revokeObjectURL).toHaveBeenCalledWith('blob:test');

    clickSpy.mockRestore();
  });
});

describe('recordsToCsv', () => {
  it('produces a header + one data row', () => {
    const rows = recordsToCsv([
      { id: 'e1', txHash: 'h1', timestamp: new Date('2026-01-01T00:00:00Z'), action: 'SWAP', amount: '10', asset: 'XLM → USDC' },
    ]);
    const lines = rows.split('\n');
    expect(lines[0]).toBe('id,txHash,timestamp,action,amount,asset');
    expect(lines[1]).toContain('e1');
    expect(lines[1]).toContain('XLM → USDC');
  });

  it('escapes double-quotes in field values', () => {
    const rows = recordsToCsv([
      { id: 'e"1', txHash: 'h1', timestamp: new Date('2026-01-01T00:00:00Z'), action: 'SWAP', amount: '10', asset: 'A' },
    ]);
    expect(rows).toContain('"e""1"');
  });

  it('returns only the header when given an empty array', () => {
    const rows = recordsToCsv([]);
    expect(rows).toBe('id,txHash,timestamp,action,amount,asset');
  });
});

describe('History CSV export (flag off)', () => {
  afterEach(cleanup);

  it('no Export CSV button when flag is off', async () => {
    // Re-mock with flag disabled using a module-level override
    const { useFeatureFlag } = await import('@/hooks/useFeatureFlag');
    vi.mocked(useFeatureFlag).mockReturnValue({ enabled: false, loading: false });

    getSwapActivity.mockResolvedValue({ swaps: [swap()] });
    render(<HistoryPageClient address={ADDRESS} />);

    await waitFor(() => expect(screen.getByTestId('trade-row')).toBeInTheDocument());
    expect(screen.queryByTestId('history-csv-export')).not.toBeInTheDocument();
  });
});
