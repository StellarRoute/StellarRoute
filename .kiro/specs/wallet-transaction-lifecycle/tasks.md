# Implementation Plan: Wallet Transaction Lifecycle UX

## Overview

Standardise the full swap transaction lifecycle across `TransactionConfirmationModal` and `TransactionHistory`. The work proceeds in dependency order: update the shared type first, then build new shared primitives (`ExplorerLink`, `useFocusTrap`, `useTransactionLifecycle`), then refactor the modal, then update the history hook and activity view, and finally add the property-based and unit/integration test suites.

## Tasks

- [x] 1. Update `TransactionStatus` type and fix all call-sites
  - In `frontend/types/transaction.ts`, replace the five old values (`pending`, `submitting`, `processing`, `success`, `failed`) with the five canonical values: `pending`, `submitted`, `confirmed`, `failed`, `dropped`
  - Search the entire `frontend/` tree for references to the removed values (`submitting`, `processing`, `success`) and update each one to the nearest equivalent new value (`submitted`, `submitted`/`confirmed`, `confirmed`)
  - Update `getStatusBadge` in `frontend/components/TransactionHistory.tsx` to compile without errors after the rename (full badge logic is addressed in task 7)
  - Update the `handleOpenChange` guard in `frontend/components/shared/TransactionConfirmationModal.tsx` that currently checks `status === 'success'` to check `status === 'confirmed'`
  - Ensure `npm run build` (or `tsc --noEmit`) passes with zero type errors before proceeding
  - _Requirements: 1.1_

- [x] 2. Create `ExplorerLink` shared component
  - Create `frontend/components/shared/ExplorerLink.tsx`
  - Accept props `{ hash: string; className?: string }`
  - Render an `<a>` with `href="https://stellar.expert/explorer/public/tx/{hash}"`, `target="_blank"`, `rel="noreferrer noopener"`, and `aria-label="View transaction {hash.slice(0, 8)} on Stellar Expert"`
  - Display the text "View on Stellar Expert" followed by an `<ExternalLink />` icon from lucide-react
  - Return `null` when `hash` is an empty string or falsy (guard inside the component)
  - Export from `frontend/components/shared/index.ts`
  - _Requirements: 3.3, 3.4, 3.5, 3.6, 3.7_

  - [ ]* 2.1 Write property test for `ExplorerLink` URL construction
    - **Property 4: ExplorerLink URL is well-formed for any non-empty hash**
    - **Validates: Requirements 3.3, 3.6**
    - Create `frontend/components/shared/ExplorerLink.test.tsx`
    - Use `fc.stringOf(fc.char(), { minLength: 1, maxLength: 64 })` to generate arbitrary hashes
    - Assert `href === "https://stellar.expert/explorer/public/tx/{hash}"`, `target="_blank"`, and `rel` contains both `"noreferrer"` and `"noopener"`
    - Also assert `aria-label === "View transaction {hash.slice(0, 8)} on Stellar Expert"`
    - Assert nothing is rendered when `hash` is `""`
    - Tag: `// Feature: wallet-transaction-lifecycle, Property 4: ExplorerLink URL is well-formed for any non-empty hash`

- [x] 3. Create `useFocusTrap` hook
  - Create `frontend/hooks/useFocusTrap.ts`
  - Signature: `function useFocusTrap(containerRef: React.RefObject<HTMLElement>, active: boolean): void`
  - When `active` is `true`, attach a `keydown` listener to the container that intercepts `Tab` and `Shift+Tab`
  - Query focusable descendants with the selector: `a[href], button:not([disabled]), input, select, textarea, [tabindex]:not([tabindex="-1"])`
  - On `Tab` from the last focusable element, move focus to the first; on `Shift+Tab` from the first, move focus to the last
  - Remove the listener when `active` becomes `false` or the component unmounts
  - _Requirements: 4.7_

  - [ ]* 3.1 Write property test for `useFocusTrap` cycling behaviour
    - **Property 7: Focus trap keeps Tab cycling within the modal**
    - **Validates: Requirements 4.7**
    - Create `frontend/hooks/useFocusTrap.test.ts`
    - Use `fc.integer({ min: 1, max: 10 })` to generate arbitrary counts of focusable buttons inside a container
    - Activate the trap, press Tab N times, assert focus never leaves the container and wraps correctly
    - Also assert Shift+Tab from the first element wraps to the last
    - Tag: `// Feature: wallet-transaction-lifecycle, Property 7: Focus trap keeps Tab cycling within the modal`

