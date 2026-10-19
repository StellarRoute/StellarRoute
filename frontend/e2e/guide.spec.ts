import { expect, test, type Page } from "@playwright/test";

/**
 * E2E coverage for the public `/guide` page (issue #1287).
 *
 * `/guide` is a static, document-style route. It must render for an anonymous
 * visitor and must never take part in a swap. The two negative acceptance
 * criteria (no Freighter prompt, no prepare/submit) are asserted *positively*
 * so the spec fails loudly if `/guide` ever starts touching the wallet or the
 * swap transaction pipeline.
 */

const GUIDE_HEADING = /your first live swap/i;

/**
 * Install a Freighter stub that records every prompting/signing call. Reading
 * wallet availability is fine; `requestAccess` (the permission prompt) and
 * `signTransaction` are not, so they are captured into `window.__walletCalls`.
 */
async function installWalletProbe(page: Page) {
  await page.addInitScript(() => {
    const calls: string[] = [];
    (window as unknown as { __walletCalls: string[] }).__walletCalls = calls;
    (window as unknown as { freighterApi: Record<string, unknown> }).freighterApi = {
      isConnected: async () => ({ isConnected: false, error: null }),
      isAllowed: async () => ({ isAllowed: false, error: null }),
      requestAccess: async () => {
        calls.push("requestAccess");
        return { address: "GABC" };
      },
      getAddress: async () => ({ address: null }),
      getNetwork: async () => ({ network: "testnet", networkPassphrase: "" }),
      getNetworkDetails: async () => ({
        network: "testnet",
        networkUrl: "",
        networkPassphrase: "",
      }),
      signTransaction: async () => {
        calls.push("signTransaction");
        return { signedTxXdr: "AAAA" };
      },
      setAllowed: async () => ({ isAllowed: true }),
      WatchWalletChanges: () => undefined,
    };
  });
}

test.describe("/guide", () => {
  test("loads for an anonymous visitor and renders the guide", async ({ page }) => {
    await page.goto("/guide");

    // Primary heading.
    await expect(
      page.getByRole("heading", { level: 1, name: GUIDE_HEADING })
    ).toBeVisible();

    // First and last documented steps are present.
    await expect(
      page.getByRole("heading", { name: /connect your wallet/i })
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: /confirm in your wallet/i })
    ).toBeVisible();

    // Landing on the guide must not auto-open the wallet onboarding dialog.
    await expect(page.getByRole("dialog")).toHaveCount(0);
  });

  test("never prompts the wallet for access", async ({ page }) => {
    await installWalletProbe(page);

    await page.goto("/guide");
    await expect(
      page.getByRole("heading", { level: 1, name: GUIDE_HEADING })
    ).toBeVisible();

    // Read through the guide; no interaction may trigger a wallet prompt.
    await page
      .getByRole("heading", { name: /confirm in your wallet/i })
      .scrollIntoViewIfNeeded();
    await page.waitForTimeout(200);

    const walletCalls = await page.evaluate(
      () =>
        (window as unknown as { __walletCalls?: string[] }).__walletCalls ?? []
    );
    expect(walletCalls, "/guide must not prompt Freighter").toEqual([]);
  });

  test("never initiates a swap prepare or submit request", async ({ page }) => {
    const prepareRequests: string[] = [];
    const submitRequests: string[] = [];

    page.on("request", (request) => {
      const url = request.url();
      if (url.includes("/swap/prepare")) prepareRequests.push(url);
      if (url.includes("/swap/submit")) submitRequests.push(url);
    });

    await page.goto("/guide");
    await expect(
      page.getByRole("heading", { level: 1, name: GUIDE_HEADING })
    ).toBeVisible();

    await page
      .getByRole("heading", { name: /confirm in your wallet/i })
      .scrollIntoViewIfNeeded();
    await page.waitForTimeout(300);

    expect(prepareRequests, "/guide must not call /swap/prepare").toHaveLength(0);
    expect(submitRequests, "/guide must not call /swap/submit").toHaveLength(0);
  });
});
