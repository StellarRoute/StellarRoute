import type { Page } from "@playwright/test";

export interface FreighterMockOptions {
  signBehavior?: "resolve" | "reject";
  signedTxXdr?: string;
  rejectMessage?: string;
  horizonSubmitStatus?: 200 | 400 | "hang";
  horizonSubmitHash?: string;
}

export const E2E_USDC_ISSUER =
  "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";

export const E2E_WALLET_ADDRESS =
  "GAKCEEZFGQ6HPTROFALCQXBEMJSF6DMZEHRYCGPTNJ5KLW3ZW7NXAEIO";

/**
 * Configure Freighter mock behavior (via window.__stellarrouteFreighterMock) and
 * stub Horizon account + transaction routes required by the swap pipeline.
 */
export async function setupSwapE2E(
  page: Page,
  options: FreighterMockOptions = {}
) {
  const {
    signBehavior = "resolve",
    signedTxXdr = "AAAAmock_signed_xdr_e2e",
    rejectMessage = "User declined",
    horizonSubmitStatus = 200,
    horizonSubmitHash = "e2e_tx_hash",
  } = options;

  await page.addInitScript((config) => {
    localStorage.clear();
    localStorage.setItem("stellarroute.wallet.lastWalletId", "freighter");
    localStorage.setItem("stellarroute.wallet.autoReconnect", "true");
    (
      window as unknown as {
        __stellarrouteFreighterMock?: FreighterMockOptions;
      }
    ).__stellarrouteFreighterMock = config;
  }, { signBehavior, signedTxXdr, rejectMessage });

  await page.route("**/accounts/**", async (route) => {
    if (route.request().method() !== "GET") {
      await route.continue();
      return;
    }
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        id: E2E_WALLET_ADDRESS,
        account_id: E2E_WALLET_ADDRESS,
        sequence: "12345",
        balances: [{ balance: "1000.0000000", asset_type: "native" }],
      }),
    });
  });

  await page.route("**/horizon*/transactions", async (route) => {
    if (route.request().method() !== "POST") {
      await route.continue();
      return;
    }
    if (horizonSubmitStatus === "hang") {
      return;
    }
    if (horizonSubmitStatus === 400) {
      await route.fulfill({
        status: 400,
        contentType: "application/json",
        body: JSON.stringify({
          title: "Transaction Failed",
          detail: "Transaction failed: tx_bad_seq",
          extras: { result_codes: { transaction: "tx_bad_seq" } },
        }),
      });
      return;
    }
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ hash: horizonSubmitHash }),
    });
  });
}
