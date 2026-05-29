/**
 * Property-based tests for notificationManager (lib/notifications.ts).
 * Uses fast-check@3.22.0.
 *
 * Feature: browser-transaction-notifications
 */

import * as fc from 'fast-check';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  buildExplorerUrl,
  buildNotificationBody,
  buildNotificationTitle,
  dispatchTransactionNotification,
  type NotificationParams,
  type TerminalStatus,
} from './notifications';

// ---------------------------------------------------------------------------
// Arbitraries
// ---------------------------------------------------------------------------

const terminalStatusArb = fc.constantFrom<TerminalStatus>(
  'confirmed',
  'failed',
  'dropped',
);

/** Non-empty printable string (avoids empty strings that could mask bugs). */
const nonEmptyStringArb = fc.string({ minLength: 1, maxLength: 64 });

const notificationParamsArb = (
  statusOverride?: TerminalStatus,
): fc.Arbitrary<NotificationParams> =>
  fc
    .record({
      status: statusOverride ? fc.constant(statusOverride) : terminalStatusArb,
      txHash: nonEmptyStringArb,
      fromAsset: nonEmptyStringArb,
      fromAmount: nonEmptyStringArb,
      toAsset: nonEmptyStringArb,
      toAmount: nonEmptyStringArb,
      txId: nonEmptyStringArb,
    })
    .map((r) => r as NotificationParams);

// ---------------------------------------------------------------------------
// Property 1: Notification body contains all swap summary fields
// ---------------------------------------------------------------------------

// Feature: browser-transaction-notifications, Property 1: Notification body contains all swap summary fields for any terminal status
describe('Property 1: Notification body contains all swap summary fields for any terminal status', () => {
  it('contains fromAsset, fromAmount, toAsset, toAmount verbatim', () => {
    fc.assert(
      fc.property(notificationParamsArb(), (params) => {
        const body = buildNotificationBody(params);
        expect(body).toContain(params.fromAsset);
        expect(body).toContain(params.fromAmount);
        expect(body).toContain(params.toAsset);
        expect(body).toContain(params.toAmount);
      }),
      { numRuns: 100 },
    );
  });
});

// ---------------------------------------------------------------------------
// Property 2: Confirmed notification body matches exact template
// ---------------------------------------------------------------------------

// Feature: browser-transaction-notifications, Property 2: Confirmed notification body matches exact template
describe('Property 2: Confirmed notification body matches exact template', () => {
  it('returns exactly the confirmed template for any confirmed params', () => {
    fc.assert(
      fc.property(notificationParamsArb('confirmed'), (params) => {
        const body = buildNotificationBody(params);
        const expected = `Swapped ${params.fromAmount} ${params.fromAsset} → ${params.toAmount} ${params.toAsset}\nTx: ${params.txHash}`;
        expect(body).toBe(expected);
      }),
      { numRuns: 100 },
    );
  });
});

// ---------------------------------------------------------------------------
// Property 3: Failed and dropped notification bodies match exact templates
// ---------------------------------------------------------------------------

// Feature: browser-transaction-notifications, Property 3: Failed and dropped notification bodies match exact templates
describe('Property 3: Failed and dropped notification bodies match exact templates', () => {
  it('returns exactly the failed template for any failed params', () => {
    fc.assert(
      fc.property(notificationParamsArb('failed'), (params) => {
        const body = buildNotificationBody(params);
        const expected = `Swap of ${params.fromAmount} ${params.fromAsset} → ${params.toAmount} ${params.toAsset} failed.`;
        expect(body).toBe(expected);
      }),
      { numRuns: 100 },
    );
  });

  it('returns exactly the dropped template for any dropped params', () => {
    fc.assert(
      fc.property(notificationParamsArb('dropped'), (params) => {
        const body = buildNotificationBody(params);
        const expected = `Swap of ${params.fromAmount} ${params.fromAsset} → ${params.toAmount} ${params.toAsset} was dropped. You may resubmit.`;
        expect(body).toBe(expected);
      }),
      { numRuns: 100 },
    );
  });
});

