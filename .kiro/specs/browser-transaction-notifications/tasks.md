# Implementation Plan: Browser Transaction Notifications

## Overview

Implement opt-in browser push notifications for swap terminal states (`confirmed`, `failed`, `dropped`). The work is split into four incremental layers: pure utility functions, the React hook, the UI toggle component, and finally wiring everything into the existing transaction lifecycle and settings panel.

## Tasks

- [x] 1. Implement `notificationManager` pure utility module
  - Create `frontend/lib/notifications.ts` with all exported types and functions
  - Implement `isNotificationSupported()` — checks `typeof window !== 'undefined' && 'Notification' in window`
  - Implement `buildNotificationTitle(status)` — returns `"Swap Confirmed"` | `"Swap Failed"` | `"Swap Dropped"`
  - Implement `buildNotificationBody(params)` — constructs body string per the three status templates
  - Implement `buildExplorerUrl(txHash)` — returns `"https://stellar.expert/explorer/public/tx/{txHash}"`
  - Implement `dispatchTransactionNotification(params, preference)` — guards on preference, API availability, and `Notification.permission`; wraps `new Notification()` in try/catch; does not mutate params
  - Export `TerminalStatus`, `NotificationParams`, and `NotificationPreference` types
  - _Requirements: 1.1, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 4.1, 4.2, 4.4, 4.5, 6.1, 6.2, 6.3, 6.4, 6.5_

  - [x]* 1.1 Write unit tests for `notificationManager`
    - Create `frontend/lib/notifications.test.ts`
    - Test `buildNotificationTitle` for all three terminal statuses
    - Test `buildNotificationBody` for each status with concrete example values
    - Test `buildExplorerUrl` with a concrete hash
    - Test `dispatchTransactionNotification` does not call `new Notification()` when `preference.enabled` is `false`
    - Test `dispatchTransactionNotification` does not call `new Notification()` when `Notification.permission` is `'denied'`
    - Test `dispatchTransactionNotification` does not throw when `window.Notification` is undefined
    - Test `dispatchTransactionNotification` sets `tag`, `icon`, and `data.url` correctly for a confirmed transaction
    - Test `isNotificationSupported` returns `false` when `window.Notification` is absent
    - _Requirements: 1.1, 3.7, 4.1, 4.2, 4.4, 6.4, 6.5_

  - [x]* 1.2 Write property tests for `notificationManager`
    - Create `frontend/lib/notifications.property.test.ts`
    - **Property 1: Notification body contains all swap summary fields for any terminal status**
    - **Validates: Requirements 3.6, 6.6**
    - **Property 2: Confirmed notification body matches exact template**
    - **Validates: Requirements 6.1**
    - **Property 3: Failed and dropped notification bodies match exact templates**
    - **Validates: Requirements 6.2, 6.3**
    - **Property 4: No notification dispatched when preference false or permission not granted**
    - **Validates: Requirements 3.7, 4.1, 4.2, 4.4**
    - **Property 5: Dispatch does not mutate transaction params**
    - **Validates: Requirements 4.5**
    - **Property 6: Notification tag equals transaction id**
    - **Validates: Requirements 6.4**
    - **Property 9: Explorer URL contains transaction hash for any non-empty hash**
    - **Validates: Requirements 3.5**
    - **Property 10: Correct notification title for each terminal status**
    - **Validates: Requirements 3.1, 3.2, 3.3**

- [x] 2. Checkpoint — Ensure all `notifications.ts` tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 3. Implement `useBrowserNotifications` hook
  - Create `frontend/hooks/useBrowserNotifications.ts`
  - On mount: read `localStorage.getItem('stellarroute.settings.browserNotifications')` inside try/catch; read `Notification.permission`; if permission is `'denied'` override preference to `false` (do not write back); set `isHydrated = true` via `queueMicrotask` (mirrors `useExpertSettings` pattern)
  - Implement `enableNotifications()`: write `'true'` to localStorage, update in-memory preference, call `Notification.requestPermission()`, handle `'granted'` / `'denied'` / `'default'` results, write `'false'` back on non-grant
  - Implement `disableNotifications()`: write `'false'` to localStorage, update in-memory preference, do NOT call `requestPermission()`
  - Derive `isDisabled` from `permissionState === 'denied' || permissionState === 'unsupported'`
  - Export `BrowserNotificationsState` interface and `useBrowserNotifications` function
  - _Requirements: 1.2, 1.3, 1.4, 1.5, 1.6, 2.1, 2.2, 2.3, 2.4, 2.5, 5.1, 5.2, 5.3, 7.1, 7.2, 7.3_

  - [x]* 3.1 Write unit tests for `useBrowserNotifications`
    - Create `frontend/hooks/useBrowserNotifications.test.ts`
    - Test initialises with `false` when localStorage key is absent
    - Test restores `true` from localStorage on mount
    - Test overrides to `false` when `Notification.permission === 'denied'` regardless of localStorage value
    - Test `enableNotifications()` calls `Notification.requestPermission()` exactly once
    - Test `enableNotifications()` sets preference to `false` when permission is denied
    - Test `disableNotifications()` does NOT call `Notification.requestPermission()`
    - Test localStorage errors are caught and preference defaults to `false`
    - _Requirements: 1.2, 1.3, 1.4, 1.5, 2.2, 2.4, 2.5, 5.2, 5.3_

  - [x]* 3.2 Write property tests for `useBrowserNotifications`
    - Create `frontend/hooks/useBrowserNotifications.property.test.ts`
    - **Property 7: Preference persistence round-trip**
    - **Validates: Requirements 2.2, 2.3**

