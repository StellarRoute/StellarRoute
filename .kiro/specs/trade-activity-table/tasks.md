# Implementation Plan: Trade Activity Table

## Overview

Implement the `TradeActivityTable` React component and its supporting pieces in the
StellarRoute frontend. Work proceeds bottom-up: formatting utilities → data hook →
presentational sub-components → root table → integration into `history/page.tsx`.

## Tasks

- [ ] 1. Create formatting utilities
  - [ ] 1.1 Create `frontend/lib/trade-format.ts` with `truncateTxHash`, `formatTradeTimestamp`, `formatTradeAmount`, and `stellarExplorerUrl`
    - `truncateTxHash`: returns `hash.slice(0,8) + "…" + hash.slice(-4)` for hashes longer than 12 chars, otherwise returns the hash unchanged
    - `formatTradeTimestamp`: formats a `Date` as `"YYYY-MM-DD HH:mm UTC"` using UTC getters
    - `formatTradeAmount`: parses the decimal string, formats to at most 7 decimal places, strips trailing zeros
    - `stellarExplorerUrl`: returns `` `https://stellar.expert/explorer/public/tx/${txHash}` ``
    - _Requirements: 1.2, 1.3, 1.4, 1.5_

  - [ ]* 1.2 Write property tests for formatting utilities in `frontend/lib/trade-format.test.ts`
    - **Property 1: TxHash truncation format** — `fc.string({ minLength: 13 })`
    - **Validates: Requirements 1.2**
    - **Property 3: Timestamp format** — `fc.date()`
    - **Validates: Requirements 1.4**
    - **Property 4: Amount formatting** — `fc.float({ min: 0 })` mapped to string
    - **Validates: Requirements 1.5**
    - Each test must include comment `// Feature: trade-activity-table, Property N: <text>`

- [ ] 2. Define shared types
  - [ ] 2.1 Create `frontend/components/shared/TradeActivityTable.types.ts` exporting `TradeStatus`, `TradeRecord`, `SortColumn`, `SortDirection`, and `SortState`
    - Match the exact shapes defined in the design document
    - _Requirements: 1.1, 2.1, 2.2, 2.3, 5.1_

- [ ] 3. Implement `useTradeActivity` hook
  - [ ] 3.1 Create `frontend/hooks/useTradeActivity.ts`
    - Accept `{ walletAddress, pageSize = 20 }` options
    - Read from `useTransactionHistory`, map `TransactionRecord` → `TradeRecord` (hash fallback to id, capitalise status, build pair string)
    - Apply client-side sort (timestamp: Date numeric, amount: parseFloat, status: lexicographic) without mutating source array
    - Slice sorted array for current page: `sortedRecords.slice((page-1)*pageSize, page*pageSize)`
    - Reset `currentPage` to 1 whenever `sortState` changes
    - Return `{ records, totalRecords, currentPage, totalPages, sortState, isLoading, setPage, setSortState }`
    - Default sort: `{ column: 'timestamp', direction: 'desc' }`
    - Return empty records when `walletAddress` is null
    - _Requirements: 5.1, 5.2, 5.3, 5.5, 6.1, 6.6_

  - [ ]* 3.2 Write property tests for `useTradeActivity` in `frontend/hooks/useTradeActivity.test.ts`
    - **Property 6: Sorted order invariant** — `fc.array(tradeRecordArb)` for each `SortColumn`
    - **Validates: Requirements 5.2, 5.3**
    - **Property 7: Page size invariant** — `fc.array(tradeRecordArb, { minLength: 21 })`
    - **Validates: Requirements 6.1**
    - **Property 9: Sort change resets page** — `fc.record({ column: sortColumnArb, direction: sortDirArb })`
    - **Validates: Requirements 6.6**
    - Include unit tests: default sort is timestamp descending (Req 5.5)

- [ ] 4. Implement `StatusBadge` component
  - [ ] 4.1 Create `frontend/components/shared/StatusBadge.tsx`
    - Use shadcn/ui `Badge` primitive
    - `Pending` → yellow scheme, `Success` → green scheme, `Failed` → red scheme, unknown → neutral grey with label `"Unknown"`
    - Set `aria-label="Trade status: <label>"` on the badge element
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

  - [ ]* 4.2 Write property tests for `StatusBadge` in `frontend/components/shared/StatusBadge.test.tsx`
    - **Property 5: Status badge label and aria-label** — `fc.oneof(fc.constant('Pending'), fc.constant('Success'), fc.constant('Failed'), fc.string())`
    - **Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5**

