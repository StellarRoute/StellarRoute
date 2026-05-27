import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  buildExplorerUrl,
  buildNotificationBody,
  buildNotificationTitle,
  dispatchTransactionNotification,
  isNotificationSupported,
  type NotificationParams,
  type NotificationPreference,
} from './notifications';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const confirmedParams: NotificationParams = {
  status: 'confirmed',
  txHash: 'abc123def456',
  fromAsset: 'XLM',
  fromAmount: '100',
  toAsset: 'USDC',
  toAmount: '25.50',
  txId: 'tx-001',
};

const failedParams: NotificationParams = {
  status: 'failed',
  fromAsset: 'XLM',
  fromAmount: '100',
  toAsset: 'USDC',
  toAmount: '25.50',
  txId: 'tx-002',
};

const droppedParams: NotificationParams = {
  status: 'dropped',
  fromAsset: 'XLM',
  fromAmount: '100',
  toAsset: 'USDC',
  toAmount: '25.50',
  txId: 'tx-003',
};

const enabledPreference: NotificationPreference = { enabled: true };
const disabledPreference: NotificationPreference = { enabled: false };

// ---------------------------------------------------------------------------
// buildNotificationTitle
// ---------------------------------------------------------------------------

describe('buildNotificationTitle', () => {
  it('returns "Swap Confirmed" for confirmed status', () => {
    expect(buildNotificationTitle('confirmed')).toBe('Swap Confirmed');
  });

  it('returns "Swap Failed" for failed status', () => {
    expect(buildNotificationTitle('failed')).toBe('Swap Failed');
  });

  it('returns "Swap Dropped" for dropped status', () => {
    expect(buildNotificationTitle('dropped')).toBe('Swap Dropped');
  });
});

// ---------------------------------------------------------------------------
// buildNotificationBody
// ---------------------------------------------------------------------------

describe('buildNotificationBody', () => {
  it('returns correct body for confirmed status', () => {
    expect(buildNotificationBody(confirmedParams)).toBe(
      'Swapped 100 XLM → 25.50 USDC\nTx: abc123def456',
    );
  });

  it('returns correct body for failed status', () => {
    expect(buildNotificationBody(failedParams)).toBe(
      'Swap of 100 XLM → 25.50 USDC failed.',
    );
  });

  it('returns correct body for dropped status', () => {
    expect(buildNotificationBody(droppedParams)).toBe(
      'Swap of 100 XLM → 25.50 USDC was dropped. You may resubmit.',
    );
  });
});

// ---------------------------------------------------------------------------
// buildExplorerUrl
// ---------------------------------------------------------------------------

describe('buildExplorerUrl', () => {
  it('returns the correct Stellar Expert URL for a given hash', () => {
    expect(buildExplorerUrl('abc123def456')).toBe(
      'https://stellar.expert/explorer/public/tx/abc123def456',
    );
  });
});

// ---------------------------------------------------------------------------
// isNotificationSupported
// ---------------------------------------------------------------------------

describe('isNotificationSupported', () => {
  it('returns false when window.Notification is absent', () => {
    const original = (window as Record<string, unknown>).Notification;
    // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
    delete (window as Record<string, unknown>).Notification;

    expect(isNotificationSupported()).toBe(false);

    // Restore
    (window as Record<string, unknown>).Notification = original;
  });

  it('returns true when window.Notification is present', () => {
    // jsdom does not provide Notification by default, so we install a stub
    const original = (window as Record<string, unknown>).Notification;
    (window as Record<string, unknown>).Notification = vi.fn();

    expect(isNotificationSupported()).toBe(true);

    (window as Record<string, unknown>).Notification = original;
  });
});

// ---------------------------------------------------------------------------
// dispatchTransactionNotification
// ---------------------------------------------------------------------------

describe('dispatchTransactionNotification', () => {
  let NotificationMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    NotificationMock = vi.fn();
    // Install mock Notification constructor with granted permission
    Object.defineProperty(NotificationMock, 'permission', {
      value: 'granted',
      configurable: true,
      writable: true,
    });
    (window as Record<string, unknown>).Notification = NotificationMock;
  });

  afterEach(() => {
    // Remove the mock so other tests start clean
    delete (window as Record<string, unknown>).Notification;
    vi.restoreAllMocks();
  });

  it('does not call new Notification() when preference.enabled is false', () => {
    dispatchTransactionNotification(confirmedParams, disabledPreference);
    expect(NotificationMock).not.toHaveBeenCalled();
  });

  it('does not call new Notification() when Notification.permission is "denied"', () => {
    Object.defineProperty(NotificationMock, 'permission', {
      value: 'denied',
      configurable: true,
    });
    dispatchTransactionNotification(confirmedParams, enabledPreference);
    expect(NotificationMock).not.toHaveBeenCalled();
  });

  it('does not throw when window.Notification is undefined', () => {
    delete (window as Record<string, unknown>).Notification;
    expect(() =>
      dispatchTransactionNotification(confirmedParams, enabledPreference),
    ).not.toThrow();
  });

  it('sets tag, icon, and data.url correctly for a confirmed transaction', () => {
    dispatchTransactionNotification(confirmedParams, enabledPreference);

    expect(NotificationMock).toHaveBeenCalledOnce();
    const [title, options] = NotificationMock.mock.calls[0] as [
      string,
      NotificationOptions,
    ];

    expect(title).toBe('Swap Confirmed');
    expect(options.tag).toBe('tx-001');
    expect(options.icon).toBe('/icons/icon-192.png');
    expect((options.data as { url: string }).url).toBe(
      'https://stellar.expert/explorer/public/tx/abc123def456',
    );
  });

  it('does not set data.url for a failed transaction', () => {
    dispatchTransactionNotification(failedParams, enabledPreference);

    expect(NotificationMock).toHaveBeenCalledOnce();
    const [, options] = NotificationMock.mock.calls[0] as [
      string,
      NotificationOptions,
    ];
    expect(options.data).toBeUndefined();
  });

  it('does not mutate the params object', () => {
    const paramsCopy = { ...confirmedParams };
    dispatchTransactionNotification(confirmedParams, enabledPreference);
    expect(confirmedParams).toEqual(paramsCopy);
  });
});
