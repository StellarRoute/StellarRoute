import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { BrowserNotificationSettings } from './BrowserNotificationSettings';

afterEach(() => {
  cleanup();
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function renderComponent(
  overrides: Partial<React.ComponentProps<typeof BrowserNotificationSettings>> = {},
) {
  const defaults = {
    browserNotifications: false,
    permissionState: 'default' as NotificationPermission,
    isDisabled: false,
    onEnable: vi.fn().mockResolvedValue(undefined),
    onDisable: vi.fn(),
  };
  return render(<BrowserNotificationSettings {...defaults} {...overrides} />);
}

// ---------------------------------------------------------------------------
// Rendering states
// ---------------------------------------------------------------------------

describe('BrowserNotificationSettings — rendering', () => {
  it('renders an enabled toggle when browserNotifications=true and permission is granted', () => {
    renderComponent({
      browserNotifications: true,
      permissionState: 'granted',
      isDisabled: false,
    });

    const toggle = screen.getByRole('switch');
    expect(toggle).toBeInTheDocument();
    expect(toggle).toHaveAttribute('aria-checked', 'true');
    expect(toggle).not.toBeDisabled();
  });

  it('renders a disabled toggle with "blocked by browser" label when permissionState is "denied"', () => {
    renderComponent({
      browserNotifications: false,
      permissionState: 'denied',
      isDisabled: true,
    });

    const toggle = screen.getByRole('switch');
    expect(toggle).toBeDisabled();
    expect(toggle).toHaveAttribute(
      'aria-label',
      expect.stringContaining('blocked by browser'),
    );
  });

  it('renders a disabled toggle with "not supported" label when permissionState is "unsupported"', () => {
    renderComponent({
      browserNotifications: false,
      permissionState: 'unsupported' as NotificationPermission | 'unsupported',
      isDisabled: true,
    });

    const toggle = screen.getByRole('switch');
    expect(toggle).toBeDisabled();
    expect(toggle).toHaveAttribute(
      'aria-label',
      expect.stringContaining('not supported'),
    );
  });

  it('renders an interactive toggle when not disabled', () => {
    renderComponent({ isDisabled: false });

    const toggle = screen.getByRole('switch');
    expect(toggle).not.toBeDisabled();
  });
});

// ---------------------------------------------------------------------------
// Interaction
// ---------------------------------------------------------------------------

describe('BrowserNotificationSettings — interaction', () => {
  it('calls onEnable when toggle is clicked and notifications are currently disabled', async () => {
    const onEnable = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();

    renderComponent({
      browserNotifications: false,
      isDisabled: false,
      onEnable,
    });

    await user.click(screen.getByRole('switch'));
    expect(onEnable).toHaveBeenCalledOnce();
  });

  it('calls onDisable when toggle is clicked and notifications are currently enabled', async () => {
    const onDisable = vi.fn();
    const user = userEvent.setup();

    renderComponent({
      browserNotifications: true,
      permissionState: 'granted',
      isDisabled: false,
      onDisable,
    });

    await user.click(screen.getByRole('switch'));
    expect(onDisable).toHaveBeenCalledOnce();
  });

  it('does not call onEnable or onDisable when toggle is disabled', async () => {
    const onEnable = vi.fn().mockResolvedValue(undefined);
    const onDisable = vi.fn();
    const user = userEvent.setup();

    renderComponent({
      isDisabled: true,
      permissionState: 'denied',
      onEnable,
      onDisable,
    });

    await user.click(screen.getByRole('switch'));
    expect(onEnable).not.toHaveBeenCalled();
    expect(onDisable).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// Accessibility
// ---------------------------------------------------------------------------

describe('BrowserNotificationSettings — accessibility', () => {
  it('aria-label is present and non-empty in all states', () => {
    const states: Array<
      Partial<React.ComponentProps<typeof BrowserNotificationSettings>>
    > = [
      { browserNotifications: false, permissionState: 'default', isDisabled: false },
      { browserNotifications: true, permissionState: 'granted', isDisabled: false },
      { browserNotifications: false, permissionState: 'denied', isDisabled: true },
      {
        browserNotifications: false,
        permissionState: 'unsupported' as NotificationPermission | 'unsupported',
        isDisabled: true,
      },
    ];

    for (const state of states) {
      const { unmount } = renderComponent(state);
      const toggle = screen.getByRole('switch');
      const label = toggle.getAttribute('aria-label');
      expect(label).toBeTruthy();
      expect(label!.length).toBeGreaterThan(0);
      unmount();
    }
  });
});