- [ ] 5. Implement `SortableHeader` component
  - [ ] 5.1 Create `frontend/components/shared/SortableHeader.tsx`
    - Render a `<th>` with a clickable/keyboard-activatable header
    - Show a sort-direction arrow icon on the active column
    - Set `aria-sort="ascending"` or `"descending"` on the active column, `"none"` on all other sortable headers
    - Handle `onClick`, `onKeyDown` (Enter/Space) to call `onSort(column)`
    - _Requirements: 5.2, 5.3, 5.4, 7.3, 7.4_

- [ ] 6. Implement `LoadingSkeleton` and `EmptyState` components
  - [ ] 6.1 Create `frontend/components/shared/LoadingSkeleton.tsx`
    - Use shadcn/ui `Skeleton` primitive
    - Render exactly 5 placeholder rows matching the 6-column table layout
    - _Requirements: 3.1, 3.2_

  - [ ] 6.2 Create `frontend/components/shared/EmptyState.tsx`
    - Render a `<tr>` with a single `<td colSpan={6}>` spanning the full table width
    - Display the message `"No trades yet. Execute a swap to see your activity here."`
    - _Requirements: 4.1, 4.2, 4.3_

- [ ] 7. Implement `PaginationControls` component
  - [ ] 7.1 Create `frontend/components/shared/PaginationControls.tsx`
    - Use shadcn/ui `Button` primitives
    - Render previous, page-number, and next controls
    - Disable "previous" on page 1; disable "next" on last page
    - Add `aria-label` to each control (e.g. `"Previous page"`, `"Next page"`, `"Page N"`)
    - _Requirements: 6.2, 6.3, 6.4, 7.5_

- [ ] 8. Implement root `TradeActivityTable` component
  - [ ] 8.1 Create `frontend/components/shared/TradeActivityTable.tsx`
    - Accept `{ walletAddress, pageSize? }` props
    - Call `useTradeActivity` hook
    - Render a semantic `<table>` with `<caption>Trade Activity</caption>`, `<thead>`, and `<tbody>`
    - While loading: render `<LoadingSkeleton>` inside `<tbody>`
    - When empty: render `<EmptyState>` inside `<tbody>`
    - When populated: render one `<tr>` per `TradeRecord` with columns TxHash, Pair, Side, Amount, Timestamp, StatusBadge in that order
    - TxHash cell: anchor to `stellarExplorerUrl(txHash)` with `target="_blank"`; use `truncateTxHash` when viewport < 768 px (via `window.innerWidth` or a `useMediaQuery` hook)
    - Format Timestamp with `formatTradeTimestamp`; format Amount with `formatTradeAmount`
    - Wire `SortableHeader` for Timestamp, Amount, Status columns
    - Render `PaginationControls` only when `totalRecords > pageSize`
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 3.1, 3.3, 4.1, 5.4, 6.5, 7.1, 7.2_

  - [ ]* 8.2 Write property tests for `TradeActivityTable` in `frontend/components/shared/TradeActivityTable.test.tsx`
    - **Property 2: TxHash explorer URL** — `fc.hexaString({ minLength: 64, maxLength: 64 })`
    - **Validates: Requirements 1.3**
    - **Property 8: Pagination visibility threshold** — `fc.array(tradeRecordArb)`
    - **Validates: Requirements 6.5**
    - **Property 10: Keyboard sort activation** — `fc.constantFrom('timestamp','amount','status')`
    - **Validates: Requirements 7.3**
    - **Property 11: aria-sort correctness** — `fc.record({ column: sortColumnArb, direction: sortDirArb })`
    - **Validates: Requirements 7.4**
    - Include unit tests: loading skeleton renders 5 rows (Req 3.2), skeleton replaced after fetch (Req 3.3), empty state message text (Req 4.2), first page disables previous (Req 6.3), last page disables next (Req 6.4), `<caption>` text (Req 7.2), semantic elements present (Req 7.1), pagination aria-labels (Req 7.5), aria-sort on initial render (Req 5.4)

- [ ] 9. Checkpoint — Ensure all tests pass
  - Run the test suite and confirm all passing. Ask the user if any questions arise before proceeding.

- [ ] 10. Integrate into history page
  - [ ] 10.1 Update `frontend/app/history/page.tsx` to import and render `<TradeActivityTable>`, passing the current wallet address
    - Remove or replace any existing placeholder content in that page
    - _Requirements: 1.1, 3.1, 4.1_

- [ ] 11. Final checkpoint — Ensure all tests pass
  - Run the full test suite and confirm everything is green. Ask the user if any questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for a faster MVP
- Each task references specific requirements for traceability
- Property tests use **fast-check** (`npm install --save-dev fast-check`)
- Each property test must include the comment `// Feature: trade-activity-table, Property N: <text>`
- Property tests run a minimum of 100 iterations (fast-check default)