- [x] 4. Create `useTransactionLifecycle` hook
  - Create `frontend/hooks/useTransactionLifecycle.ts`
  - Implement the interface from the design: `{ status, txHash, errorMessage, tradeParams, initiateSwap, cancel, resubmit, tryAgain, dismiss }`
  - `initiateSwap`: sets status to `pending`, calls `useWallet` to sign the XDR, on rejection sets status to `failed` with `errorMessage = "Signature rejected. You can try again or dismiss."`, on success calls Horizon submit, sets status to `submitted`, then `confirmed` on success or `failed` on error
  - Start a `setTimeout` (default 60 s, configurable via `deadlineMs` parameter) when entering `submitted`; if it fires while still `submitted`, transition to `dropped` and clear the timer on any earlier terminal transition
  - `cancel`: if status is `pending`, abort the signing request and reset to `review`
  - `resubmit`: if status is `dropped`, re-run the submission flow from `pending`
  - `tryAgain`: reset status to `review` with original `tradeParams` preserved
  - `dismiss`: reset status to `review` and clear `txHash`/`errorMessage`
  - Internally call `useTransactionHistory` to persist each status transition via `addTransaction` / `updateTransactionStatus`
  - _Requirements: 1.2, 1.3, 1.4, 1.5, 1.6, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

- [x] 5. Refactor `TransactionConfirmationModal`
  - [x] 5.1 Add new props and update the props interface
    - Add `onTryAgain`, `onResubmit`, `onDismiss`, `onDone` callbacks to `TransactionConfirmationModalProps` in `frontend/components/shared/TransactionConfirmationModal.tsx`
    - Update `status` type to `TransactionStatus | 'review'` using the new five-value `TransactionStatus`
    - _Requirements: 1.2, 1.3, 1.4, 1.5, 1.6_

  - [x] 5.2 Implement `submitted` and `dropped` state UI panels
    - Add a `submitted` panel: spinner + "Submitting to network…" heading + `aria-live="polite"` region + `ExplorerLink` when `txHash` is present
    - Add a `dropped` panel: timeout icon + "Transaction Dropped" heading + explanation text + "Resubmit" button (calls `onResubmit`) + "Dismiss" button (calls `onDismiss`)
    - Rename the existing `success` panel to `confirmed`: update heading to "Swap Confirmed!", replace the inline explorer anchor with `<ExplorerLink hash={txHash} />` placed immediately below the hash display, add a "Done" button that calls `onDone`
    - Update the `failed` panel to add a "Try Again" button (calls `onTryAgain`) alongside the existing "Dismiss" button; update "Dismiss" to call `onDismiss`
    - Add a "Cancel" button to the `pending` panel that calls `onCancel`
    - _Requirements: 1.3, 1.4, 1.5, 1.6, 2.1, 2.2, 2.3, 2.4, 2.5, 3.1, 3.2_

  - [x] 5.3 Add accessibility attributes
    - Add a visually-hidden `<p id="modal-state-desc">` inside the modal that contains a plain-language description of the current state; wire it to the modal root via `aria-describedby="modal-state-desc"`
    - Add an `aria-live="polite"` region (separate from the description paragraph) for transient announcements
    - Add `tabIndex={-1}` to the modal container element so it can receive programmatic focus in the `submitted` state
    - _Requirements: 4.3, 4.9, 4.10_

  - [x] 5.4 Implement per-state focus management
    - Create a `ref` for each focus target: `confirmBtnRef` (review), `cancelBtnRef` (pending), `containerRef` (submitted), `doneBtnRef` (confirmed), `tryAgainBtnRef` (failed), `resubmitBtnRef` (dropped)
    - Add a `useEffect` that fires on `status` changes and calls `.focus()` on the appropriate ref
    - Wire `useFocusTrap(containerRef, isOpen)` to keep Tab cycling within the modal while it is open
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7_

  - [x] 5.5 Implement Escape key behaviour
    - Replace the current `handleOpenChange` guard with a `keydown` listener on the modal container
    - For `review`, `confirmed`, `failed`, `dropped`: call `onOpenChange(false)` on Escape
    - For `pending`, `submitted`: suppress the close and instead write "Transaction in progress. Use the Cancel button to abort." into the `aria-live` region
    - _Requirements: 4.8, 4.9_

  - [ ]* 5.6 Write property test for Escape key behaviour
    - **Property 6: Escape key closes the modal only in non-in-flight states**
    - **Validates: Requirements 4.8, 4.9**
    - Create or extend `frontend/components/shared/TransactionConfirmationModal.test.tsx`
    - Use `fc.constantFrom('review', 'confirmed', 'failed', 'dropped')` for closeable states and `fc.constantFrom('pending', 'submitted')` for in-flight states
    - Fire a `keydown` Escape event; assert `onOpenChange(false)` is called for closeable states and NOT called for in-flight states
    - Assert the `aria-live` region contains the expected announcement for in-flight states
    - Tag: `// Feature: wallet-transaction-lifecycle, Property 6: Escape key closes the modal only in non-in-flight states`

  - [ ]* 5.7 Write property test for recovery actions presence
    - **Property 5: Recovery actions and Retry buttons are present for every non-success terminal state**
    - **Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5**
    - Use `fc.constantFrom('failed', 'dropped', 'pending')` to generate modal status values
    - Assert "Try Again" + "Dismiss" are present for `failed`; "Resubmit" + "Dismiss" for `dropped`; "Cancel" for `pending`
    - Tag: `// Feature: wallet-transaction-lifecycle, Property 5: Recovery actions and Retry buttons are present for every non-success terminal state`

  - [ ]* 5.8 Write property test for `aria-describedby`
    - **Property 8: Modal aria-describedby points to a non-empty description for every state**
    - **Validates: Requirements 4.10**
    - Use `fc.constantFrom('review', 'pending', 'submitted', 'confirmed', 'failed', 'dropped')` to generate all modal states
    - Render the modal for each status; assert the root element has `aria-describedby` and the referenced element has non-empty `textContent`
    - Tag: `// Feature: wallet-transaction-lifecycle, Property 8: Modal aria-describedby points to a non-empty description for every state`

