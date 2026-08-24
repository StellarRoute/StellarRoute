/**
 * Property-based tests for useBrowserNotifications hook.
 * Uses fast-check@3.22.0.
 *
 * Feature: browser-transaction-notifications
 */

import * as fc from 'fast-check';
import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { useBrowserNotifications } from './useBrowserNotifications';

const STORAGE_KEY = 'stellarroute.settings.browserNotifications';

function installNotificationMock(permission: NotificationPermission = 'granted') {
  const mock = vi.fn();
  Object.defineProperty(mock, 'permission', {
    value: permission,
    configurable: true,
    writable: true,
  });
  mock.requestPermission = vi.fn().mockResolvedValue(permission);
  (window as Record<string, unknown>).Notification = mock;
  return mock;
}

beforeEach(() => {
  localStorage.clear();
});

afterEach(() => {
  delete (window as Record<string, unknown>).Notification;
  localStorage.clear();
  vi.restoreAllMocks();
});

// ---------------------------------------------------------------------------
// Property 7: Preference persistence round-trip
// ---------------------------------------------------------------------------

// Feature: browser-transaction-notifications, Property 7: Preference persistence round-trip
describe('Property 7: Preference persistence round-trip', () => {
  it('localStorage contains "true" or "false" string matching the written preference', async () => {
    await fc.assert(
      fc.asyncProperty(fc.boolean(), async (initialValue) => {
        localStorage.clear();
        installNotificationMock('granted');

        if (initialValue) {
          localStorage.setItem(STORAGE_KEY, 'true');
        }

        const { result, unmount } = renderHook(() => useBrowserNotifications());

        await act(async () => {
          await Promise.resolve();
        });

        // The in-memory value should match what was stored
        const storedRaw = localStorage.getItem(STORAGE_KEY);
        // storedRaw may be null (absent = false) or "true"/"false"
        const storedBool = storedRaw === 'true';
        expect(result.current.browserNotifications).toBe(storedBool);

        // Now write a new value via disableNotifications and verify round-trip
        act(() => {
          result.current.disableNotifications();
        });

        expect(localStorage.getItem(STORAGE_KEY)).toBe('false');
        expect(result.current.browserNotifications).toBe(false);

        unmount();
        delete (window as Record<string, unknown>).Notification;
        localStorage.clear();
      }),
      { numRuns: 20 }, // fewer runs since each involves async React rendering
    );
  });

  it('re-initialising the hook restores the same in-memory boolean value', async () => {
    await fc.assert(
      fc.asyncProperty(fc.boolean(), async (value) => {
        localStorage.clear();
        installNotificationMock('granted');

        // Pre-seed localStorage
        localStorage.setItem(STORAGE_KEY, value ? 'true' : 'false');

        const { result, unmount } = renderHook(() => useBrowserNotifications());

        await act(async () => {
          await Promise.resolve();
        });

        expect(result.current.browserNotifications).toBe(value);

        unmount();
        delete (window as Record<string, unknown>).Notification;
        localStorage.clear();
      }),
      { numRuns: 20 },
    );
  });
});
