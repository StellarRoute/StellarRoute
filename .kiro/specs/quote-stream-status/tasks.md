# Implementation Plan: Real-time Connection Status Indicator for Quotes Stream

## Overview

Implement `useQuoteStreamStatus` (debounce-based derivation hook) and `QuoteStreamStatusIndicator` (controlled presentational component), then wire them into `SwapCard` replacing the existing inline "Retrying quote..." span. The implementation follows the TypeScript/React patterns already established in `frontend/hooks/` and `frontend/components/swap/`.

## Tasks

- [x] 1. Define shared types for the feature
  - Create `frontend/hooks/useQuoteStreamStatus.ts` with the exported type definitions: `ConnectionStatus`, `Mode`, `UseQuoteStreamStatusOptions`, `UseQuoteStreamStatusInputs`, `UseQuoteStreamStatusResult`
  - Export the pure `deriveRawStatus` helper function that maps `(isRecovering, error, isOnline)` → `ConnectionStatus` with no side effects
  - _Requirements: 1.1, 1.2, 1.3, 3.1, 6.1_

- [x] 2. Implement `useQuoteStreamStatus` hook
  - [x] 2.1 Implement the hook body with debounce timer logic
    - Use `useState` for the emitted `status` (default `"connected"`) and `useRef` for the pending timer ID
    - Implement the `connected → reconnecting` grace-period debounce: start a `setTimeout` for `reconnectGracePeriodMs` (default 3000 ms) when raw status becomes `"reconnecting"`; cancel and reset the timer if raw status returns to `"connected"` before it fires
    - Implement immediate transitions: `* → disconnected` (when `isOnline` becomes `false`) and `reconnecting → connected` both bypass the grace period and update state synchronously inside the effect
    - Handle undefined inputs by defaulting to safe values (`isRecovering: false`, `error: null`, `isOnline: true`) so the hook emits `"connected"` before `useQuoteRefresh` is mounted
    - Default `mode` to `"polling"` to match the current `useQuoteRefresh` implementation
    - Clean up the timer in the `useEffect` return function to prevent memory leaks on unmount
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 3.4, 4.4, 6.2, 6.3_

  - [ ]* 2.2 Write property test — Property 1: Status derivation correctness
    - Create `frontend/hooks/useQuoteStreamStatus.property.test.ts`
    - Use `vi.useFakeTimers()` and `renderHook` from `@testing-library/react`
    - **Property 1: Status Derivation Correctness** — for any `(isRecovering: boolean, error: Error | null, isOnline: boolean, gracePeriodMs: number)`, after advancing fake timers past `gracePeriodMs`, the emitted status matches `deriveRawStatus(isRecovering, error, isOnline)`
    - Use `fc.boolean()`, `fc.option(fc.constant(new Error("e")), { nil: null })`, `fc.boolean()`, `fc.nat({ max: 10000 })` as arbitraries; `numRuns: 100`
    - Tag: `// Feature: quote-stream-status, Property 1: Status derivation correctness`
    - **Validates: Requirements 1.1, 1.2, 1.3, 1.8**

  - [ ]* 2.3 Write property test — Property 2: Flicker suppression
    - **Property 2: Flicker Suppression** — for any `gracePeriodMs` and `recoveryDelayMs < gracePeriodMs`, when inputs transition `connected → reconnecting` then back to `connected` before the grace period elapses, the emitted status is `"connected"` throughout and `"reconnecting"` is never observed
    - Use `fc.nat({ max: 5000 })` for `gracePeriodMs` and `fc.nat({ max: 4999 })` for `recoveryDelayMs`; filter to ensure `recoveryDelayMs < gracePeriodMs`; `numRuns: 100`
    - Tag: `// Feature: quote-stream-status, Property 2: Flicker suppression`
    - **Validates: Requirements 1.4, 1.5, 4.1**

  - [ ]* 2.4 Write property test — Property 3: Immediate offline transition
    - **Property 3: Immediate Offline Transition** — for any prior status (`"connected"` or `"reconnecting"`), when `isOnline` transitions to `false`, the emitted status immediately becomes `"disconnected"` without advancing timers
    - Use `fc.constantFrom("connected", "reconnecting")` as arbitrary; `numRuns: 100`
    - Tag: `// Feature: quote-stream-status, Property 3: Immediate offline transition`
    - **Validates: Requirements 1.7, 4.3**

  - [ ]* 2.5 Write property test — Property 4: Immediate recovery transition
    - **Property 4: Immediate Recovery Transition** — for any `gracePeriodMs`, after the grace period has elapsed and status is `"reconnecting"`, when inputs change to the `connected` derivation, the emitted status immediately becomes `"connected"` without an additional debounce
    - Use `fc.nat({ max: 10000 })` as arbitrary; `numRuns: 100`
    - Tag: `// Feature: quote-stream-status, Property 4: Immediate recovery transition`
    - **Validates: Requirements 4.2**

  - [ ]* 2.6 Write property test — Property 5: Single timer per grace period
    - **Property 5: Single Timer Per Grace Period** — for any number of rapid `connected → reconnecting` transitions (2–10) within a grace period window, the hook maintains only one active timer; the grace period fires exactly once after the last transition, not once per transition
    - Use `fc.integer({ min: 2, max: 10 })` and `fc.nat({ max: 5000 })`; `numRuns: 100`
    - Tag: `// Feature: quote-stream-status, Property 5: Single timer per grace period`
    - **Validates: Requirements 4.4**

  - [ ]* 2.7 Write property test — Property 7: Mode change immediacy
    - **Property 7: Mode Change Immediacy** — for any prior connection status and any initial mode, when the `mode` input changes, the returned `mode` value immediately reflects the new value without waiting for the grace period
    - Use `fc.constantFrom("connected", "reconnecting")` and `fc.constantFrom("stream", "polling")`; `numRuns: 100`
    - Tag: `// Feature: quote-stream-status, Property 7: Mode change immediacy`
    - **Validates: Requirements 3.5**

  - [ ]* 2.8 Write property test — Property 8: Hook determinism
    - **Property 8: Hook Determinism** — calling `deriveRawStatus(isRecovering, error, isOnline)` twice with identical arguments always returns the same `ConnectionStatus`
    - Use `fc.boolean()`, `fc.option(fc.constant(new Error("e")), { nil: null })`, `fc.boolean()`; `numRuns: 100`
    - Tag: `// Feature: quote-stream-status, Property 8: Hook determinism`
    - **Validates: Requirements 6.2**

  - [ ]* 2.9 Write unit tests for `useQuoteStreamStatus`
    - Create `frontend/hooks/useQuoteStreamStatus.test.ts`
    - Test: hook emits `"connected"` by default when no inputs are provided (Requirement 6.3)
    - Test: hook uses 3000 ms grace period when `reconnectGracePeriodMs` is not provided (Requirement 1.6)
    - Test: hook emits `mode: "polling"` by default (Requirement 3.4)
    - Test: status becomes `"disconnected"` immediately when `isOnline` becomes `false` (Requirement 1.7)
    - Test: status becomes `"connected"` immediately when recovering from `"reconnecting"` (Requirement 4.2)
    - Test: timer is cleaned up on unmount (no state updates after unmount)
    - _Requirements: 1.6, 1.7, 3.4, 4.2, 6.3_

