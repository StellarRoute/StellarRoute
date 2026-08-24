import { expect, test, type Page } from "@playwright/test";

const BREAKPOINTS = [
  { name: "mobile", width: 375, height: 812 },
  { name: "tablet", width: 768, height: 1024 },
  { name: "desktop", width: 1280, height: 960 },
] as const;

async function stabilizeUi(page: Page) {
  await page.addStyleTag({
    content: `
      *, *::before, *::after {
        animation: none !important;
        transition: none !important;
      }
    `,
  });
  await page.waitForTimeout(250);
}

async function gotoSwap(page: Page, theme: "light" | "dark") {
  await page.goto("/swap");
  await page.waitForSelector("[data-testid='swap-card']", { timeout: 10_000 });
  await page.evaluate((selectedTheme) => {
    document.documentElement.classList.toggle("dark", selectedTheme === "dark");
  }, theme);
  await stabilizeUi(page);
}

for (const theme of ["light", "dark"] as const) {
  test.describe(`swap visual baseline (${theme})`, () => {
    for (const viewport of BREAKPOINTS) {
      test(`swap idle ${viewport.name}`, async ({ page }) => {
        await page.setViewportSize({ width: viewport.width, height: viewport.height });
        await gotoSwap(page, theme);
        await expect(page).toHaveScreenshot(`swap-idle-${theme}-${viewport.name}.png`);
      });

      test(`route summary ${viewport.name}`, async ({ page }) => {
        await page.setViewportSize({ width: viewport.width, height: viewport.height });
        await gotoSwap(page, theme);
        await page.getByRole("button", { name: /connect wallet/i }).click();
        await page.locator("input[placeholder='0.00']").first().fill("42");
        await page.waitForTimeout(600);
        await expect(page).toHaveScreenshot(`swap-routes-${theme}-${viewport.name}.png`);
      });

      test(`wallet connect state ${viewport.name}`, async ({ page }) => {
        await page.setViewportSize({ width: viewport.width, height: viewport.height });
        await gotoSwap(page, theme);
        await expect(page.getByRole("button", { name: /connect wallet/i })).toHaveScreenshot(
          `swap-wallet-connect-${theme}-${viewport.name}.png`,
        );
      });
    }
  });
}

