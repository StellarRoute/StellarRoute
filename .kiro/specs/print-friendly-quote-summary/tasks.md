# Implementation Plan: Print-Friendly Quote Summary

## Overview

Implement a CSS-first print layout for the StellarRoute swap card. The work is additive: two new components (`PrintHeader`, `PrintButton`), one new utility (`formatPrintTimestamp`), a `@media print` block in `globals.css`, and targeted `print:` Tailwind class additions to four existing components. No new data-fetching or state management is required.

## Tasks

- [ ] 1. Add `@media print` global styles to `globals.css`
  - Append a `@media print` block to `frontend/app/globals.css` containing:
    - `@page { size: portrait; margin: 1cm; }` rule
    - `print-color-adjust: exact` and `-webkit-print-color-adjust: exact` on `*` to preserve meaningful colour (price impact severity)
    - `[data-testid="swap-card"] .card-bg-gradient { display: none !important; }` to suppress animated gradient `<div>` elements
    - `box-shadow: none`, `backdrop-filter: none`, `background: white` on `[data-testid="swap-card"] > div`
    - `font-size: 10pt`, `max-width: 100%`, `width: 100%` on `[data-testid="swap-card"]`
    - `body { display: block; }` for single-column layout
  - _Requirements: 6.1, 6.2, 6.3, 6.5, 6.6, 2.6, 7.3, 7.4_

- [ ] 2. Create `formatPrintTimestamp` utility and its tests
  - [ ] 2.1 Implement `formatPrintTimestamp` in `frontend/lib/print-utils.ts`
    - Export `formatPrintTimestamp(isoString: string): string`
    - Parse the input with `new Date(isoString)` and extract UTC components
    - Return a string in the format `YYYY-MM-DD HH:mm UTC` with zero-padded month, day, hour, and minute
    - Return `"Invalid Date"` when the input does not parse to a valid date
    - _Requirements: 4.2_

  - [ ]* 2.2 Write property test for `formatPrintTimestamp` — Property 6
    - **Property 6: formatPrintTimestamp produces correctly formatted output**
    - **Validates: Requirements 4.2**
    - File: `frontend/lib/print-utils.test.ts`
    - Use `fc.date()` from fast-check to generate arbitrary valid `Date` objects, convert each to an ISO string, and assert the output matches `/^\d{4}-\d{2}-\d{2} \d{2}:\d{2} UTC$/`
    - Also assert the year, month, day, hour, and minute components in the output match the UTC values of the input date
    - Run 100 iterations (fast-check default)
    - Tag: `// Feature: print-friendly-quote-summary, Property 6: formatPrintTimestamp produces correctly formatted output`
    - _Requirements: 4.2_

- [ ] 3. Create `PrintHeader` component and its tests
  - [ ] 3.1 Implement `PrintHeader` in `frontend/components/swap/PrintHeader.tsx`
    - Accept props: `capturedAt: string`, `fromSymbol: string`, `toSymbol: string`
    - Apply `hidden print:block` to the root element so it is invisible on screen and visible only in print view
    - Display the text `"StellarRoute"` as the document title
    - Display the trading pair as `{fromSymbol} → {toSymbol}`
    - Display the formatted timestamp by calling `formatPrintTimestamp(capturedAt)` from `frontend/lib/print-utils.ts`
    - _Requirements: 4.1, 4.2, 4.3, 4.4_

  - [ ]* 3.2 Write unit tests for `PrintHeader`
    - File: `frontend/components/swap/__tests__/PrintHeader.test.tsx`
    - Example: renders `"StellarRoute"` text
    - Example: renders `fromSymbol` and `toSymbol` separated by `→`
    - Example: root element has both `hidden` and `print:block` classes
    - Example: renders the formatted timestamp string returned by `formatPrintTimestamp`
    - _Requirements: 4.1, 4.2, 4.3, 4.4_

- [ ] 4. Create `PrintButton` component and its tests
  - [ ] 4.1 Implement `PrintButton` in `frontend/components/swap/PrintButton.tsx`
    - Accept prop: `disabled?: boolean`
    - Apply `print:hidden` class so the button never appears in the printed output
    - Set `aria-label="Print quote summary"` on the button element
    - On click, guard with `typeof window !== 'undefined' && typeof window.print === 'function'` before calling `window.print()`
    - When `disabled` is `true`, the click handler must be a no-op (do not call `window.print()`)
    - _Requirements: 5.2, 5.3, 5.4, 5.5_

  - [ ]* 4.2 Write unit tests for `PrintButton`
    - File: `frontend/components/swap/__tests__/PrintButton.test.tsx`
    - Example: renders a button with `aria-label="Print quote summary"`
    - Example: calls `window.print()` on click when `disabled` is `false` or omitted
    - Example: does NOT call `window.print()` on click when `disabled` is `true`
    - Example: root element has `print:hidden` class
    - _Requirements: 5.2, 5.3, 5.4, 5.5_

