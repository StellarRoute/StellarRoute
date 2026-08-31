/**
 * A11y test suite: Accessibility checks for /orderbook and /history
 *
 * Issue #1230 — Playwright a11y scan for /orderbook and /history
 *
 * Covers:
 *   1. /orderbook — axe scan on empty and populated states
 *   2. /history   — axe scan on empty and populated states
 *   3. /guide     — optional axe scan (page exists in app/guide)
 *
 * All scans use @axe-core/playwright scoped to the page <main> (or <body>
 * when <main> is absent). Only `critical` and `serious` violations fail the
 * test; `moderate` / `minor` are surfaced in the report but do not block CI.
 *
 * Baseline exclusions mirror the swap a11y spec (color-contrast deferred;
 * see docs/a11y-testing.md).
 *
 * Additive constraint: a11y-swap-flow.spec.ts is NOT modified.
 * No /swap files are changed. No production env vars are set.
 * Production files touched by this PR: none (new file only).
 */

import { test, expect, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import type { Result } from "axe-core";

// ---------------------------------------------------------------------------
// Baseline exclusions (must match rationale in docs/a11y-testing.md)
// ---------------------------------------------------------------------------

/**
 * color-contrast: indigo primary palette fails WCAG AA against dark-mode
 * backgrounds. Deferred as a design-token change; must not block unrelated CI.
 */
const BASELINE_EXCLUSIONS: string[] = ["color-contrast"];

// ---------------------------------------------------------------------------
// Fixtures — mirrors the patterns in orderbook.spec.ts and history.spec.ts
// ---------------------------------------------------------------------------

const MOCK_PAIRS = [
  {
    base: "XLM",
    counter: "USDC",
    base_asset: "native",
    counter_asset:
      "USDC:GA5ZSEJYB37JRC5AVCIAZDL2Y343IFRMA2EO3HJWV2XG7H5V5CQRUP7W",
  },
];

function pairsResponseFixture() {
  return { pairs: MOCK_PAIRS, total: MOCK_PAIRS.length };
}

function emptyOrderbookFixture() {
  return {
    bids: [],
    asks: [],
    base: "native",
    counter: "USDC:GA5ZSEJYB37JRC5AVCIAZDL2Y343IFRMA2EO3HJWV2XG7H5V5CQRUP7W",
  };
}

function populatedOrderbookFixture() {
  return {
    bids: [
      { price: "0.1000", amount: "500.0000000", total: "50.0000000" },
    ],
    asks: [
      { price: "0.1001", amount: "300.0000000", total: "30.0300000" },
    ],
    base: "native",
    counter: "USDC:GA5ZSEJYB37JRC5AVCIAZDL2Y343IFRMA2EO3HJWV2XG7H5V5CQRUP7W",
  };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Run an axe scan on the page (scoped to <main> when present, falling back to
 * <body>) and return only high-severity violations.
 */
async function scanForHighSeverity(page: Page): Promise<Result[]> {
  // Determine scan root: prefer <main> for narrower scope
  const hasMain = await page.locator("main").count();
  const selector = hasMain > 0 ? "main" : "body";

  const results = await new AxeBuilder({ page })
    .include(selector)
    .disableRules(BASELINE_EXCLUSIONS)
    .analyze();

  return results.violations.filter(
    (v) => v.impact === "critical" || v.impact === "serious"
  );
}

/**
 * Assert no high-severity violations, with a human-readable failure report.
 */
function assertNoHighSeverityViolations(violations: Result[]): void {
  const report = violations
    .map(
      (v) =>
        `[${v.impact?.toUpperCase()}] ${v.id}: ${v.description}\n` +
        `  Help: ${v.helpUrl}\n` +
        `  Nodes:\n` +
        v.nodes.map((n) => `    ${n.html}`).join("\n")
    )
    .join("\n\n");

  expect(
    violations,
    violations.length > 0
      ? `High-severity a11y violations found:\n\n${report}`
      : ""
  ).toHaveLength(0);
}

async function mockOrderbookEndpoints(page: Page, orderbookBody: object) {
  await page.route("**/api/v1/pairs**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(pairsResponseFixture()),
    });
  });

  await page.route("**/api/v1/orderbook/**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(orderbookBody),
    });
  });
}

// ---------------------------------------------------------------------------
// Cleanup
// ---------------------------------------------------------------------------

test.afterEach(async ({ page }) => {
  await page.unroute("**");
});

// ---------------------------------------------------------------------------
// Group 1 — /orderbook a11y
// ---------------------------------------------------------------------------

