/**
 * Property-based tests for BrowserNotificationSettings component.
 * Uses fast-check@3.22.0.
 *
 * Feature: browser-transaction-notifications
 */

import * as fc from 'fast-check';
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { BrowserNotificationSettings } from './BrowserNotificationSettings';

afterEach(() => {
  cleanup();
});

// ---------------------------------------------------------------------------
// Property 8: Toggle aria-label describes current state for any boolean value
// ---------------------------------------------------------------------------

// Feature: browser-transaction-notifications, Property 8: Toggle aria-label describes current state for any boolean value
describe('Property 8: Toggle aria-label describes current state for any boolean value', () => {
  it('aria-label is non-empty for any browserNotifications boolean when toggle is not disabled', () => {
    fc.assert(
      fc.property(fc.boolean(), (browserNotifications) => {
        const { unmount } = render(
          <BrowserNotificationSettings
            browserNotifications={browserNotifications}
            permissionState="granted"
            isDisabled={false}
            onEnable={vi.fn().mockResolvedValue(undefined)}
            onDisable={vi.fn()}
          />,
        );

        const toggle = screen.getByRole('switch');
        const label = toggle.getAttribute('aria-label');

        expect(label).toBeTruthy();
        expect(label!.length).toBeGreaterThan(0);

        // Label should describe the current state
        if (browserNotifications) {
          expect(label).toContain('enabled');
        } else {
          expect(label).toContain('disabled');
        }

        unmount();
      }),
      { numRuns: 100 },
    );
  });
});
