import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';

import StatusPage, { metadata } from './page';

/** Shape returned by `GET /health`. */
const HEALTH_FIXTURE = {
  status: 'healthy',
  timestamp: '2026-01-01T00:00:00Z',
  version: '1.0.0',
  components: {
    database: 'healthy',
    redis: 'healthy',
  },
};

/** Shape returned by `GET /health/deps`. */
const DEPS_FIXTURE = {
  status: 'ok',
  timestamp: '2026-01-01T00:00:00Z',
  components: {
    horizon: 'healthy',
    soroban_rpc: 'healthy',
  },
};

const refreshHealth = vi.fn();
const refreshDeps = vi.fn();

vi.mock('@/hooks/useApi', () => ({
  useHealth: vi.fn(),
  useHealthDeps: vi.fn(),
}));

import { useHealth, useHealthDeps } from '@/hooks/useApi';

type HealthOverrides = {
  health?: Partial<typeof HEALTH_FIXTURE> | null;
  deps?: Partial<typeof DEPS_FIXTURE> | null;
  loading?: boolean;
  error?: Error | null;
};

/** Drive the status dashboard from fixtures instead of a live /health fetch. */
function mockHealth(overrides: HealthOverrides = {}) {
  const { health, deps, loading = false, error = null } = overrides;
  vi.mocked(useHealth).mockReturnValue({
    data: health === null ? undefined : { ...HEALTH_FIXTURE, ...health },
    loading,
    error,
    refresh: refreshHealth,
  } as never);
  vi.mocked(useHealthDeps).mockReturnValue({
    data: deps === null ? undefined : { ...DEPS_FIXTURE, ...deps },
    loading,
    error,
    refresh: refreshDeps,
  } as never);
}

describe('StatusPage', () => {
  beforeEach(() => {
    vi.mocked(useHealth).mockReset();
    vi.mocked(useHealthDeps).mockReset();
  });

  it('renders the status page heading and subtitle', () => {
    mockHealth();

    render(<StatusPage />);

    expect(
      screen.getByRole('heading', { level: 1, name: 'API Status' }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        'Real-time health monitoring of StellarRoute services and dependencies',
      ),
    ).toBeInTheDocument();
    expect(screen.getByRole('main')).toBeInTheDocument();
  });

  it('shows a status heading for a healthy health payload', () => {
    mockHealth();

    render(<StatusPage />);

    expect(screen.getByText('All Systems Operational')).toBeInTheDocument();
    expect(screen.getByText(/Version: 1\.0\.0/)).toBeInTheDocument();
  });

  it('shows a degraded status heading when the API reports degraded', () => {
    mockHealth({ health: { status: 'degraded' } });

    render(<StatusPage />);

    expect(screen.getByText('Service Degraded')).toBeInTheDocument();
    expect(
      screen.queryByText('All Systems Operational'),
    ).not.toBeInTheDocument();
  });

  it('shows the unavailable status heading when the API reports no status', () => {
    mockHealth({ health: { status: '' } });

    render(<StatusPage />);

    expect(screen.getByText('Status Unavailable')).toBeInTheDocument();
    expect(
      screen.queryByText('All Systems Operational'),
    ).not.toBeInTheDocument();
  });

  it('lists core components and external dependencies from the mocked fetch', () => {
    mockHealth();

    render(<StatusPage />);

    expect(screen.getByText('Core Components')).toBeInTheDocument();
    expect(screen.getByText('database')).toBeInTheDocument();
    expect(screen.getByText('redis')).toBeInTheDocument();

    expect(screen.getByText('External Dependencies')).toBeInTheDocument();
    expect(screen.getByText('horizon')).toBeInTheDocument();
    expect(screen.getByText('soroban rpc')).toBeInTheDocument();
  });

  it('renders the loading state while the health fetch is in flight', () => {
    mockHealth({ health: null, deps: null, loading: true });

    render(<StatusPage />);

    // The page heading is always present; the dashboard shows a spinner.
    expect(
      screen.getByRole('heading', { level: 1, name: 'API Status' }),
    ).toBeInTheDocument();
    expect(screen.getByTestId('icon')).toHaveClass('animate-spin');
    expect(screen.queryByText('All Systems Operational')).not.toBeInTheDocument();
  });

  it('shows the connection error state when the health fetch fails', () => {
    mockHealth({ health: null, deps: null, error: new Error('Network error') });

    render(<StatusPage />);

    expect(screen.getByText('Connection Error')).toBeInTheDocument();
    expect(
      screen.getByText(/network connection interrupted/i),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Retry/i })).toBeInTheDocument();
  });

  it('keeps auto-refresh on and exposes the manual refresh control', () => {
    mockHealth();

    render(<StatusPage />);

    expect(
      screen.getByRole('button', { name: /Auto-refresh ON/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: /Refresh status/i }),
    ).toBeInTheDocument();
  });

  it('makes no live network calls of its own', () => {
    mockHealth();
    const fetchSpy = vi.spyOn(globalThis, 'fetch');

    render(<StatusPage />);

    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('does not render any swap health pill or swap deck', () => {
    mockHealth();

    const { container } = render(<StatusPage />);

    // The /status surface must stay independent of the /swap deck.
    expect(container.querySelector('[data-testid="swap-card"]')).toBeNull();
    expect(container.querySelector('[data-testid="health-pill"]')).toBeNull();
    expect(container.querySelector('[data-slot="swap-card"]')).toBeNull();
  });
});

describe('StatusPage metadata', () => {
  it('keeps the SEO title, description and canonical path', () => {
    expect(metadata).toMatchObject({
      title: 'API Status',
      description:
        'Live health status of the StellarRoute Stellar DEX aggregator API and dependencies',
      alternates: { canonical: expect.stringContaining('/status') },
    });
  });
});