- [x] 6. Update `useTransactionHistory` hook
  - [x] 6.1 Add `downgradePendingOnReload` pure function
    - Add the exported pure function `downgradePendingOnReload(records: TransactionRecord[]): TransactionRecord[]` to `frontend/hooks/useTransactionHistory.ts`
    - It maps any record with `status === 'pending'` or `status === 'submitted'` to `{ ...tx, status: 'dropped' }`; all other records pass through unchanged
    - _Requirements: 6.3_

  - [x] 6.2 Apply downgrade on initialisation and wallet-address change
    - In the lazy `useState` initialiser, wrap the `JSON.parse` result with `downgradePendingOnReload` before returning
    - In the `walletAddress`-change `useEffect`, wrap the parsed records with `downgradePendingOnReload` before calling `setTransactions`
    - _Requirements: 6.2, 6.3_

  - [ ]* 6.3 Write property test for `downgradePendingOnReload`
    - **Property 2: Reload downgrades in-flight statuses to `dropped`**
    - **Validates: Requirements 6.3**
    - Create `frontend/hooks/useTransactionHistory.test.ts`
    - Use `fc.array(fc.record({ ...fields, status: fc.constantFrom('pending', 'submitted', 'confirmed', 'failed', 'dropped') }))` to generate arbitrary record arrays
    - Apply `downgradePendingOnReload`; assert every previously-`pending`/`submitted` record is now `dropped` and all others are unchanged
    - Tag: `// Feature: wallet-transaction-lifecycle, Property 2: Reload downgrades in-flight statuses to dropped`

  - [ ]* 6.4 Write property test for localStorage round-trip
    - **Property 1: localStorage round-trip preserves all record fields**
    - **Validates: Requirements 6.4**
    - Use `fc.record` with `fc.constantFrom(...statuses)` for `status` and `fc.option(fc.string())` for optional fields to generate arbitrary `TransactionRecord` objects
    - Serialise with `JSON.stringify`, deserialise with `JSON.parse`, assert deep equality across all fields
    - Tag: `// Feature: wallet-transaction-lifecycle, Property 1: localStorage round-trip preserves all record fields`

  - [ ]* 6.5 Write property test for persistence round-trip via hook
    - **Property 10: Persistence round-trip — write then reload restores all records**
    - **Validates: Requirements 6.1, 6.2, 6.3**
    - Use `fc.array(fc.record(...))` with mixed statuses; write records via `addTransaction`, re-initialise the hook (simulating a page reload by re-running the lazy initialiser against the same `localStorage` key), assert all records are restored with `pending`/`submitted` downgraded to `dropped` and all others unchanged
    - Tag: `// Feature: wallet-transaction-lifecycle, Property 10: Persistence round-trip — write then reload restores all records`