- [x] 3. Checkpoint — hook layer complete
  - Ensure all tests in `useQuoteStreamStatus.test.ts` and `useQuoteStreamStatus.property.test.ts` pass; ask the user if questions arise.

- [x] 4. Implement `QuoteStreamStatusIndicator` component
  - [x] 4.1 Create the component file and implement all visual states
    - Create `frontend/components/swap/QuoteStreamStatusIndicator.tsx`
    - Implement the five visual states from the design table: `connected/stream` (green dot, "Live"), `connected/polling` (blue dot, "Polling"), `reconnecting/stream` (amber pulsing dot, "Reconnecting"), `reconnecting/polling` (amber pulsing dot, "Reconnecting (polling)"), `disconnected/any` (red dot, "Disconnected")
    - Detect `prefers-reduced-motion: reduce` via `window.matchMedia` (the mock is already in `frontend/vitest.setup.ts`); replace the pulse animation with a static dot when active
    - Return `null` when `hideWhenConnected` is `true` and `status` is `"connected"`
    - Use Tailwind CSS classes consistent with existing swap components
    - _Requirements: 2.1, 2.2, 2.3, 2.5, 2.6, 2.7, 3.2, 3.3_

  - [x] 4.2 Add accessibility attributes to the component
    - Add `aria-label="Quote stream status: {label}"` on the root element (e.g., `"Quote stream status: Live"`)
    - Add `aria-live="polite"` for `connected` and `reconnecting` states; `aria-live="assertive"` for `disconnected`
    - Add `aria-hidden="true"` on the decorative dot `<span>`
    - Ensure the text label `<span>` is always rendered alongside the dot so colour is never the sole indicator
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_

  - [ ]* 4.3 Write property test — Property 6: Accessibility completeness
    - Add property test to `frontend/components/swap/QuoteStreamStatusIndicator.property.test.ts`
    - **Property 6: Accessibility Completeness** — for any `(status: ConnectionStatus, mode: Mode)` combination, the rendered component includes: an `aria-label` containing the full status description, a non-empty visible text label, and `aria-hidden="true"` on the dot element
    - Use `fc.constantFrom("connected", "reconnecting", "disconnected")` and `fc.constantFrom("stream", "polling")`; `numRuns: 100`
    - Tag: `// Feature: quote-stream-status, Property 6: Accessibility completeness`
    - **Validates: Requirements 5.1, 5.5, 5.6**

  - [ ]* 4.4 Write unit tests for `QuoteStreamStatusIndicator`
    - Create `frontend/components/swap/QuoteStreamStatusIndicator.test.tsx`
    - Test all five visual states: correct dot colour class and label text for each `(status, mode)` combination (Requirements 2.1–2.3, 3.2–3.3)
    - Test `hideWhenConnected={true}` returns null when `status="connected"` (Requirement 2.5)
    - Test pulse animation class is applied when `status="reconnecting"` and `prefers-reduced-motion` is false (Requirement 2.6)
    - Test pulse animation class is absent when `prefers-reduced-motion: reduce` is active — override `window.matchMedia` mock to return `matches: true` (Requirement 2.7)
    - Test `aria-live="polite"` when `status="reconnecting"` (Requirement 5.2)
    - Test `aria-live="assertive"` when `status="disconnected"` (Requirement 5.3)
    - Test `aria-live="polite"` when `status="connected"` (Requirement 5.4)
    - _Requirements: 2.1, 2.2, 2.3, 2.5, 2.6, 2.7, 3.2, 3.3, 5.2, 5.3, 5.4_