- [ ] 5. Checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 6. Add print classes to `PriceInfoPanel`
  - Wrap the `PriceSparkline` section in a `<div className="print:hidden">` (hides sparkline in print view)
  - Wrap the export buttons `<div>` in a `<div className="print:hidden">` (hides export JSON/CSV buttons in print view)
  - Add `print:break-inside-avoid` to the outer container `<div>` (prevents page breaks inside the panel)
  - Add `print:border-black/80` to the outer container `<div>` (ensures border is visible on paper)
  - _Requirements: 1.7, 1.10, 6.4, 6.6_

  - [ ]* 6.1 Write property test for `PriceInfoPanel` — Property 2
    - **Property 2: PriceInfoPanel renders the minimum received value**
    - **Validates: Requirements 2.4**
    - File: `frontend/components/swap/__tests__/PriceInfoPanel.test.tsx` (create if absent)
    - Use `fc.string({ minLength: 1 })` to generate arbitrary non-empty `minReceived` strings and assert each is present in the rendered DOM
    - Run 100 iterations
    - Tag: `// Feature: print-friendly-quote-summary, Property 2: PriceInfoPanel renders the minimum received value`
    - _Requirements: 2.4_

  - [ ]* 6.2 Write class-presence unit tests for `PriceInfoPanel`
    - File: `frontend/components/swap/__tests__/PriceInfoPanel.test.tsx`
    - Example: sparkline wrapper has `print:hidden` class
    - Example: export buttons wrapper has `print:hidden` class
    - Example: outer container has `print:break-inside-avoid` class
    - _Requirements: 1.7, 1.10, 6.4_

- [ ] 7. Add print classes to `FeeBreakdownPanel`
  - Add `print:break-inside-avoid` to the outer container `<div>` (prevents page breaks inside the panel)
  - Add `print:border-black/80` to the outer container `<div>` (ensures border is visible on paper)
  - _Requirements: 6.4, 6.6_

  - [ ]* 7.1 Write property test for `FeeBreakdownPanel` — Property 3
    - **Property 3: FeeBreakdownPanel renders total fee and net output**
    - **Validates: Requirements 2.7**
    - File: `frontend/components/swap/__tests__/FeeBreakdownPanel.test.tsx` (create if absent)
    - Use `fc.string({ minLength: 1 })` for both `totalFee` and `netOutput`; assert both strings appear in the rendered DOM
    - Run 100 iterations
    - Tag: `// Feature: print-friendly-quote-summary, Property 3: FeeBreakdownPanel renders total fee and net output`
    - _Requirements: 2.7_

  - [ ]* 7.2 Write class-presence unit tests for `FeeBreakdownPanel`
    - File: `frontend/components/swap/__tests__/FeeBreakdownPanel.test.tsx`
    - Example: outer container has `print:break-inside-avoid` class
    - _Requirements: 6.4_

- [ ] 8. Add `forcePrintExpanded` prop and print classes to `RouteDisplay`
  - Add `forcePrintExpanded?: boolean` prop (default `false`) to `RouteDisplayProps`
  - In the detail drawer render condition, change `{showDetails && selectedRoute && (…)}` to `{(showDetails || forcePrintExpanded) && selectedRoute && (…)}` so per-hop details always render when `forcePrintExpanded` is `true`
  - Wrap the entire "Alternative Routes" section (heading + scroll container) in `<div className="print:hidden">`
  - Add `print:hidden` class to the chevron expand/collapse `<button>`
  - Add `print:hidden` class to the `extendedRouteDetails` diagnostics block `<div data-testid="extended-diagnostics">`
  - Add `print:break-inside-avoid` to the outer container `<div data-testid="route-display">`
  - _Requirements: 3.4, 3.5, 3.6, 3.7, 6.4_

  - [ ]* 8.1 Write property test for `RouteDisplay` route summary — Property 4
    - **Property 4: RouteDisplay renders route summary fields**
    - **Validates: Requirements 3.1**
    - File: `frontend/components/swap/RouteDisplay.test.tsx` (extend existing file)
    - Use `fc.record({ fromAsset: fc.string({ minLength: 1 }), venue: fc.string({ minLength: 1 }), toAsset: fc.string({ minLength: 1 }) })` to generate route objects; render `RouteDisplay` with a single `alternativeRoute` built from those fields and assert all three values appear in the DOM
    - Run 100 iterations
    - Tag: `// Feature: print-friendly-quote-summary, Property 4: RouteDisplay renders route summary fields`
    - _Requirements: 3.1_

  - [ ]* 8.2 Write property test for `RouteDisplay` hop fee total — Property 5
    - **Property 5: RouteDisplay per-hop fee total equals sum of hop fees**
    - **Validates: Requirements 3.3**
    - File: `frontend/components/swap/RouteDisplay.test.tsx`
    - Use `fc.array(fc.float({ min: 0, max: 1, noNaN: true }), { minLength: 1, maxLength: 6 })` to generate hop fee arrays; build a route with those fees as strings; open the detail drawer; assert the "Estimated total fees" value equals the arithmetic sum rounded to 5 decimal places
    - Run 100 iterations
    - Tag: `// Feature: print-friendly-quote-summary, Property 5: RouteDisplay per-hop fee total equals sum of hop fees`
    - _Requirements: 3.3_

  - [ ]* 8.3 Write class-presence and behaviour unit tests for `RouteDisplay`
    - File: `frontend/components/swap/RouteDisplay.test.tsx`
    - Example: alternative routes section wrapper has `print:hidden` class
    - Example: chevron button has `print:hidden` class
    - Example: `[data-testid="extended-diagnostics"]` has `print:hidden` class when `extendedRouteDetails={true}`
    - Example: with `forcePrintExpanded={true}`, the detail drawer renders without clicking the chevron
    - Example: with `forcePrintExpanded={false}` (default), the detail drawer does NOT render until the chevron is clicked
    - _Requirements: 3.4, 3.5, 3.6, 3.7_

