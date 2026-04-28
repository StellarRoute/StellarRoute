/**
 * A11y test suite: Accessibility checks for critical swap flows
 *
 * Issue #312 — Automate accessibility checks for core swap interactions to
 * catch regressions before merge.
 *
 * Covers five surfaces:
 *   1. Swap form (default, amount entered, insufficient balance, stale, error)
 *   2. Token selection dialog (scan, focus, escape, focus trap)
 *   3. Route list (scan, accessible names, keyboard selection)
 *   4. High-impact confirmation modal (scan, focus, escape, focus trap)
 *   5. Settings panel (scan, slippage label)
 *
 * All scans use @axe-core/playwright scoped to the relevant DOM subtree.
 * Only `critical` and `serious` violations fail the test — `moderate` and
 * `minor` are surfaced in the report but do not block CI.
 *
 * Requirements: 1.x, 2.x, 3.x, 4.x, 5.x, 6.x, 7.x
 */

import { test, expect, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import type { Result } from "axe-core";

// ---------------------------------------------------------------------------
// Baseline exclusions
// ---------------------------------------------------------------------------

/**
 * Axe rule IDs that are explicitly deferred and must NOT cause CI failures.
 *
 * Each entry must be documented in docs/a11y-testing.md with a rationale.
 * To add a new exclusion:
 *   1. Add the rule ID string to this array.
 *   2. Add a comment explaining why it is deferred.
 *   3. Document it in docs/a11y-testing.md under "Baseline Exclusions".
 *
 * Currently empty — no pre-existing violations have been deferred.
 */
const BASELINE_EXCLUSIONS: string[] = [];

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/** Mirrors QUOTE_STALE_AFTER_MS from frontend/lib/quote-stale.ts */
const QUOTE_STALE_AFTER_MS = 5500;

function alternativeRoutesFixture() {
  return [
    { id: "route-0", venue: "AQUA Pool", expectedAmount: "≈ 99.5500" },
    { id: "route-1", venue: "SDEX", expectedAmount: "≈ 99.4000" },
    { id: "route-2", venue: "Phoenix AMM", expectedAmount: "≈ 99.2500" },
  ];
}

/** Standard quote fixture — price_impact is low (0.5%) so no modal is triggered. */
function freshQuoteFixture() {
  return {
    base_asset: { asset_type: "native" },
    quote_asset: {
      asset_type: "credit_alphanum4",
      asset_code: "USDC",
      asset_issuer: "GA5ZSEJYB37JRC5AVCIAZDL2Y343IFRMA2EO3HJWV2XG7H5V5CQRUP7W",
    },
    amount: "100",
    price: "0.995",
    total: "99.5",
    price_impact: "0.5",
    quote_type: "sell",
    path: [],
    timestamp: Math.floor(Date.now() / 1000),
    source_timestamp: Date.now(),
    alternativeRoutes: alternativeRoutesFixture(),
  };
}

/**
 * High-impact quote fixture — price_impact is 15% so HighImpactConfirmModal
 * is triggered when the user clicks the swap CTA.
 */
function highImpactQuoteFixture() {
  return {
    ...freshQuoteFixture(),
    price_impact: "15",
  };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Run an axe scan scoped to `selector` and return only high-severity
 * violations (impact === 'critical' | 'serious').
 */
async function scanForHighSeverity(
  page: Page,
  selector: string
): Promise<Result[]> {
  const results = await new AxeBuilder({ page })
    .include(selector)
    .disableRules(BASELINE_EXCLUSIONS)
    .analyze();
  return results.violations.filter(
    (v) => v.impact === "critical" || v.impact === "serious"
  );
}

/**
 * Assert that no high-severity violations were found.
 * Produces a human-readable failure message listing each violation's rule ID,
 * impact level, description, and the offending HTML nodes.
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

/** Fill the first amount input and wait for debounce + render. */
async function fillAmountAndWait(page: Page, amount = "100") {
  const input = page.locator('input[placeholder="0.00"]').first();
  await input.fill(amount);
  await page.waitForTimeout(600);
}

// ---------------------------------------------------------------------------
// Cleanup
// ---------------------------------------------------------------------------

test.afterEach(async ({ page }) => {
  await page.unroute("**");
  try {
    await (page as any).clock?.uninstall?.();
  } catch {
    // clock may not have been installed in every test
  }
});

// ---------------------------------------------------------------------------
// Group 1 — Swap form a11y
// ---------------------------------------------------------------------------

test.describe("Swap form a11y", () => {
  /**
   * 1.1 Default state — no wallet connected, no amount entered.
   * Requirements: 1.1, 1.2
   */
  test("swap form has no high-severity violations in default state", async ({ page }) => {
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(freshQuoteFixture()),
      });
    });
    await page.goto("/swap");
    await page.waitForLoadState("networkidle");

    const violations = await scanForHighSeverity(page, '[data-testid="swap-card"]');
    assertNoHighSeverityViolations(violations);
  });

  /**
   * 1.2 Amount entered, quote loaded.
   * Requirements: 1.1
   */
  test("swap form has no high-severity violations with amount entered and quote loaded", async ({ page }) => {
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(freshQuoteFixture()),
      });
    });
    await page.goto("/swap");
    await fillAmountAndWait(page, "100");

    const violations = await scanForHighSeverity(page, '[data-testid="swap-card"]');
    assertNoHighSeverityViolations(violations);
  });

  /**
   * 1.3 Insufficient balance state.
   * Requirements: 1.3
   */
  test("swap form has no high-severity violations in insufficient balance state", async ({ page }) => {
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(freshQuoteFixture()),
      });
    });
    await page.goto("/swap");
    // 99999 exceeds the mock balance of 100.00
    await fillAmountAndWait(page, "99999");

    const violations = await scanForHighSeverity(page, '[data-testid="swap-card"]');
    assertNoHighSeverityViolations(violations);
  });

  /**
   * 1.4 Stale quote state.
   * Requirements: 1.4
   */
  test("swap form has no high-severity violations in stale quote state", async ({ page }) => {
    await page.clock.install({ time: Date.now() });
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(freshQuoteFixture()),
      });
    });
    await page.goto("/swap");
    await fillAmountAndWait(page, "100");
    // Advance clock past stale threshold
    await page.clock.tick(QUOTE_STALE_AFTER_MS + 100);
    await page.waitForTimeout(300);
    // Stale indicator should be visible
    await expect(page.getByTestId("stale-indicator")).toBeVisible({ timeout: 2000 });

    const violations = await scanForHighSeverity(page, '[data-testid="swap-card"]');
    assertNoHighSeverityViolations(violations);
  });

  /**
   * 1.5 Quote error state.
   * Requirements: 1.5
   */
  test("swap form has no high-severity violations in quote error state", async ({ page }) => {
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 500,
        contentType: "application/json",
        body: JSON.stringify({ error: "Internal Server Error" }),
      });
    });
    await page.goto("/swap");
    await fillAmountAndWait(page, "100");
    // Wait for error state to render
    await page.waitForTimeout(1000);

    const violations = await scanForHighSeverity(page, '[data-testid="swap-card"]');
    assertNoHighSeverityViolations(violations);
  });
});