test.describe("/orderbook a11y", () => {
  /**
   * 1.1 Empty orderbook state — zero high-severity violations.
   */
  test("/orderbook (empty state) has no high-severity a11y violations", async ({
    page,
  }) => {
    await mockOrderbookEndpoints(page, emptyOrderbookFixture());

    await page.goto("/orderbook");
    await page.waitForLoadState("networkidle");
    // Allow React to settle after pair auto-selection
    await page.waitForTimeout(500);

    const violations = await scanForHighSeverity(page);
    assertNoHighSeverityViolations(violations);
  });

  /**
   * 1.2 Populated orderbook state — zero high-severity violations.
   */
  test("/orderbook (populated state) has no high-severity a11y violations", async ({
    page,
  }) => {
    await mockOrderbookEndpoints(page, populatedOrderbookFixture());

    await page.goto("/orderbook");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(500);

    const violations = await scanForHighSeverity(page);
    assertNoHighSeverityViolations(violations);
  });

  /**
   * 1.3 Bid and ask row cells have non-empty accessible names (aria-label).
   *    VirtualizedOrderSide sets aria-label on each [role="row"] via i18n.
   */
  test("bid and ask rows have non-empty aria-label attributes", async ({
    page,
  }) => {
    await mockOrderbookEndpoints(page, populatedOrderbookFixture());

    await page.goto("/orderbook");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(500);

    const rows = page.locator('[role="row"]');
    const count = await rows.count();

    // If rows are present, each must have a non-empty aria-label
    for (let i = 0; i < count; i++) {
      const label = await rows.nth(i).getAttribute("aria-label");
      expect(
        label?.trim(),
        `Row ${i} must have a non-empty aria-label`
      ).toBeTruthy();
    }
  });

  /**
   * 1.4 Pool progressbar on /orderbook is absent (this page has no pool stats).
   *    Guard: confirms the scan is on /orderbook and not /analytics bleed.
   */
  test("does not contain analytics progressbar elements", async ({ page }) => {
    await mockOrderbookEndpoints(page, emptyOrderbookFixture());

    await page.goto("/orderbook");
    await page.waitForLoadState("networkidle");

    // role="progressbar" with pool utilisation aria-label only appears on /analytics
    await expect(
      page.getByRole("progressbar", { name: /pool utilisation/i })
    ).not.toBeAttached();
  });
});

// ---------------------------------------------------------------------------
// Group 2 — /history a11y
// ---------------------------------------------------------------------------

test.describe("/history a11y", () => {
  /**
   * 2.1 Empty history state — zero high-severity violations.
   */
  test("/history (empty state) has no high-severity a11y violations", async ({
    page,
  }) => {
    await page.addInitScript(() => {
      // Seed empty history so the page skips the loading skeleton fast
      localStorage.setItem(
        "stellar_route_tx_history_GBSU...XYZ9",
        JSON.stringify([])
      );
    });

    await page.goto("/history");
    // Wait for loading skeleton to disappear
    await expect(
      page.locator('[aria-label="Loading transaction history"]')
    ).not.toBeVisible({ timeout: 8000 });

    const violations = await scanForHighSeverity(page);
    assertNoHighSeverityViolations(violations);
  });

  /**
   * 2.2 Populated history state — zero high-severity violations.
   */
  test("/history (populated state) has no high-severity a11y violations", async ({
    page,
  }) => {
    await page.addInitScript(() => {
      const mockTxs = [
        {
          id: "tx-a11y-1",
          hash: "0xaaaa1234567890abcdef1234567890abcdef1234567890abcdef1234567890aa",
          timestamp: Date.now() - 120_000,
          fromAsset: "XLM",
          fromAmount: "20.00",
          toAsset: "USDC",
          toAmount: "2.45",
          exchangeRate: "0.1225",
          status: "confirmed",
        },
        {
          id: "tx-a11y-2",
          hash: "0xbbbb1234567890abcdef1234567890abcdef1234567890abcdef1234567890bb",
          timestamp: Date.now() - 3_600_000,
          fromAsset: "USDC",
          fromAmount: "10.00",
          toAsset: "XLM",
          toAmount: "81.50",
          exchangeRate: "8.15",
          status: "failed",
          errorMessage: "Slippage tolerance exceeded",
        },
      ];
      localStorage.setItem(
        "stellar_route_tx_history_GBSU...XYZ9",
        JSON.stringify(mockTxs)
      );
    });

    await page.goto("/history");
    await expect(
      page.locator('[aria-label="Loading transaction history"]')
    ).not.toBeVisible({ timeout: 8000 });

    const violations = await scanForHighSeverity(page);
    assertNoHighSeverityViolations(violations);
  });

  /**
   * 2.3 "Transaction History" heading is present and scannable.
   */
  test("/history heading is visible", async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem(
        "stellar_route_tx_history_GBSU...XYZ9",
        JSON.stringify([])
      );
    });

    await page.goto("/history");
    await expect(
      page.locator('[aria-label="Loading transaction history"]')
    ).not.toBeVisible({ timeout: 8000 });

    await expect(
      page.getByRole("heading", { name: /transaction history/i })
    ).toBeVisible();
  });
});

// ---------------------------------------------------------------------------
// Group 3 — /guide a11y (optional)
// ---------------------------------------------------------------------------

test.describe("/guide a11y", () => {
  /**
   * 3.1 /guide page — zero high-severity violations.
   *    The guide page is static; no mocking is required.
   */
  test("/guide has no high-severity a11y violations", async ({ page }) => {
    await page.goto("/guide");
    await page.waitForLoadState("networkidle");

    const violations = await scanForHighSeverity(page);
    assertNoHighSeverityViolations(violations);
  });
});