- [x] 7. Update `TransactionHistory` / Activity View
  - [x] 7.1 Update `getStatusBadge` for all five statuses
    - In `frontend/components/TransactionHistory.tsx`, rewrite `getStatusBadge` to handle `pending`, `submitted`, `confirmed`, `failed`, `dropped` with distinct visual styles (e.g., secondary for pending/submitted, success for confirmed, destructive for failed, outline/warning for dropped)
    - Add `aria-label={"Status: " + status}` to every badge element returned by `getStatusBadge`
    - _Requirements: 1.7, 5.4_

  - [ ]* 7.2 Write property test for status badge accessible label
    - **Property 3: Status badge renders a non-empty accessible label for every valid status**
    - **Validates: Requirements 1.7, 5.4**
    - Create or extend `frontend/components/TransactionHistory.test.tsx`
    - Use `fc.constantFrom('pending', 'submitted', 'confirmed', 'failed', 'dropped')` to generate status values
    - Render the badge; assert `aria-label === "Status: {status}"` and visible text is non-empty
    - Tag: `// Feature: wallet-transaction-lifecycle, Property 3: Status badge renders a non-empty accessible label for every valid status`

  - [x] 7.3 Add Explorer column using `ExplorerLink`
    - Replace the inline `<a>` in the Explorer column with `<ExplorerLink hash={tx.hash ?? ""} />` (the component itself guards against empty strings)
    - Render a `<span>` dash (`—`) when `tx.hash` is absent, as before
    - _Requirements: 3.6, 3.7, 5.3_

  - [ ]* 7.4 Write property test for Activity View `ExplorerLink` aria-label
    - **Property 9: Activity View ExplorerLink aria-label encodes the short hash**
    - **Validates: Requirements 5.3**
    - Use `fc.stringOf(fc.char(), { minLength: 1, maxLength: 64 })` to generate arbitrary non-empty hashes
    - Render a transaction row with that hash; assert the `<a>` element has `aria-label === "View transaction {hash.slice(0, 8)} on Stellar Expert"`
    - Tag: `// Feature: wallet-transaction-lifecycle, Property 9: Activity View ExplorerLink aria-label encodes the short hash`

  - [x] 7.5 Add Retry column for `failed` and `dropped` rows
    - Add a "Retry" column header to the table in `frontend/components/TransactionHistory.tsx`
    - For rows where `tx.status === 'failed' || tx.status === 'dropped'`, render a `<button>` with `aria-label={"Retry " + tx.fromAsset + "→" + tx.toAsset + " swap from " + new Date(tx.timestamp).toLocaleDateString()}`
    - For all other rows, render an empty cell
    - Wire the button's `onClick` to a `onRetry` prop (or a callback from `useTransactionHistory`) — the prop can be a no-op stub at this stage; full wiring happens in task 8
    - _Requirements: 2.7, 5.2_

  - [ ]* 7.6 Write property test for Retry button presence
    - **Property 5 (Activity View portion): Retry button present for failed/dropped records**
    - **Validates: Requirements 2.7, 5.2**
    - Use `fc.record` with `fc.constantFrom('failed', 'dropped')` for status and arbitrary asset/timestamp fields
    - Render the row; assert a `<button>` is present whose `aria-label` contains `fromAsset`, `toAsset`, and a date string
    - Tag: `// Feature: wallet-transaction-lifecycle, Property 5: Recovery actions and Retry buttons are present for every non-success terminal state`

  - [x] 7.7 Update skeleton accessibility attributes
    - In `frontend/components/shared/ActivityTableSkeleton.tsx`, add `aria-busy="true"` and `aria-label="Loading transaction history"` to the outermost table container element
    - In `frontend/components/TransactionHistory.tsx`, ensure the skeleton is rendered inside the same container that will hold the table (no layout shift)
    - _Requirements: 5.5_