- [x] 5. Checkpoint — component layer complete
  - Ensure all tests in `QuoteStreamStatusIndicator.test.tsx` and `QuoteStreamStatusIndicator.property.test.ts` pass; ask the user if questions arise.

- [x] 6. Integrate into `SwapCard`
  - [x] 6.1 Wire `useQuoteStreamStatus` into `SwapCard`
    - Import `useQuoteStreamStatus` and `QuoteStreamStatusIndicator` in `frontend/components/swap/SwapCard.tsx`
    - Import `useOnlineStatus` from `frontend/hooks/useOnlineStatus.ts`
    - Call `useOnlineStatus()` to get `isOnline`
    - Call `useQuoteStreamStatus({ isRecovering: quote.isRecovering, error: quote.error, isOnline })` and destructure `{ status: streamStatus, mode: streamMode }`
    - _Requirements: 6.1, 6.4, 6.5_

  - [x] 6.2 Render `QuoteStreamStatusIndicator` in the header row and remove the old inline span
    - Add `<QuoteStreamStatusIndicator status={streamStatus} mode={streamMode} />` inside the header `<div className="flex items-center gap-1">`, adjacent to the existing refresh `<Button>`
    - Remove the `{quote.isRecovering && <span data-testid="recovering-indicator">Retrying quote...</span>}` block (replaced by the new indicator)
    - Keep the existing `{quote.isStale && <span data-testid="stale-indicator">…</span>}` block — it is a separate concern
    - _Requirements: 2.4, 6.5_

  - [ ]* 6.3 Write integration tests for `SwapCard` wiring
    - Update `frontend/components/swap/SwapCard.test.tsx`
    - Test: `QuoteStreamStatusIndicator` is rendered in the header row (query by `aria-label` matching `"Quote stream status:"`) (Requirement 2.4)
    - Test: when `useQuoteRefresh` mock returns `isRecovering: true`, the indicator shows the "Reconnecting" label (Requirement 6.5)
    - Test: when `useQuoteRefresh` mock returns `isRecovering: false` and `error: null`, the indicator shows the "Polling" label (default mode) (Requirement 3.4, 6.5)
    - Test: the old `data-testid="recovering-indicator"` span is no longer rendered (replaced by the new component)
    - _Requirements: 2.4, 3.4, 6.5_

- [x] 7. Export the new component from the swap barrel
  - Add `QuoteStreamStatusIndicator` to `frontend/components/swap/index.ts` so it is accessible via the barrel export
  - _Requirements: 6.4_

- [x] 8. Final checkpoint — Ensure all tests pass
  - Run `npm run test --run` in `frontend/` and confirm all new and existing tests pass; ask the user if questions arise.

## Notes

- Sub-tasks marked with `*` are optional and can be skipped for a faster MVP
- Property tests use `vi.useFakeTimers()` / `vi.advanceTimersByTime()` to control the debounce timer deterministically; restore real timers in `afterEach`
- The `window.matchMedia` mock in `frontend/vitest.setup.ts` returns `matches: false` by default; override it per-test for reduced-motion coverage
- Icon mocking: if any icon imports cause test failures, check `frontend/__mocks__/lucide-react.tsx`
- `mode` defaults to `"polling"` throughout because `useQuoteRefresh` uses HTTP polling; the `"stream"` path is reserved for future WebSocket integration
- All 8 correctness properties from the design document are covered by property tests in tasks 2.2–2.8 and 4.3
