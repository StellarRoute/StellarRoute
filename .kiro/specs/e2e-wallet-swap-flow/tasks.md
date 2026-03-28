# Implementation Plan: E2E Wallet Swap Flow

## Overview

Add a Playwright-based E2E test suite to the StellarRoute frontend covering the full wallet-connect → swap-confirm user journey. Tests run deterministically via network mocking and `Math.random` patching, with CI integration and artifact capture on failure.

## Tasks

- [ ] 1. Set up Playwright framework and project structure
  - Install `@playwright/test` as a dev dependency in `frontend/package.json`
  - Add `"e2e": "playwright test"` and `"e2e:ui": "playwright test --ui"` scripts to `frontend/package.json`
  - Create `frontend/playwright.config.ts` with `baseURL`, `testDir: './e2e'`, `outputDir: './playwright-report'`, `screenshot: 'only-on-failure'`, `trace: 'retain-on-failure'`, `workers: process.env.CI ? 1 : undefined`, single `chromium` project, and `webServer` pointing to `npm run start` on port 3000
  - Create the `frontend/e2e/` directory tree: `fixtures/`, `helpers/`, `pages/`, `specs/`
  - _Requirements: 1.1, 1.2, 1.3, 1.4_

- [ ] 2. Create fixture files and mock route helpers
  - [ ] 2.1 Create `frontend/e2e/fixtures/pairs.json` with the XLM/USDC pair fixture matching the `PairsResponse` interface
  - [ ] 2.2 Create `frontend/e2e/fixtures/quote.json` with deterministic `total: "10.500000"`, `price: "0.105000"`, and `path` fields matching the `PriceQuote` interface
  - [ ] 2.3 Implement `frontend/e2e/helpers/mockRoutes.ts` with `applyDefaultMocks(page)` that intercepts `**/api/v1/pairs` and `**/api/v1/quote/**` with the fixture files, plus a catch-all handler that aborts unmatched requests and fails the test with the unmocked URL
  - [ ] 2.4 Implement `frontend/e2e/helpers/deterministicRandom.ts` with `patchMathRandom(page, value)` that calls `page.addInitScript` to replace `Math.random` with a constant-returning function
  - [ ]* 2.5 Write property test for pairs mock intercept
    - **Property 1: Pairs mock intercept**
    - **Validates: Requirements 2.1, 2.3**
  - [ ]* 2.6 Write property test for quote mock intercept
    - **Property 2: Quote mock intercept**
    - **Validates: Requirements 2.2, 2.3**
  - _Requirements: 2.1, 2.2, 2.3, 2.5_

- [ ] 3. Implement SwapPage page object
  - Create `frontend/e2e/pages/SwapPage.ts` with all locators (`connectButton`, `walletAddress`, `maxButton`, `pairSelect`, `sellAmountInput`, `validationError`, `estimatedReceive`, `refreshQuoteButton`, `reviewSwapButton`, modal locators) and action methods (`connectWallet()`, `selectPair()`, `enterSellAmount()`, `clickReviewSwap()`, `confirmSwap()`)
  - _Requirements: 3.1, 3.2, 4.1, 5.1, 6.1, 7.1_

- [ ] 4. Write wallet connection spec
  - [ ] 4.1 Create `frontend/e2e/specs/wallet-connect.spec.ts` with `beforeEach` calling `applyDefaultMocks`
  - [ ] 4.2 Implement test asserting wallet button visible and address absent on fresh load
  - [ ] 4.3 Implement test asserting `isConnected: true`, mock address visible, and "Max" button enabled after `connectWallet()`
  - [ ] 4.4 Implement test asserting timeout failure if connect does not complete within 5 seconds (use `toBeVisible({ timeout: 5000 })`)
  - [ ]* 4.5 Write property test for wallet connection flow
    - **Property 3: Wallet connection flow**
    - **Validates: Requirements 3.1, 3.2, 3.3**
  - _Requirements: 3.1, 3.2, 3.3, 3.4_

- [ ] 5. Write pair selection and sell amount spec
  - [ ] 5.1 Create `frontend/e2e/specs/pair-amount.spec.ts` with `beforeEach` calling `applyDefaultMocks` and `connectWallet()`
  - [ ] 5.2 Implement test asserting pair selector is populated with fixture pairs on load
  - [ ] 5.3 Implement test asserting sell-amount input label updates to base asset after pair selection
  - [ ] 5.4 Implement test asserting no validation error and "Review Swap" enabled for valid amount `"100"`
  - [ ] 5.5 Implement test asserting validation error displayed and "Review Swap" disabled for invalid input `"abc"`
  - [ ] 5.6 Implement test asserting precision-exceeded error for amount with more than 7 decimal places
  - [ ]* 5.7 Write property test for valid input enables Review Swap
    - **Property 4: Valid input enables Review Swap**
    - **Validates: Requirements 4.1, 4.2, 4.3, 5.4**
  - [ ]* 5.8 Write property test for invalid input disables Review Swap
    - **Property 5: Invalid input disables Review Swap**
    - **Validates: Requirements 4.4, 4.5**
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