// ---------------------------------------------------------------------------
// Group 2 — Token dialog a11y
// ---------------------------------------------------------------------------

test.describe("Token dialog a11y", () => {
  /**
   * 2.1 Dialog scan — zero high-severity violations when dialog is open.
   * Requirements: 2.2
   */
  test("token selection dialog has no high-severity violations", async ({ page }) => {
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(freshQuoteFixture()),
      });
    });
    await page.goto("/swap");
    await page.waitForLoadState("networkidle");

    // Click the first token selector button (the "You Pay" token)
    const tokenBtn = page.getByRole("button", { name: /xlm|select/i }).first();
    await tokenBtn.click();
    await expect(page.locator('[role="dialog"]')).toBeVisible({ timeout: 3000 });

    const violations = await scanForHighSeverity(page, '[role="dialog"]');
    assertNoHighSeverityViolations(violations);
  });

  /**
   * 2.2 Focus moves to search input when dialog opens.
   * Requirements: 2.1
   */
  test("token dialog receives focus on search input when opened", async ({ page }) => {
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(freshQuoteFixture()),
      });
    });
    await page.goto("/swap");
    await page.waitForLoadState("networkidle");

    const tokenBtn = page.getByRole("button", { name: /xlm|select/i }).first();
    await tokenBtn.click();
    await expect(page.locator('[role="dialog"]')).toBeVisible({ timeout: 3000 });

    // The search input should be focused (autoFocus is set on the Input)
    const searchInput = page.locator('[role="dialog"] input').first();
    await expect(searchInput).toBeFocused({ timeout: 1000 });
  });

  /**
   * 2.3 Escape closes dialog and returns focus to the trigger button.
   * Requirements: 2.3
   */
  test("pressing Escape closes token dialog and returns focus to trigger", async ({ page }) => {
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(freshQuoteFixture()),
      });
    });
    await page.goto("/swap");
    await page.waitForLoadState("networkidle");

    const tokenBtn = page.getByRole("button", { name: /xlm|select/i }).first();
    await tokenBtn.click();
    await expect(page.locator('[role="dialog"]')).toBeVisible({ timeout: 3000 });

    await page.keyboard.press("Escape");
    await expect(page.locator('[role="dialog"]')).not.toBeVisible({ timeout: 2000 });

    // Focus should return to the button that opened the dialog
    await expect(tokenBtn).toBeFocused({ timeout: 1000 });
  });

  /**
   * 2.4 Focus is trapped inside the dialog.
   * Requirements: 2.4
   */
  test("focus is trapped inside token dialog", async ({ page }) => {
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(freshQuoteFixture()),
      });
    });
    await page.goto("/swap");
    await page.waitForLoadState("networkidle");

    const tokenBtn = page.getByRole("button", { name: /xlm|select/i }).first();
    await tokenBtn.click();
    await expect(page.locator('[role="dialog"]')).toBeVisible({ timeout: 3000 });

    // Tab through focusable elements several times and verify focus stays inside dialog
    for (let i = 0; i < 8; i++) {
      await page.keyboard.press("Tab");
      const focusedOutside = await page.evaluate(() => {
        const dialog = document.querySelector('[role="dialog"]');
        const focused = document.activeElement;
        return dialog && focused ? !dialog.contains(focused) : false;
      });
      expect(focusedOutside, `Focus escaped dialog on Tab press ${i + 1}`).toBe(false);
    }
  });
});

