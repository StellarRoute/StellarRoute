'use client';

import { useCallback, useEffect, useState } from 'react';

const STORAGE_KEY = 'stellarroute.settings.browserNotifications';

export interface BrowserNotificationsState {
  /** Whether the user has opted in (in-memory, reconciled with browser permission) */
  browserNotifications: boolean;
  /** Current browser permission state, or "unsupported" if API is unavailable */
  permissionState: NotificationPermission | 'unsupported';
  /** Whether the toggle should be disabled (permission denied or API unsupported) */
  isDisabled: boolean;
  /** Whether the hook has completed hydration from localStorage */
  isHydrated: boolean;
  /** Enable notifications: persists preference and calls requestPermission() */
  enableNotifications: () => Promise<void>;
  /** Disable notifications: persists preference, does NOT call requestPermission() */
  disableNotifications: () => void;
}

function getInitialPermissionState(): NotificationPermission | 'unsupported' {
  if (typeof window === 'undefined' || !('Notification' in window)) {
    return 'unsupported';
  }
  return Notification.permission;
}

export function useBrowserNotifications(): BrowserNotificationsState {
  const [browserNotifications, setBrowserNotifications] = useState(false);
  const [permissionState, setPermissionState] = useState<
    NotificationPermission | 'unsupported'
  >('default');
  const [isHydrated, setIsHydrated] = useState(false);

  // Hydrate from localStorage on mount, reconcile with browser permission state
  useEffect(() => {
    if (typeof window === 'undefined') return;

    const currentPermission = getInitialPermissionState();
    setPermissionState(currentPermission);

    try {
      const stored =
        localStorage.getItem(STORAGE_KEY) === 'true';

      queueMicrotask(() => {
        // If browser has denied permission, override stored preference to false
        // (do NOT write back to localStorage per spec)
        const effective =
          currentPermission === 'denied' || currentPermission === 'unsupported'
            ? false
            : stored;
        setBrowserNotifications(effective);
        setIsHydrated(true);
      });
    } catch (e) {
      console.error(
        'Failed to load browser notification preference from localStorage',
        e,
      );
      queueMicrotask(() => {
        setBrowserNotifications(false);
        setIsHydrated(true);
      });
    }
  }, []);

  const enableNotifications = useCallback(async () => {
    if (typeof window === 'undefined' || !('Notification' in window)) {
      return;
    }

    // Persist optimistic true first, then reconcile after permission result
    try {
      localStorage.setItem(STORAGE_KEY, 'true');
    } catch (e) {
      console.error(
        'Failed to persist browser notification preference to localStorage',
        e,
      );
    }
    setBrowserNotifications(true);

    try {
      const result = await Notification.requestPermission();
      setPermissionState(result);

      if (result !== 'granted') {
        // Permission not granted — revert preference
        setBrowserNotifications(false);
        try {
          localStorage.setItem(STORAGE_KEY, 'false');
        } catch (e) {
          console.error(
            'Failed to persist browser notification preference to localStorage',
            e,
          );
        }
      }
    } catch (e) {
      console.error('Notification.requestPermission() failed', e);
      setBrowserNotifications(false);
      try {
        localStorage.setItem(STORAGE_KEY, 'false');
      } catch (lsErr) {
        console.error(
          'Failed to persist browser notification preference to localStorage',
          lsErr,
        );
      }
    }
  }, []);

  const disableNotifications = useCallback(() => {
    setBrowserNotifications(false);
    try {
      localStorage.setItem(STORAGE_KEY, 'false');
    } catch (e) {
      console.error(
        'Failed to persist browser notification preference to localStorage',
        e,
      );
    }
    // Do NOT call Notification.requestPermission()
  }, []);

  const isDisabled =
    permissionState === 'denied' || permissionState === 'unsupported';

  return {
    browserNotifications,
    permissionState,
    isDisabled,
    isHydrated,
    enableNotifications,
    disableNotifications,
  };
}
