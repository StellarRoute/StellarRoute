import { render, screen, waitFor, cleanup } from '@testing-library/react';
import { StatusDashboard } from './StatusDashboard';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

const mockRefreshHealth = vi.fn();
const mockRefreshDeps = vi.fn();
const mockRefreshCache = vi.fn();
const mockRefreshPool = vi.fn();

const mockHealthData = {
  status: 'healthy',
  timestamp: '2024-01-01T00:00:00Z',
  version: '1.0.0',
  components: {
    database: 'healthy',
    redis: 'healthy',
  },
};

const mockDepsData = {
  status: 'ok',
  timestamp: '2024-01-01T00:00:00Z',
  components: {
    horizon: 'healthy',
    soroban_rpc: 'healthy',
  },
};

const mockCacheData = {
  quote_hits: 12345,
  quote_misses: 2345,
  hit_ratio: 0.84,
  stale_quote_rejections: 150,
  stale_inputs_excluded: 42,
};

const mockPoolData = {
  primary: {
    max_connections: 20,
    size: 10,
    idle: 3,
    in_use: 7,
    utilisation: 0.7,
  },
};

vi.mock('@/hooks/useApi', () => ({
  useHealth: vi.fn(() => ({
    data: mockHealthData,
    loading: false,
    error: null,
    refresh: mockRefreshHealth,
  })),
  useHealthDeps: vi.fn(() => ({
    data: mockDepsData,
    loading: false,
    error: null,
    refresh: mockRefreshDeps,
  })),
  useCacheMetrics: vi.fn(() => ({
    data: mockCacheData,
    loading: false,
    error: null,
    refresh: mockRefreshCache,
  })),
  usePoolStats: vi.fn(() => ({
    data: mockPoolData,
    loading: false,
    error: null,
    refresh: mockRefreshPool,
  })),
}));

describe('StatusDashboard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it('renders loading state initially', async () => {
    const { useHealth, useHealthDeps } = await import('@/hooks/useApi');
    vi.mocked(useHealth).mockReturnValueOnce({
      data: undefined,
      loading: true,
      error: null,
      refresh: mockRefreshHealth,
    });
    vi.mocked(useHealthDeps).mockReturnValueOnce({
      data: undefined,
      loading: true,
      error: null,
      refresh: mockRefreshDeps,
    });

    render(<StatusDashboard />);
    const spinner = screen.getByTestId('icon');
    expect(spinner).toBeInTheDocument();
    expect(spinner).toHaveClass('animate-spin');
  });

  it('fetches and displays healthy status', async () => {
    render(<StatusDashboard />);

    await waitFor(() => {
      expect(screen.getByText('All Systems Operational')).toBeInTheDocument();
    });

    expect(screen.getByText(/Version: 1\.0\.0/)).toBeInTheDocument();
  });

  it('handles fetch errors gracefully', async () => {
    const { useHealth, useHealthDeps } = await import('@/hooks/useApi');
    vi.mocked(useHealth).mockReturnValueOnce({
      data: undefined,
      loading: false,
      error: new Error('Network error'),
      refresh: mockRefreshHealth,
    });
    vi.mocked(useHealthDeps).mockReturnValueOnce({
      data: undefined,
      loading: false,
      error: null,
      refresh: mockRefreshDeps,
    });

    render(<StatusDashboard />);

    await waitFor(() => {
      expect(screen.getByText('Connection Error')).toBeInTheDocument();
    });

    expect(screen.getByText(/network connection interrupted/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Retry/i })).toBeInTheDocument();
  });

  it('displays component statuses', async () => {
    render(<StatusDashboard />);

    await waitFor(() => {
      expect(screen.getByText('database')).toBeInTheDocument();
    });

    expect(screen.getByText('redis')).toBeInTheDocument();
    expect(screen.getByText('horizon')).toBeInTheDocument();
  });

  it('displays status indicators legend', async () => {
    render(<StatusDashboard />);

    await waitFor(() => {
      expect(screen.getByText('Status Indicators:')).toBeInTheDocument();
    });

    expect(screen.getByText(/Healthy\/OK/)).toBeInTheDocument();
    expect(screen.getByText(/Warning/)).toBeInTheDocument();
  });

  it('has refresh button', async () => {
    render(<StatusDashboard />);

    await waitFor(() => {
      expect(screen.getByText('All Systems Operational')).toBeInTheDocument();
    });

    const buttons = screen.getAllByRole('button');
    expect(buttons.length).toBeGreaterThan(0);
  });

  it('displays performance KPI cards', async () => {
    render(<StatusDashboard />);

    await waitFor(() => {
      expect(screen.getByText('Performance KPIs')).toBeInTheDocument();
    });

    expect(screen.getByText('Cache Hit Ratio')).toBeInTheDocument();
    expect(screen.getByText('Stale Rejections')).toBeInTheDocument();
    expect(screen.getByText('DB Connections (Primary)')).toBeInTheDocument();
    expect(screen.getByText('DB Utilization (Primary)')).toBeInTheDocument();
  });

  it('formats cache metrics correctly', async () => {
    render(<StatusDashboard />);

    await waitFor(() => {
      expect(screen.getByText('Performance KPIs')).toBeInTheDocument();
    });

    expect(screen.getByText('84.0%')).toBeInTheDocument();
    expect(screen.getByText('12.3K hits / 2.3K misses')).toBeInTheDocument();
    expect(screen.getByText('150')).toBeInTheDocument();
  });

  it('formats pool stats correctly', async () => {
    render(<StatusDashboard />);

    await waitFor(() => {
      expect(screen.getByText('Performance KPIs')).toBeInTheDocument();
    });

    expect(screen.getByText('70%')).toBeInTheDocument();
    expect(screen.getByText('10')).toBeInTheDocument();
  });
});