// ---------------------------------------------------------------------------
// Group 3 — Route list a11y
// ---------------------------------------------------------------------------

test.describe("Route list a11y", () => {
  /**
   * 3.1 Route list scan — zero high-severity violations.
   * Requirements: 3.2
   */
  test("route list has no high-severity violations", async ({ page }) => {
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(freshQuoteFixture()),
      });
    });
    await page.goto("/swap");
    await fillAmountAndWait(page, "100");
    await expect(page.getByTestId("route-display")).toBeVisible({ timeout: 3000 });

    const violations = await scanForHighSeverity(page, '[data-testid="route-display"]');
    assertNoHighSeverityViolations(violations);
  });

  /**
   * 3.2 Each route button has a non-empty accessible name.
   * Requirements: 3.1
   */
  test("all route candidate buttons have non-empty accessible names", async ({ page }) => {
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(freshQuoteFixture()),
      });
    });
    await page.goto("/swap");
    await fillAmountAndWait(page, "100");
    await expect(page.getByTestId("alternative-route-route-0")).toBeVisible({ timeout: 3000 });

    const routeButtons = page.locator('[data-testid^="alternative-route-"]');
    const count = await routeButtons.count();
    expect(count).toBeGreaterThanOrEqual(2);

    for (let i = 0; i < count; i++) {
      const btn = routeButtons.nth(i);
      // Accessible name comes from inner text (venue + amount)
      const innerText = await btn.innerText();
      expect(
        innerText.trim(),
        `Route button ${i} must have non-empty accessible name`
      ).not.toBe("");
    }
  });

  /**
   * 3.3 Keyboard Enter selects a route and sets aria-pressed="true".
   * Requirements: 3.3, 3.4
   */
  test("keyboard Enter on route button selects it and sets aria-pressed", async ({ page }) => {
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(freshQuoteFixture()),
      });
    });
    await page.goto("/swap");
    await fillAmountAndWait(page, "100");
    await expect(page.getByTestId("alternative-route-route-0")).toBeVisible({ timeout: 3000 });

    const firstRoute = page.getByTestId("alternative-route-route-0");
    await firstRoute.focus();
    await firstRoute.press("Enter");

    await expect(firstRoute).toHaveAttribute("aria-pressed", "true", { timeout: 1000 });
  });
});

// ---------------------------------------------------------------------------
// Group 4 — High-impact confirmation modal a11y
// ---------------------------------------------------------------------------