- [ ] 6. Write quote fetch and display spec
  - [ ] 6.1 Create `frontend/e2e/specs/quote.spec.ts` with `beforeEach` calling `applyDefaultMocks` and `connectWallet()`
  - [ ] 6.2 Implement test asserting estimated receive amount matches fixture `total` after debounce (use `page.waitForResponse('**/api/v1/quote/**')`)
  - [ ] 6.3 Implement test asserting "Refresh quote" click records a new `QuoteAPI` request and updates displayed quote
  - [ ] 6.4 Implement test asserting "Refresh quote" button is disabled and no additional request is made when clicked within cooldown
  - [ ]* 6.5 Write property test for quote display matches fixture
    - **Property 6: Quote display matches fixture**
    - **Validates: Requirements 5.1**
  - [ ]* 6.6 Write property test for refresh quote fires a new network request
    - **Property 7: Refresh quote fires a new network request**
    - **Validates: Requirements 5.2, 5.3**
  - _Requirements: 5.1, 5.2, 5.3, 5.4_

- [ ] 7. Checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 8. Write review modal spec
  - [ ] 8.1 Create `frontend/e2e/specs/review-modal.spec.ts` with `beforeEach` calling `applyDefaultMocks`, `connectWallet()`, entering sell amount `"100"`, and waiting for quote
  - [ ] 8.2 Implement test asserting modal opens in `review` state (Confirm Swap + Cancel buttons visible) after clicking "Review Swap"
  - [ ] 8.3 Implement test asserting "You Pay" shows entered amount and base asset, "You Receive" shows fixture `total` and counter asset
  - [ ] 8.4 Implement test asserting exchange rate, network fee, and route path are visible in `review` state
  - [ ] 8.5 Implement test asserting modal closes and form values are retained after clicking "Cancel"
  - [ ]* 8.6 Write property test for Review Swap opens modal in review state
    - **Property 8: Review Swap opens modal in review state**
    - **Validates: Requirements 6.1**
  - [ ]* 8.7 Write property test for modal review state displays correct trade details
    - **Property 9: Modal review state displays correct trade details**
    - **Validates: Requirements 6.2, 6.3, 6.4**
  - [ ]* 8.8 Write property test for cancel closes modal without clearing form
    - **Property 10: Cancel closes modal without clearing form**
    - **Validates: Requirements 6.5**
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

- [ ] 9. Write swap success spec
  - [ ] 9.1 Create `frontend/e2e/specs/swap-success.spec.ts`; call `patchMathRandom(page, 0.9)` in `beforeEach` before navigation, then `applyDefaultMocks`, `connectWallet()`, enter amount, wait for quote, open modal
  - [ ] 9.2 Implement test asserting modal transitions `pending` → `submitting` → `processing` → `success`, each within 5 seconds, and success state shows received amount
  - [ ] 9.3 Implement test asserting modal closes after clicking "Done" in `success` state
  - [ ]* 9.4 Write property test for happy-path state machine
    - **Property 11: Happy-path state machine**
    - **Validates: Requirements 7.1, 7.2, 7.3, 7.4, 7.5**
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

- [ ] 10. Write swap failure spec
  - [ ] 10.1 Create `frontend/e2e/specs/swap-failure.spec.ts`; call `patchMathRandom(page, 0.1)` in `beforeEach` before navigation, then `applyDefaultMocks`, `connectWallet()`, enter amount, wait for quote, open modal
  - [ ] 10.2 Implement test asserting modal reaches `failed` state after "Confirm Swap" is clicked
  - [ ] 10.3 Implement test asserting error message is displayed in `failed` state
  - [ ] 10.4 Implement test asserting modal closes after clicking "Dismiss"
  - [ ]* 10.5 Write property test for failure-path state machine
    - **Property 12: Failure-path state machine**
    - **Validates: Requirements 8.1, 8.2, 8.3**
  - _Requirements: 8.1, 8.2, 8.3_

- [ ] 11. Checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 12. Add E2E README and CI job
  - [ ] 12.1 Create `frontend/e2e/README.md` documenting which endpoints are mocked, the fixture format, and how to override fixtures per test
  - [ ] 12.2 Add the `e2e` job to `.github/workflows/ci.yml` after the `frontend` job: install deps, `npx playwright install --with-deps chromium`, `npm run build`, `npm run e2e` with `CI: true`, and upload `frontend/playwright-report/` as an artifact on failure with 7-day retention
  - _Requirements: 2.4, 9.1, 9.2, 9.3, 9.4, 9.5_

## Notes

- Tasks marked with `*` are optional and can be skipped for a faster MVP
- `patchMathRandom` must be called before `page.goto()` so `addInitScript` takes effect on the initial page load
- Use `page.waitForResponse('**/api/v1/quote/**')` instead of `page.waitForTimeout` to avoid brittle fixed delays in CI
- Property tests in this suite are Playwright E2E tests (not fast-check) because they involve browser state and DOM assertions
