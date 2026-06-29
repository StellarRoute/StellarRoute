/**
 * Tests for checkAddressChange in lib/wallet/index.ts (Issue #737)
 *
 * Covers:
 * - xBull account changes update wallet context public key
 * - Freighter behavior unchanged
 * - Returns null when no change detected
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { checkAddressChange } from './index';

// Mock @stellar/freighter-api (alias wired in vitest.config.ts)
import * as freighter from '@stellar/freighter-api';

beforeEach(() => {
  vi.clearAllMocks();
  // Reset window.xbull
  delete (window as unknown as Record<string, unknown>).xbull;
});

describe('checkAddressChange – Freighter', () => {
  it('returns null when address has not changed', async () => {
    vi.mocked(freighter.getAddress).mockResolvedValueOnce({
      address: 'GABC123',
    });

    const result = await checkAddressChange('freighter', 'GABC123');
    expect(result).toBeNull();
  });

  it('returns new address when Freighter address changed', async () => {
    vi.mocked(freighter.getAddress).mockResolvedValueOnce({
      address: 'GNEW456',
    });

    const result = await checkAddressChange('freighter', 'GABC123');
    expect(result).toBe('GNEW456');
  });

  it('returns null when currentAddress is null', async () => {
    const result = await checkAddressChange('freighter', null);
    expect(result).toBeNull();
    expect(freighter.getAddress).not.toHaveBeenCalled();
  });

  it('returns null when Freighter API errors', async () => {
    vi.mocked(freighter.getAddress).mockRejectedValueOnce(new Error('Extension error'));

    const result = await checkAddressChange('freighter', 'GABC123');
    expect(result).toBeNull();
  });
});

describe('checkAddressChange – xBull', () => {
  it('returns null when xBull is not installed', async () => {
    const result = await checkAddressChange('xbull', 'GABC123');
    expect(result).toBeNull();
  });

  it('uses getPublicKey() for passive detection when available', async () => {
    const mockGetPublicKey = vi.fn().mockResolvedValueOnce('GNEW_XBULL_456');
    (window as unknown as Record<string, unknown>).xbull = {
      getPublicKey: mockGetPublicKey,
      connect: vi.fn(),
    };

    const result = await checkAddressChange('xbull', 'GABC123');
    expect(result).toBe('GNEW_XBULL_456');
    expect(mockGetPublicKey).toHaveBeenCalledTimes(1);
  });

  it('falls back to connect() when getPublicKey is not available', async () => {
    const mockConnect = vi.fn().mockResolvedValueOnce({ publicKey: 'GNEW_XBULL_456' });
    (window as unknown as Record<string, unknown>).xbull = {
      connect: mockConnect,
    };

    const result = await checkAddressChange('xbull', 'GABC123');
    expect(result).toBe('GNEW_XBULL_456');
    expect(mockConnect).toHaveBeenCalledTimes(1);
  });

  it('returns null when xBull address has not changed', async () => {
    const mockGetPublicKey = vi.fn().mockResolvedValueOnce('GABC123');
    (window as unknown as Record<string, unknown>).xbull = {
      getPublicKey: mockGetPublicKey,
      connect: vi.fn(),
    };

    const result = await checkAddressChange('xbull', 'GABC123');
    expect(result).toBeNull();
  });

  it('returns null when xBull throws', async () => {
    (window as unknown as Record<string, unknown>).xbull = {
      getPublicKey: vi.fn().mockRejectedValueOnce(new Error('User denied')),
      connect: vi.fn(),
    };

    const result = await checkAddressChange('xbull', 'GABC123');
    expect(result).toBeNull();
  });
});