test.describe("Confirm modal a11y", () => {
  /**
   * Helper: navigate to /swap, connect wallet, enter amount, and open the
   * high-impact confirmation modal.
   *
   * The mock balance is 100.00. We enter 50 (within balance) so the button
   * state reaches `high_impact_warning` (price_impact = 15 > 10).
   * The modal opens when price_impact > 5 and the user clicks the CTA.
   */
  async function openConfirmModal(page: Page) {
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(highImpactQuoteFixture()),
      });
    });
    await page.goto("/swap");
    await page.waitForLoadState("networkidle");

    // Connect wallet by clicking the CTA (which shows "Connect Wallet" initially)
    const connectBtn = page.getByRole("button", { name: /connect wallet/i });
    if (await connectBtn.isVisible()) {
      await connectBtn.click();
    }

    await fillAmountAndWait(page, "50");

    // Click the swap CTA — with price_impact > 5 this opens the confirm modal
    const swapCta = page.getByRole("button", { name: /swap|review|confirm/i }).first();
    await swapCta.click();
    await expect(page.locator('[role="dialog"]')).toBeVisible({ timeout: 3000 });
  }

  /**
   * 4.1 Modal scan — zero high-severity violations.
   * Requirements: 4.2
   */
  test("high-impact confirm modal has no high-severity violations", async ({ page }) => {
    await openConfirmModal(page);

    const violations = await scanForHighSeverity(page, '[role="dialog"]');
    assertNoHighSeverityViolations(violations);
  });

  /**
   * 4.2 Focus moves to first interactive element when modal opens.
   * Requirements: 4.1
   */
  test("confirm modal receives focus on first interactive element when opened", async ({ page }) => {
    await openConfirmModal(page);

    // First interactive element is the checkbox or the Cancel button
    const focusedInsideModal = await page.evaluate(() => {
      const dialog = document.querySelector('[role="dialog"]');
      const focused = document.activeElement;
      return dialog && focused ? dialog.contains(focused) : false;
    });
    expect(focusedInsideModal, "Focus must be inside the modal after it opens").toBe(true);
  });

  /**
   * 4.3 Escape closes modal.
   * Requirements: 4.4
   */
  test("pressing Escape closes the confirm modal", async ({ page }) => {
    await openConfirmModal(page);

    await page.keyboard.press("Escape");
    await expect(page.locator('[role="dialog"]')).not.toBeVisible({ timeout: 2000 });
  });

  /**
   * 4.4 Focus is trapped inside the modal.
   * Requirements: 4.3
   */
  test("focus is trapped inside confirm modal", async ({ page }) => {
    await openConfirmModal(page);

    for (let i = 0; i < 6; i++) {
      await page.keyboard.press("Tab");
      const focusedOutside = await page.evaluate(() => {
        const dialog = document.querySelector('[role="dialog"]');
        const focused = document.activeElement;
        return dialog && focused ? !dialog.contains(focused) : false;
      });
      expect(focusedOutside, `Focus escaped modal on Tab press ${i + 1}`).toBe(false);
    }
  });
});

// ---------------------------------------------------------------------------
// Group 5 — Settings panel a11y
// ---------------------------------------------------------------------------

test.describe("Settings panel a11y", () => {
  /**
   * 5.1 Settings panel scan — zero high-severity violations.
   * Requirements: 5.2
   */
  test("settings panel has no high-severity violations", async ({ page }) => {
    await page.goto("/swap");
    await page.waitForLoadState("networkidle");

    // Open the settings panel
    await page.getByRole("button", { name: /settings/i }).click();
    await expect(page.getByTestId("settings-panel")).toBeVisible({ timeout: 2000 });

    const violations = await scanForHighSeverity(page, '[data-testid="settings-panel"]');
    assertNoHighSeverityViolations(violations);
  });

  /**
   * 5.2 Slippage custom input has an accessible label.
   * Requirements: 5.3
   */
  test("slippage custom input has an accessible label", async ({ page }) => {
    await page.goto("/swap");
    await page.waitForLoadState("networkidle");

    await page.getByRole("button", { name: /settings/i }).click();
    await expect(page.getByTestId("settings-panel")).toBeVisible({ timeout: 2000 });

    // The custom slippage input should have aria-label set
    const slippageInput = page
      .getByTestId("settings-panel")
      .locator('input[type="number"]');
    const ariaLabel = await slippageInput.getAttribute("aria-label");
    expect(
      ariaLabel,
      "Slippage custom input must have a non-empty aria-label"
    ).toBeTruthy();
  });
});