// ---------------------------------------------------------------------------
// Property 4: No notification dispatched when preference false or permission not granted
// ---------------------------------------------------------------------------

// Feature: browser-transaction-notifications, Property 4: No notification dispatched when preference false or permission not granted
describe('Property 4: No notification dispatched when preference false or permission not granted', () => {
  let NotificationMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    NotificationMock = vi.fn();
    Object.defineProperty(NotificationMock, 'permission', {
      value: 'granted',
      configurable: true,
      writable: true,
    });
    (window as Record<string, unknown>).Notification = NotificationMock;
  });

  afterEach(() => {
    delete (window as Record<string, unknown>).Notification;
    vi.restoreAllMocks();
  });

  it('does not construct Notification and does not throw when preference.enabled is false', () => {
    fc.assert(
      fc.property(notificationParamsArb(), (params) => {
        NotificationMock.mockClear();
        expect(() =>
          dispatchTransactionNotification(params, { enabled: false }),
        ).not.toThrow();
        expect(NotificationMock).not.toHaveBeenCalled();
      }),
      { numRuns: 100 },
    );
  });

  it('does not construct Notification and does not throw when permission is "denied"', () => {
    fc.assert(
      fc.property(notificationParamsArb(), (params) => {
        NotificationMock.mockClear();
        Object.defineProperty(NotificationMock, 'permission', {
          value: 'denied',
          configurable: true,
        });
        expect(() =>
          dispatchTransactionNotification(params, { enabled: true }),
        ).not.toThrow();
        expect(NotificationMock).not.toHaveBeenCalled();
      }),
      { numRuns: 100 },
    );
  });

  it('does not construct Notification and does not throw when permission is "default"', () => {
    fc.assert(
      fc.property(notificationParamsArb(), (params) => {
        NotificationMock.mockClear();
        Object.defineProperty(NotificationMock, 'permission', {
          value: 'default',
          configurable: true,
        });
        expect(() =>
          dispatchTransactionNotification(params, { enabled: true }),
        ).not.toThrow();
        expect(NotificationMock).not.toHaveBeenCalled();
      }),
      { numRuns: 100 },
    );
  });
});

// ---------------------------------------------------------------------------
// Property 5: Dispatch does not mutate transaction params
// ---------------------------------------------------------------------------

// Feature: browser-transaction-notifications, Property 5: Dispatch does not mutate transaction params
describe('Property 5: Dispatch does not mutate transaction params', () => {
  afterEach(() => {
    delete (window as Record<string, unknown>).Notification;
    vi.restoreAllMocks();
  });

  it('leaves params deeply equal to original after dispatch (denied permission)', () => {
    fc.assert(
      fc.property(notificationParamsArb(), (params) => {
        // Use denied permission so no Notification is constructed
        const NotificationMock = vi.fn();
        Object.defineProperty(NotificationMock, 'permission', {
          value: 'denied',
          configurable: true,
        });
        (window as Record<string, unknown>).Notification = NotificationMock;

        const snapshot = { ...params };
        dispatchTransactionNotification(params, { enabled: true });

        expect(params.status).toBe(snapshot.status);
        expect(params.txHash).toBe(snapshot.txHash);
        expect(params.fromAsset).toBe(snapshot.fromAsset);
        expect(params.fromAmount).toBe(snapshot.fromAmount);
        expect(params.toAsset).toBe(snapshot.toAsset);
        expect(params.toAmount).toBe(snapshot.toAmount);
        expect(params.txId).toBe(snapshot.txId);
      }),
      { numRuns: 100 },
    );
  });

  it('leaves params deeply equal to original after dispatch (granted permission)', () => {
    fc.assert(
      fc.property(notificationParamsArb(), (params) => {
        const NotificationMock = vi.fn();
        Object.defineProperty(NotificationMock, 'permission', {
          value: 'granted',
          configurable: true,
        });
        (window as Record<string, unknown>).Notification = NotificationMock;

        const snapshot = { ...params };
        dispatchTransactionNotification(params, { enabled: true });

        expect(params.status).toBe(snapshot.status);
        expect(params.txHash).toBe(snapshot.txHash);
        expect(params.fromAsset).toBe(snapshot.fromAsset);
        expect(params.fromAmount).toBe(snapshot.fromAmount);
        expect(params.toAsset).toBe(snapshot.toAsset);
        expect(params.toAmount).toBe(snapshot.toAmount);
        expect(params.txId).toBe(snapshot.txId);
      }),
      { numRuns: 100 },
    );
  });
});