- [ ] 9. Add property test for `QuoteSummary`
  - [ ]* 9.1 Write property test for `QuoteSummary` — Property 1
    - **Property 1: QuoteSummary renders all provided financial values**
    - **Validates: Requirements 2.1, 2.2, 2.3**
    - File: `frontend/components/swap/QuoteSummary.test.tsx` (extend existing file)
    - Use `fc.record({ rate: fc.string({ minLength: 1 }), fee: fc.string({ minLength: 1 }), priceImpact: fc.string({ minLength: 1 }) })` and assert all three values appear in the rendered DOM
    - Run 100 iterations
    - Tag: `// Feature: print-friendly-quote-summary, Property 1: QuoteSummary renders all provided financial values`
    - _Requirements: 2.1, 2.2, 2.3_

- [ ] 10. Checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 11. Wire `PrintHeader` and `PrintButton` into `SwapCard` and add `print:hidden` classes
  - Import `PrintHeader` and `PrintButton` at the top of `frontend/components/swap/SwapCard.tsx`
  - Render `<PrintHeader capturedAt={new Date().toISOString()} fromSymbol={fromSymbol} toSymbol={toSymbol} />` as the first child inside `<CardContent>`, before the header row
  - Render `<PrintButton disabled={!fromAmount || parseFloat(fromAmount) === 0} />` inside the info panels conditional block, after `<ShareQuoteButton>`
  - Pass `forcePrintExpanded={true}` to `<RouteDisplay>` (via `RoutePanelAsync`)
  - Add `print:hidden` to the animated background gradient `<div>` elements (the two `absolute` blur divs); add a `card-bg-gradient` class to each so the CSS selector in `globals.css` also targets them
  - Add `print:hidden` to the header controls row `<div>` (compact toggle + `SettingsPanel` + refresh button)
  - Add `print:hidden` to the Pay section outer `<div>` (token selector + amount input)
  - Add `print:hidden` to the toggle `<Button>` (ArrowUpDown)
  - Add `print:hidden` to the Receive section outer `<div>`
  - Add `print:hidden` to the `<SwapButton>` / connect-wallet wrapper `<div>`
  - Add `print:hidden` to the `<ShareQuoteButton>` wrapper `<div>`
  - Add `print:hidden` to the stale indicator `<span data-testid="stale-indicator">` and recovering indicator `<div data-testid="recovering-indicator">`
  - Add `print:hidden` to the `requiresFreshQuote` indicator `<span>`
  - Add `print:hidden` to the "Powered by" footer `<p>`
  - _Requirements: 1.1–1.11, 4.1–4.4, 5.1–5.5_

  - [ ]* 11.1 Write unit tests for `SwapCard` print integration
    - File: `frontend/components/swap/SwapCard.test.tsx` (extend existing file)
    - Example: `PrintButton` is absent when `fromAmount` is `"0"` or empty
    - Example: `PrintButton` is present when `fromAmount` is a positive number string
    - Example: `PrintHeader` is rendered inside the card regardless of `fromAmount`
    - _Requirements: 5.1, 4.1_

- [ ] 12. Create Playwright E2E print preview tests
  - File: `frontend/e2e/print-preview.spec.ts`
  - Use `page.emulateMedia({ media: 'print' })` to activate print styles in Chromium
  - Navigate to the swap page with a pre-filled quote (use URL params or direct state seeding)
  - Assert that `[data-testid="swap-card"]` is visible and has no horizontal overflow (`scrollWidth <= offsetWidth`)
  - Assert that `[data-testid="route-display"]` is visible
  - Assert that the `PriceInfoPanel` outer container is visible
  - Assert that the `FeeBreakdownPanel` outer container is visible
  - Assert that the `PrintHeader` root element is visible (print media active)
  - Assert that the sparkline wrapper is not visible (has `print:hidden`)
  - Assert that the export buttons wrapper is not visible (has `print:hidden`)
  - Repeat the visibility assertions for WebKit if the Playwright project config includes a WebKit project
  - _Requirements: 7.1, 7.2_

- [ ] 13. Final checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation at logical boundaries
- Property tests validate universal correctness properties using fast-check (already in devDependencies at 3.22.0)
- Unit tests validate specific examples and class-presence checks
- The `forcePrintExpanded` prop is passed through `RoutePanelAsync` — check whether that wrapper forwards all props to `RouteDisplay` and add the prop to its interface if needed
