import { render, screen, waitFor, fireEvent, cleanup } from '@testing-library/react';
import { StatusDashboard } from './StatusDashboard';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Mock fetch
const mockFetch = vi.fn();
global.fetch = mockFetch;

const mockHealthyResponse = {
  data: {
    status: 'healthy',
    timestamp: '2024-01-01T00:00:00Z',
    version: '1.0.0',
    components: {
      database: 'healthy',
      redis: 'healthy',
    },
  },
};

const mockDepsResponse = {
  data: {
    status: 'ok',
    timestamp: '2024-01-01T00:00:00Z',
    components: {
      horizon: 'healthy',
      soroban_rpc: 'healthy',
    },
  },
};

describe('StatusDashboard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it('renders loading state initially', () => {
    mockFetch.mockImplementation(() => new Promise(() => {})); // Never resolves
    render(<StatusDashboard />);
    // Check for the loading spinner by data-testid
    const spinner = screen.getByTestId('icon');
    expect(spinner).toBeInTheDocument();
    expect(spinner).toHaveClass('animate-spin');
  });

  it('fetches and displays healthy status', async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => mockHealthyResponse,
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => mockDepsResponse,
      });

    render(<StatusDashboard />);

    await waitFor(() => {
      expect(screen.getByText('All Systems Operational')).toBeInTheDocument();
    }, { timeout: 3000 });

    expect(screen.getByText(/Version: 1\.0\.0/)).toBeInTheDocument();
  });

  it('handles fetch errors gracefully', async () => {
    mockFetch.mockRejectedValue(new Error('Network error'));

    render(<StatusDashboard />);

    await waitFor(() => {
      expect(screen.getByText('Connection Error')).toBeInTheDocument();
    }, { timeout: 3000 });

    expect(screen.getByText('Network error')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Retry/i })).toBeInTheDocument();
  });

  it('displays component statuses', async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => mockHealthyResponse,
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => mockDepsResponse,
      });

    render(<StatusDashboard />);

    await waitFor(() => {
      expect(screen.getByText('database')).toBeInTheDocument();
    }, { timeout: 3000 });

    expect(screen.getByText('redis')).toBeInTheDocument();
    expect(screen.getByText('horizon')).toBeInTheDocument();
  });

  it('displays status indicators legend', async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => mockHealthyResponse,
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => mockDepsResponse,
      });

    render(<StatusDashboard />);

    await waitFor(() => {
      expect(screen.getByText('Status Indicators:')).toBeInTheDocument();
    }, { timeout: 3000 });

    expect(screen.getByText(/Healthy\/OK/)).toBeInTheDocument();
    expect(screen.getByText(/Warning/)).toBeInTheDocument();
  });

  it('has refresh button', async () => {
    mockFetch
      .mockResolvedValue({
        ok: true,
        json: async () => mockHealthyResponse,
      })
      .mockResolvedValue({
        ok: true,
        json: async () => mockDepsResponse,
      });

    render(<StatusDashboard />);

    await waitFor(() => {
      expect(screen.getByText('All Systems Operational')).toBeInTheDocument();
    }, { timeout: 3000 });

    const buttons = screen.getAllByRole('button');
    expect(buttons.length).toBeGreaterThan(0);
  });
});