// ---------------------------------------------------------------------------
// Property 6: Notification tag equals transaction id
// ---------------------------------------------------------------------------

// Feature: browser-transaction-notifications, Property 6: Notification tag equals transaction id
describe('Property 6: Notification tag equals transaction id for any dispatched notification', () => {
  let NotificationMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    NotificationMock = vi.fn();
    Object.defineProperty(NotificationMock, 'permission', {
      value: 'granted',
      configurable: true,
    });
    (window as Record<string, unknown>).Notification = NotificationMock;
  });

  afterEach(() => {
    delete (window as Record<string, unknown>).Notification;
    vi.restoreAllMocks();
  });

  it('Notification constructor receives options.tag === params.txId', () => {
    fc.assert(
      fc.property(notificationParamsArb(), (params) => {
        NotificationMock.mockClear();
        dispatchTransactionNotification(params, { enabled: true });

        expect(NotificationMock).toHaveBeenCalledOnce();
        const [, options] = NotificationMock.mock.calls[0] as [
          string,
          NotificationOptions,
        ];
        expect(options.tag).toBe(params.txId);
      }),
      { numRuns: 100 },
    );
  });
});

// ---------------------------------------------------------------------------
// Property 9: Explorer URL contains transaction hash for any non-empty hash
// ---------------------------------------------------------------------------

// Feature: browser-transaction-notifications, Property 9: Explorer URL contains transaction hash for any non-empty hash
describe('Property 9: Explorer URL contains transaction hash for any non-empty hash', () => {
  it('starts with the base URL and contains txHash verbatim', () => {
    fc.assert(
      fc.property(nonEmptyStringArb, (txHash) => {
        const url = buildExplorerUrl(txHash);
        expect(url.startsWith('https://stellar.expert/explorer/public/tx/')).toBe(true);
        expect(url).toContain(txHash);
      }),
      { numRuns: 100 },
    );
  });
});

// ---------------------------------------------------------------------------
// Property 10: Correct notification title for each terminal status
// ---------------------------------------------------------------------------

// Feature: browser-transaction-notifications, Property 10: Correct notification title for each terminal status
describe('Property 10: Correct notification title for each terminal status', () => {
  const expectedTitles: Record<TerminalStatus, string> = {
    confirmed: 'Swap Confirmed',
    failed: 'Swap Failed',
    dropped: 'Swap Dropped',
  };

  it('buildNotificationTitle returns the correct title for any terminal status', () => {
    fc.assert(
      fc.property(terminalStatusArb, (status) => {
        expect(buildNotificationTitle(status)).toBe(expectedTitles[status]);
      }),
      { numRuns: 100 },
    );
  });

  it('dispatchTransactionNotification passes the correct title to Notification constructor', () => {
    const NotificationMock = vi.fn();
    Object.defineProperty(NotificationMock, 'permission', {
      value: 'granted',
      configurable: true,
    });
    (window as Record<string, unknown>).Notification = NotificationMock;

    try {
      fc.assert(
        fc.property(notificationParamsArb(), (params) => {
          NotificationMock.mockClear();
          dispatchTransactionNotification(params, { enabled: true });

          expect(NotificationMock).toHaveBeenCalledOnce();
          const [title] = NotificationMock.mock.calls[0] as [string];
          expect(title).toBe(expectedTitles[params.status]);
        }),
        { numRuns: 100 },
      );
    } finally {
      delete (window as Record<string, unknown>).Notification;
    }
  });
});