- [x] 4. Checkpoint — Ensure all `useBrowserNotifications` tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 5. Implement `BrowserNotificationSettings` component
  - Create `frontend/components/settings/BrowserNotificationSettings.tsx`
  - Accept `BrowserNotificationSettingsProps`: `browserNotifications`, `permissionState`, `isDisabled`, `onEnable`, `onDisable`
  - When `isDisabled && permissionState === 'denied'`: render disabled toggle with `aria-label` containing `"blocked by browser"`
  - When `isDisabled && permissionState === 'unsupported'`: render disabled toggle with `aria-label` containing `"not supported"`
  - Otherwise: render interactive toggle matching the visual style of the Expert Mode toggle in `ExpertSettings.tsx`; `aria-label` describes current state (e.g. `"Browser notifications: enabled"` / `"Browser notifications: disabled"`)
  - Use `Bell` / `BellOff` icons from `lucide-react`
  - _Requirements: 5.1, 5.4, 5.5, 5.6, 7.3_

  - [x]* 5.1 Write unit tests for `BrowserNotificationSettings`
    - Create `frontend/components/settings/BrowserNotificationSettings.test.tsx`
    - Test renders enabled toggle when `browserNotifications=true` and permission is granted
    - Test renders disabled toggle with "blocked by browser" label when `permissionState='denied'`
    - Test renders disabled toggle with "not supported" label when `permissionState='unsupported'`
    - Test clicking the toggle calls `onEnable` or `onDisable` appropriately
    - Test `aria-label` is present and non-empty in all states
    - _Requirements: 5.4, 5.5, 5.6_

  - [x]* 5.2 Write property tests for `BrowserNotificationSettings`
    - Create `frontend/components/settings/BrowserNotificationSettings.property.test.tsx`
    - **Property 8: Toggle aria-label describes current state for any boolean value**
    - **Validates: Requirements 5.6**

- [x] 6. Checkpoint — Ensure all `BrowserNotificationSettings` tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 7. Wire `useBrowserNotifications` into `SettingsPanel`
  - Modify `frontend/components/settings/SettingsPanel.tsx`
  - Import and call `useBrowserNotifications()` inside the panel (or accept its state as props — match the existing pattern used by `ExpertSettings`)
  - Render `<BrowserNotificationSettings>` inside the settings drawer alongside the existing sections, passing `browserNotifications`, `permissionState`, `isDisabled`, `onEnable`, and `onDisable`
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_

- [x] 8. Wire `dispatchTransactionNotification` into `useTransactionLifecycle`
  - Modify `frontend/hooks/useTransactionLifecycle.ts`
  - Add `notificationPreference?: NotificationPreference` to `UseTransactionLifecycleOptions` (defaults to `{ enabled: false }`)
  - Import `dispatchTransactionNotification` and `NotificationParams` from `frontend/lib/notifications.ts`
  - After each terminal state transition (`confirmed`, `failed`, `dropped`), construct `NotificationParams` from `tradeParams`, `txHash`, `txId`, and `status`, then call `dispatchTransactionNotification(params, notificationPreference)`
  - _Requirements: 3.8, 4.3_

  - [x]* 8.1 Write integration tests for `useTransactionLifecycle` notification dispatch
    - Add integration test cases to the existing `useTransactionLifecycle` test file (or create a new one if none exists)
    - Mock `dispatchTransactionNotification`; run the hook through `confirmed`, `failed`, and `dropped` transitions
    - Assert the mock was called exactly once per terminal transition with correct `NotificationParams`
    - _Requirements: 3.8_

- [x] 9. Final checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for a faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation at each layer boundary
- Property tests use `fast-check@3.22.0` (already in `devDependencies`) with a minimum of 100 iterations per property
- Each property test file uses the tag format: `// Feature: browser-transaction-notifications, Property {N}: {title}`
- The `notificationManager` module has no React dependencies, making it independently testable without `renderHook`
- `useBrowserNotifications` follows the `useExpertSettings` initialisation pattern (try/catch localStorage, `queueMicrotask` for hydration)
- The `lucide-react` mock at `frontend/__mocks__/lucide-react.tsx` covers `Bell` and `BellOff` icons in tests