- [x] 8. Wire `useTransactionLifecycle` into `SwapCard` and `TransactionHistory`
  - In `frontend/components/swap/SwapCard.tsx`, replace the existing ad-hoc status state with `useTransactionLifecycle`; pass `status`, `txHash`, `errorMessage`, `tradeParams`, `onTryAgain`, `onResubmit`, `onDismiss`, `onCancel`, `onDone` through to `TransactionConfirmationModal`
  - In `frontend/components/TransactionHistory.tsx`, pass an `onRetry` callback to each Retry button that calls `useTransactionLifecycle.resubmit` (or opens the modal pre-populated) for the given record
  - Remove the "Demo mode" banners from the modal now that the real lifecycle hook drives state
  - _Requirements: 1.2, 1.3, 1.4, 1.5, 1.6, 2.1, 2.2, 2.3, 2.4, 2.5_

- [x] 9. Checkpoint — ensure all tests pass
  - Run `npm run test -- --run` from `frontend/` and confirm zero failures
  - Fix any TypeScript errors surfaced by `tsc --noEmit`
  - Ask the user if any questions arise before proceeding to integration tests

- [ ] 10. Write unit tests for new hooks and components
  - [ ]* 10.1 Unit tests for `TransactionConfirmationModal` — one test per state
    - In `frontend/components/shared/TransactionConfirmationModal.test.tsx`, add one `it` block per status value (`review`, `pending`, `submitted`, `confirmed`, `failed`, `dropped`) verifying the correct heading, action buttons, and `aria-label` values are rendered
    - _Requirements: 1.2, 1.3, 1.4, 1.5, 1.6, 2.1, 2.2, 2.3, 2.4, 2.5_

  - [ ]* 10.2 Unit tests for `ExplorerLink`
    - Verify URL construction, `rel` attribute, and `aria-label` for a concrete hash
    - Verify nothing is rendered for an empty string hash
    - _Requirements: 3.3, 3.4, 3.5_

  - [ ]* 10.3 Unit tests for `downgradePendingOnReload` — example-based
    - One test per status value confirming the expected output (pending → dropped, submitted → dropped, confirmed → confirmed, failed → failed, dropped → dropped)
    - _Requirements: 6.3_

  - [ ]* 10.4 Unit tests for `useFocusTrap`
    - Verify Tab cycles within the container and does not escape
    - Verify Shift+Tab wraps from first to last element
    - Verify the trap is inactive when `active` is `false`
    - _Requirements: 4.7_

  - [ ]* 10.5 Unit tests for `ActivityTableSkeleton` accessibility
    - Verify `aria-busy="true"` and `aria-label="Loading transaction history"` are present on the table container during loading
    - _Requirements: 5.5_

- [x] 11. Final checkpoint — ensure all tests pass
  - Run `npm run test -- --run` from `frontend/` and confirm zero failures
  - Ensure `npm run build` completes without errors
  - Ask the user if any questions arise

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Property tests use `fast-check` (already in `devDependencies`) with a minimum of 100 iterations each
- Tag format for property tests: `// Feature: wallet-transaction-lifecycle, Property {N}: {title}`
- Unit tests use Vitest + `@testing-library/react`; icon imports are auto-mocked via `frontend/__mocks__/lucide-react.tsx`
- The `review` state is a UI-only discriminant in the modal's local state and is not part of `TransactionStatus`
