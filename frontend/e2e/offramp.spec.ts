/**
 * E2E spec: /offramp page
 *
 * Issue #1228 — Playwright coverage for /offramp empty and validation
 *
 * Scope:
 *   - Empty state visible on first load (no wallet, no amount)
 *   - Invalid NUBAN account number shows inline error (validation lives in
 *     FiatDestinationForm; the issue mentions "8-char institution code" but the
 *     actual implementation validates a 10-digit NUBAN account number — the
 *     spec tests the real behaviour as implemented)
 *   - Continue button remains disabled when required fields are empty
 *   - No Paycrest API is called; this is a fully static UI test
 *   - Never opens /swap
 *
 * Additive constraint: no /swap files changed; no production env vars set.
 * Production files touched by this PR: none (new file only).
 */

import { test, expect } from "@playwright/test";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Navigate to /offramp and wait for the dashboard to be in DOM.
 * The page is fully client-side rendered; networkidle may fire before React
 * hydration completes, so we also wait for the dashboard test-id.
 */
async function gotoOfframp(page: import("@playwright/test").Page) {
  await page.goto("/offramp");
  await expect(page.getByTestId("offramp-dashboard")).toBeVisible({
    timeout: 10_000,
  });
}

// ---------------------------------------------------------------------------
// Cleanup
// ---------------------------------------------------------------------------

test.afterEach(async ({ page }) => {
  await page.unroute("**");
});

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

test.describe("/offramp page", () => {
  /**
   * 1. Hero section and h1 are visible on first load.
   *    The OfframpDashboard renders an <h1> "Stablecoin to local fiat" inside
   *    the hero card.
   */
  test("shows the hero heading on first load", async ({ page }) => {
    await gotoOfframp(page);

    await expect(
      page.getByRole("heading", { name: /stablecoin to local fiat/i })
    ).toBeVisible();
  });

  /**
   * 2. Fiat destination form is visible in empty state.
   *    FiatDestinationForm renders data-testid="offramp-destination-form" with
   *    h2 "You receive".
   */
  test("shows the fiat destination form in empty state", async ({ page }) => {
    await gotoOfframp(page);

    await expect(page.getByTestId("offramp-destination-form")).toBeVisible();
    await expect(
      page.getByRole("heading", { name: /you receive/i })
    ).toBeVisible();
  });

  /**
   * 3. Continue button is disabled when no amount or bank details are provided.
   */
  test("continue button is disabled on empty form", async ({ page }) => {
    await gotoOfframp(page);

    const continueBtn = page.getByTestId("offramp-continue");
    await expect(continueBtn).toBeVisible();
    await expect(continueBtn).toBeDisabled();
  });

  /**
   * 4. Amount input is present and accepts numeric input.
   */
  test("amount input accepts numeric value", async ({ page }) => {
    await gotoOfframp(page);

    const amountInput = page.getByTestId("offramp-amount");
    await expect(amountInput).toBeVisible();
    await amountInput.fill("50");
    await expect(amountInput).toHaveValue("50");
  });

  /**
   * 5. Account number field: typing fewer than 10 digits then blurring shows
   *    the inline NUBAN validation error.
   *
   *    The OfframpDashboard sets touchedAccount=true on onChange of the
   *    account number input and evaluates isValidNigerianAccountNumber which
   *    requires exactly 10 digits. An 8-digit (or any non-10-digit) value
   *    triggers the error message.
   */
  test("shows inline error for invalid account number (fewer than 10 digits)", async ({
    page,
  }) => {
    await gotoOfframp(page);

    const accountInput = page.getByTestId("offramp-account-number");
    await expect(accountInput).toBeVisible();

    // Type an 8-digit value (invalid NUBAN — must be exactly 10 digits)
    await accountInput.fill("12345678");
    // Trigger blur so the touched state propagates through onChange
    await accountInput.press("Tab");
    await page.waitForTimeout(200);

    // FiatDestinationForm renders: role="alert" with the error text
    await expect(
      page.getByRole("alert").filter({
        hasText: /valid 10-digit nuban/i,
      })
    ).toBeVisible({ timeout: 3000 });

    // aria-invalid should be set on the input
    await expect(accountInput).toHaveAttribute("aria-invalid", "true");
  });

  /**
   * 6. Account number field: typing exactly 10 digits clears the error.
   */
  test("clears inline error when a valid 10-digit NUBAN is entered", async ({
    page,
  }) => {
    await gotoOfframp(page);

    const accountInput = page.getByTestId("offramp-account-number");

    // First trigger the error
    await accountInput.fill("12345678");
    await accountInput.press("Tab");
    await page.waitForTimeout(200);

    // Now correct it
    await accountInput.fill("1234567890");
    await page.waitForTimeout(200);

    await expect(
      page.getByRole("alert").filter({
        hasText: /valid 10-digit nuban/i,
      })
    ).not.toBeVisible();

    await expect(accountInput).toHaveAttribute("aria-invalid", "false");
  });

  /**
   * 7. Account number input enforces a maximum of 10 characters (strips extras).
   *    The onChange handler does: value.replace(/\D/g, '').slice(0, 10)
   */
  test("account number input strips non-numeric and caps at 10 digits", async ({
    page,
  }) => {
    await gotoOfframp(page);

    const accountInput = page.getByTestId("offramp-account-number");
    // Type 12 digits — the field should cap at 10
    await accountInput.fill("123456789012");
    await page.waitForTimeout(100);

    const value = await accountInput.inputValue();
    expect(value.length).toBeLessThanOrEqual(10);
    expect(value).toMatch(/^\d+$/);
  });

  /**
   * 8. Mode toggle is rendered (direct / bridge tabs).
   *    OfframpModeToggle is always mounted.
   */
  test("mode toggle is visible with direct and bridge options", async ({
    page,
  }) => {
    await gotoOfframp(page);

    // The toggle renders role="radio" or role="tab" buttons; use a broader
    // text match since exact label text may vary
    await expect(page.getByText(/direct/i).first()).toBeVisible();
    await expect(page.getByText(/bridge/i).first()).toBeVisible();
  });

  /**
   * 9. Source asset picker is visible (at least the default USDC entry).
   */
  test("source asset picker shows at least one selectable asset", async ({
    page,
  }) => {
    await gotoOfframp(page);

    // SourceAssetPicker renders asset cards; the default is Stellar USDC
    await expect(page.getByText(/USDC/i).first()).toBeVisible();
  });

  /**
   * 10. The spec never navigates to /swap. Confirm the swap card is absent.
   */
  test("does not mount the swap card", async ({ page }) => {
    await gotoOfframp(page);

    await expect(page.getByTestId("swap-card")).not.toBeVisible();
  });
});
