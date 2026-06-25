import { expect, test } from "@playwright/test";

test.describe("swap route consolidation", () => {
  test.describe.configure({ mode: "serial" });

  test("root redirects to the production swap experience", async ({ page }) => {
    await page.goto("/");

    await expect(page).toHaveURL(/\/swap$/);
    await expect(page.getByTestId("swap-card")).toBeVisible();
    await expect(
      page.getByText(/demo swap with sell amount validation/i),
    ).toHaveCount(0);
  });

  test("header Swap navigation opens the production swap card", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto("/orderbook");

    const swapNavLink = page
      .locator("header")
      .getByRole("navigation", { name: "Main navigation" })
      .getByRole("link", { name: "Swap", exact: true });
    await expect(swapNavLink).toBeVisible();
    await swapNavLink.click();

    await expect(page).toHaveURL(/\/swap$/);
    await expect(page.getByTestId("swap-card")).toBeVisible();
    await expect(swapNavLink).toHaveAttribute("aria-current", "page");
  });
});
