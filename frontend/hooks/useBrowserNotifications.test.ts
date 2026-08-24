import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { useBrowserNotifications } from './useBrowserNotifications';

// ---------------------------------------------------------------------------
// Helpers — install / remove a mock Notification constructor on window
// ---------------------------------------------------------------------------

function installNotificationMock(
  permission: NotificationPermission = 'default',
) {
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

function removeNotificationMock() {
  delete (window as Record<string, unknown>).Notification;
}

// ---------------------------------------------------------------------------
// Setup / teardown
// ---------------------------------------------------------------------------

beforeEach(() => {
  localStorage.clear();
});

afterEach(() => {
  removeNotificationMock();
  localStorage.clear();
  vi.restoreAllMocks();
});

// ---------------------------------------------------------------------------
// Initialisation
// ---------------------------------------------------------------------------

describe('useBrowserNotifications — initialisation', () => {
  it('initialises with false when localStorage key is absent', async () => {
    installNotificationMock('default');

    const { result } = renderHook(() => useBrowserNotifications());

    // Wait for queueMicrotask hydration
    await act(async () => {
      await Promise.resolve();
    });

    expect(result.current.browserNotifications).toBe(false);
    expect(result.current.isHydrated).toBe(true);
  });

  it('restores true from localStorage on mount', async () => {
    localStorage.setItem(
      'stellarroute.settings.browserNotifications',
      'true',
    );
    installNotificationMock('granted');

    const { result } = renderHook(() => useBrowserNotifications());

    await act(async () => {
      await Promise.resolve();
    });

    expect(result.current.browserNotifications).toBe(true);
    expect(result.current.isHydrated).toBe(true);
  });

  it('overrides to false when Notification.permission is "denied" regardless of localStorage', async () => {
    localStorage.setItem(
      'stellarroute.settings.browserNotifications',
      'true',
    );
    installNotificationMock('denied');

    const { result } = renderHook(() => useBrowserNotifications());

    await act(async () => {
      await Promise.resolve();
    });

    expect(result.current.browserNotifications).toBe(false);
    expect(result.current.permissionState).toBe('denied');
    expect(result.current.isDisabled).toBe(true);
  });

  it('sets permissionState to "unsupported" when Notification API is absent', async () => {
    removeNotificationMock(); // ensure no Notification on window

    const { result } = renderHook(() => useBrowserNotifications());

    await act(async () => {
      await Promise.resolve();
    });

    expect(result.current.permissionState).toBe('unsupported');
    expect(result.current.isDisabled).toBe(true);
    expect(result.current.browserNotifications).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// enableNotifications
// ---------------------------------------------------------------------------

describe('useBrowserNotifications — enableNotifications()', () => {
  it('calls Notification.requestPermission() exactly once', async () => {
    const mock = installNotificationMock('default');
    mock.requestPermission = vi.fn().mockResolvedValue('granted');
    Object.defineProperty(mock, 'permission', {
      value: 'granted',
      configurable: true,
    });

    const { result } = renderHook(() => useBrowserNotifications());

    await act(async () => {
      await Promise.resolve();
    });

    await act(async () => {
      await result.current.enableNotifications();
    });

    expect(mock.requestPermission).toHaveBeenCalledOnce();
  });

  it('sets browserNotifications to true when permission is granted', async () => {
    const mock = installNotificationMock('default');
    mock.requestPermission = vi.fn().mockResolvedValue('granted');
    Object.defineProperty(mock, 'permission', {
      value: 'granted',
      configurable: true,
    });

    const { result } = renderHook(() => useBrowserNotifications());

    await act(async () => {
      await Promise.resolve();
    });

    await act(async () => {
      await result.current.enableNotifications();
    });

    expect(result.current.browserNotifications).toBe(true);
    expect(result.current.permissionState).toBe('granted');
  });

  it('sets browserNotifications to false when permission is denied', async () => {
    const mock = installNotificationMock('default');
    mock.requestPermission = vi.fn().mockResolvedValue('denied');

    const { result } = renderHook(() => useBrowserNotifications());

    await act(async () => {
      await Promise.resolve();
    });

    await act(async () => {
      await result.current.enableNotifications();
    });

    expect(result.current.browserNotifications).toBe(false);
    expect(result.current.permissionState).toBe('denied');
    expect(localStorage.getItem('stellarroute.settings.browserNotifications')).toBe('false');
  });

  it('sets browserNotifications to false when permission is "default" (dismissed)', async () => {
    const mock = installNotificationMock('default');
    mock.requestPermission = vi.fn().mockResolvedValue('default');

    const { result } = renderHook(() => useBrowserNotifications());

    await act(async () => {
      await Promise.resolve();
    });

    await act(async () => {
      await result.current.enableNotifications();
    });

    expect(result.current.browserNotifications).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// disableNotifications
// ---------------------------------------------------------------------------

describe('useBrowserNotifications — disableNotifications()', () => {
  it('sets browserNotifications to false and persists to localStorage', async () => {
    localStorage.setItem(
      'stellarroute.settings.browserNotifications',
      'true',
    );
    const mock = installNotificationMock('granted');
    mock.requestPermission = vi.fn().mockResolvedValue('granted');

    const { result } = renderHook(() => useBrowserNotifications());

    await act(async () => {
      await Promise.resolve();
    });

    act(() => {
      result.current.disableNotifications();
    });

    expect(result.current.browserNotifications).toBe(false);
    expect(localStorage.getItem('stellarroute.settings.browserNotifications')).toBe('false');
  });

  it('does NOT call Notification.requestPermission()', async () => {
    const mock = installNotificationMock('granted');
    mock.requestPermission = vi.fn();

    const { result } = renderHook(() => useBrowserNotifications());

    await act(async () => {
      await Promise.resolve();
    });

    act(() => {
      result.current.disableNotifications();
    });

    expect(mock.requestPermission).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// localStorage error handling
// ---------------------------------------------------------------------------

describe('useBrowserNotifications — localStorage error handling', () => {
  it('defaults to false when localStorage.getItem throws', async () => {
    installNotificationMock('default');

    const originalGetItem = Storage.prototype.getItem;
    Storage.prototype.getItem = () => {
      throw new Error('Storage unavailable');
    };

    const { result } = renderHook(() => useBrowserNotifications());

    await act(async () => {
      await Promise.resolve();
    });

    expect(result.current.browserNotifications).toBe(false);
    expect(result.current.isHydrated).toBe(true);

    Storage.prototype.getItem = originalGetItem;
  });
});
