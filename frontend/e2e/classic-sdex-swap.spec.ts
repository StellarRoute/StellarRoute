/**
 * E2E: mocked Freighter one-hop SDEX happy path (issue #1201).
 *
 * Covers the live swap journey end to end against route mocks only — no real
 * wallet, no real Horizon, no testnet funds:
 *
 *   connect mock Freighter → quote → POST /swap/prepare → wallet sign →
 *   POST /swap/submit → Horizon confirmation → "Swap confirmed"
 *
 * The spec is additive: it lives in `e2e/` and reuses
 * `e2e/fixtures/freighter-mock.ts`. It does not touch SwapCard, wallet
 * adapters, or any production component.
 *
 * Everything the page can reach is either mocked or explicitly recorded: a
 * catch-all Horizon guard (registered first, so it has the lowest precedence)
 * aborts and records any Horizon request no other route handled, which is how
 * the "no real Horizon" guarantee is asserted rather than assumed.
 */

import { test, expect, type Page } from "@playwright/test";
import {
  setupSwapE2E,
  E2E_USDC_ISSUER,
  E2E_WALLET_ADDRESS,
} from "./fixtures/freighter-mock";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const USDC_TOKEN = `USDC:${E2E_USDC_ISSUER}`;
const TESTNET_PASSPHRASE = "Test SDF Network ; September 2015";

const QUOTE_ID = "e2e-quote-id";
const TX_HASH = "e2e_tx_hash";
const PREPARED_XDR = "AAAAunsigned_envelope_e2e";
const SIGNED_XDR = "AAAAmock_signed_xdr_e2e";
const EXPECTED_OUTPUT = "1.4500000";
const MIN_OUTPUT = "1.4427500";

interface JourneyRecorder {
  prepareBodies: unknown[];
  submitBodies: unknown[];
  unmockedHorizonRequests: string[];
}

function freshRecorder(): JourneyRecorder {
  return {
    prepareBodies: [],
    submitBodies: [],
    unmockedHorizonRequests: [],
  };
}

/** Deterministic one-hop SDEX quote — the venue the live CTA can execute. */
function oneHopQuoteFixture() {
  const native = { asset_type: "native" };
  const usdc = {
    asset_type: "credit_alphanum4",
    asset_code: "USDC",
    asset_issuer: E2E_USDC_ISSUER,
  };
  return {
    base_asset: native,
    quote_asset: usdc,
    amount: "10",
    price: "0.145",
    total: EXPECTED_OUTPUT,
    quote_type: "sell",
    degraded: false,
    path: [
      {
        from_asset: native,
        to_asset: usdc,
        price: "0.145",
        source: "sdex",
        liquidity_depth: "500.0000000",
        fee_bps: 0,
      },
    ],
    timestamp: Date.now(),
    expires_at: Date.now() + 30_000,
    source_timestamp: Date.now(),
    ttl_seconds: 30,
    price_impact: "0.10",
    alternativeRoutes: [],
  };
}

function prepareFixture() {
  return {
    quote_id: QUOTE_ID,
    xdr_envelope: PREPARED_XDR,
    expected_output: EXPECTED_OUTPUT,
    min_output: MIN_OUTPUT,
    expires_at: Math.floor(Date.now() / 1000) + 120,
    execution_mode: "classic_path_payment",
    network_passphrase: TESTNET_PASSPHRASE,
  };
}

function submitFixture() {
  return {
    quote_id: QUOTE_ID,
    tx_hash: TX_HASH,
    status: "submitted",
    output_amount: EXPECTED_OUTPUT,
    ledger: 1234,
  };
}

// ---------------------------------------------------------------------------
// Mock wiring
// ---------------------------------------------------------------------------

/**
 * Install a mocked Freighter *extension* inside the page.
 *
 * `resolveFreighterApi` prefers the real `@stellar/freighter-api` module over
 * `window.freighterApi`, and every call that module makes reaches the
 * extension over `window.postMessage`
 * (`FREIGHTER_EXTERNAL_MSG_REQUEST` → `FREIGHTER_EXTERNAL_MSG_RESPONSE`,
 * matched on the `messagedId` misspelling the package actually reads).
 *
 * Answering that protocol is what makes detection, auto-reconnect and signing
 * work with no extension installed. It also avoids the failure the app hits
 * today without one: `requestAccess()` has no timeout, so a page nobody
 * answers leaves `WalletProvider` stuck in `isLoading` and the wallet picker
 * disabled forever.
 *
 * The `window.freighterApi` object below stays as a fallback for bundles where
 * the module itself does not resolve. Both read the same
 * `window.__stellarrouteFreighterMock` config the fixture sets.
 */
async function installClassicSwapSurface(page: Page) {
  await page.addInitScript(() => {
    const win = window as unknown as {
      __STELLAR_ROUTE_FLAGS__?: Record<string, boolean>;
    };
    win.__STELLAR_ROUTE_FLAGS__ = {
      ...win.__STELLAR_ROUTE_FLAGS__,
      // `swap_ui_v2` defaults to ON, which renders `CrossChainSwapDeck` —
      // not the classic `SwapCard` this issue exercises. `useFeatureFlag`
      // documents the window map as the dev/e2e override and reads it
      // post-hydration, so pinning the legacy surface here keeps the spec
      // free of env/config changes.
      swap_ui_v2: false,
      routes_beta: false,
    };
  });
}

async function installFreighterExtensionShim(page: Page) {
  await page.addInitScript((mockAddress: string) => {
    type MockConfig = {
      signBehavior?: "resolve" | "reject";
      signedTxXdr?: string;
      rejectMessage?: string;
    };

    const win = window as unknown as {
      freighter?: boolean;
      freighterApi?: unknown;
      __stellarrouteFreighterMock?: MockConfig;
    };
    const readConfig = (): MockConfig => win.__stellarrouteFreighterMock ?? {};

    // The global a real extension injects; `isConnected()` short-circuits on it.
    win.freighter = true;

    const networkDetails = {
      network: "testnet",
      networkUrl: "https://horizon-testnet.stellar.org",
      networkPassphrase: "Test SDF Network ; September 2015",
    };

    window.addEventListener("message", (event: MessageEvent) => {
      if (event.source !== window) return;
      const request = event.data as
        | { source?: string; messageId?: number; type?: string }
        | undefined;
      if (!request?.source || request.source !== "FREIGHTER_EXTERNAL_MSG_REQUEST") {
        return;
      }

      const respond = (payload: Record<string, unknown>) => {
        window.postMessage(
          {
            source: "FREIGHTER_EXTERNAL_MSG_RESPONSE",
            // `messagedId` is not a typo on our side: the package reads it.
            messagedId: request.messageId,
            ...payload,
          },
          window.location.origin,
        );
      };

      switch (request.type) {
        case "REQUEST_CONNECTION_STATUS":
          respond({ isConnected: true, publicKey: mockAddress });
          return;
        case "REQUEST_PUBLIC_KEY":
        case "REQUEST_ACCESS":
          respond({ publicKey: mockAddress });
          return;
        case "REQUEST_ALLOWED_STATUS":
        case "SET_ALLOWED_STATUS":
          respond({ isAllowed: true });
          return;
        case "REQUEST_NETWORK":
          // Both network calls are read as a nested `networkDetails` object.
          respond({ networkDetails });
          return;
        case "REQUEST_NETWORK_DETAILS":
          // The package destructures `{ networkDetails, apiError }` — a flat
          // payload throws `Cannot destructure property 'network'` inside
          // `getNetworkDetails()` and strands reconnect.
          respond({ networkDetails, apiError: "" });
          return;
        case "SUBMIT_TRANSACTION": {
          const config = readConfig();
          if (config.signBehavior === "reject") {
            respond({
              apiError: {
                code: -1,
                message: config.rejectMessage ?? "User declined",
              },
            });
            return;
          }
          respond({
            signedTransaction: config.signedTxXdr ?? "AAAAmock_signed_xdr_e2e",
            signerAddress: mockAddress,
          });
          return;
        }
        default:
          respond({});
      }
    });

    win.freighterApi = {
      isConnected: async () => ({ isConnected: true }),
      isAllowed: async () => ({ isAllowed: true }),
      requestAccess: async () => ({ address: mockAddress }),
      getAddress: async () => ({ address: mockAddress }),
      getNetwork: async () => ({
        network: "testnet",
        networkPassphrase: "Test SDF Network ; September 2015",
      }),
      getNetworkDetails: async () => ({
        network: "testnet",
        networkUrl: "https://horizon-testnet.stellar.org",
        networkPassphrase: "Test SDF Network ; September 2015",
      }),
      setAllowed: async () => ({ isAllowed: true }),
      signTransaction: async () => {
        const config = readConfig();
        if (config.signBehavior === "reject") {
          return { error: { message: config.rejectMessage ?? "User declined" } };
        }
        return {
          signedTxXdr: config.signedTxXdr ?? "AAAAmock_signed_xdr_e2e",
          signerAddress: mockAddress,
        };
      },
      WatchWalletChanges: () => undefined,
    };
  }, E2E_WALLET_ADDRESS);
}

/**
 * Record and abort every Horizon request no other route mock handled, so the
 * test fails loudly if the journey ever reaches for a real network.
 */
async function installHorizonGuard(page: Page, recorder: JourneyRecorder) {
  await page.route("**horizon*.stellar.org/**", async (route) => {
    recorder.unmockedHorizonRequests.push(
      `${route.request().method()} ${route.request().url()}`,
    );
    await route.abort("connectionrefused");
  });
  await page.route("**/horizon*/**", async (route) => {
    recorder.unmockedHorizonRequests.push(
      `${route.request().method()} ${route.request().url()}`,
    );
    await route.abort("connectionrefused");
  });
}

/** Quote + route mocks for a one-hop SDEX pair. */
async function installQuoteMocks(page: Page) {
  await page.route("**/api/v1/quote/**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(oneHopQuoteFixture()),
    });
  });

  await page.route("**/api/v1/routes/**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ routes: [] }),
    });
  });
}

/** Mock the classic API prepare → sign → submit legs and record the bodies. */
async function installSwapApiMocks(page: Page, recorder: JourneyRecorder) {
  await page.route("**/api/v1/swap/prepare", async (route) => {
    recorder.prepareBodies.push(safeJson(route.request().postData()));
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ data: prepareFixture() }),
    });
  });

  await page.route("**/api/v1/swap/submit", async (route) => {
    recorder.submitBodies.push(safeJson(route.request().postData()));
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ data: submitFixture() }),
    });
  });

  // Post-submit confirmation poll: GET /transactions/:hash (fixture only
  // handles POST /transactions, so this is the browser's confirm poll).
  await page.route("**/horizon*/transactions/*", async (route) => {
    if (route.request().method() !== "GET") {
      await route.fallback();
      return;
    }
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ hash: TX_HASH, successful: true }),
    });
  });
}

function safeJson(raw: string | null): unknown {
  if (!raw) return null;
  try {
    return JSON.parse(raw);
  } catch {
    return raw;
  }
}

// ---------------------------------------------------------------------------
// Journey helpers
// ---------------------------------------------------------------------------

function swapPageUrl(amount = "10") {
  const params = new URLSearchParams({
    from: "native",
    to: USDC_TOKEN,
    amount,
  });
  return `/swap?${params.toString()}`;
}

async function dismissSessionRecovery(page: Page) {
  const startFresh = page.getByRole("button", { name: /start fresh/i });
  if (await startFresh.isVisible().catch(() => false)) {
    await startFresh.click();
  }
}

/**
 * Wait for a live wallet session.
 *
 * The fixture seeds `stellarroute.wallet.lastWalletId` + `autoReconnect`, so
 * the provider normally reconnects on its own through the mocked extension
 * before this runs. If the first-visit connect dialog wins that race instead,
 * drive it (Continue → Freighter) — both paths end on the same session.
 */
async function ensureWalletConnected(page: Page) {
  const connected = page.locator('#wallet-button[aria-label^="Wallet "]');
  const dialog = page.getByTestId("wallet-connect-dialog");

  await expect
    .poll(
      async () => {
        if ((await connected.count()) === 1) return true;
        if (!(await dialog.isVisible().catch(() => false))) return false;

        const continueButton = dialog.getByRole("button", { name: /^continue$/i });
        if (await continueButton.isVisible().catch(() => false)) {
          await continueButton.click();
        }
        await dialog
          .getByRole("button", { name: /freighter/i })
          .first()
          .click({ timeout: 5_000 })
          .catch(() => undefined);
        return false;
      },
      { timeout: 45_000, intervals: [1_000] },
    )
    .toBe(true);

  await expect(dialog).toHaveCount(0, { timeout: 15_000 });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

test.describe("classic one-hop SDEX swap (mocked Freighter)", () => {
  let recorder: JourneyRecorder;

  test.beforeEach(async ({}, testInfo) => {
    recorder = freshRecorder();
    // Playwright's 30s default is shorter than a cold browser launch on a
    // loaded machine; extend before the `page` fixture is created.
    testInfo.setTimeout(Math.max(testInfo.timeout, 150_000));
  });

  test.afterEach(async ({ page }) => {
    // A setup-timeout can leave the fixture null; still assert the guard.
    await page?.unrouteAll({ behavior: "ignoreErrors" });
    expect(
      recorder.unmockedHorizonRequests,
      "no unmocked Horizon request may leave the browser",
    ).toEqual([]);
  });

  test("connect → quote → prepare → sign → submit → confirmed", async ({
    page,
  }) => {
    // Cold Next.js compiles routinely exceed Playwright's 30s default.
    test.setTimeout(150_000);

    // Lowest precedence first: everything else is registered after it.
    await installHorizonGuard(page, recorder);
    await setupSwapE2E(page, { horizonSubmitHash: TX_HASH });
    await installClassicSwapSurface(page);
    await installFreighterExtensionShim(page);
    await installQuoteMocks(page);
    await installSwapApiMocks(page, recorder);

    await page.goto(swapPageUrl(), { waitUntil: "domcontentloaded" });
    await dismissSessionRecovery(page);

    await expect(page.getByTestId("swap-card")).toBeVisible({ timeout: 60_000 });
    await ensureWalletConnected(page);

    const reviewButton = page.getByRole("button", { name: /review swap/i });
    await expect(reviewButton).toBeEnabled({ timeout: 20_000 });
    await reviewButton.click();

    await expect(
      page.getByRole("heading", { name: /swap confirmed/i }),
    ).toBeVisible({ timeout: 20_000 });

    // Wallet signed the exact envelope the API prepared.
    expect(recorder.prepareBodies).toHaveLength(1);
    expect(recorder.submitBodies).toHaveLength(1);

    const prepare = recorder.prepareBodies[0] as {
      route?: { hops?: Array<Record<string, unknown>> };
      amount?: string;
      sender?: string;
      slippage_bps?: number;
    };
    expect(prepare.amount).toBe("10");
    expect(prepare.sender).toBe(E2E_WALLET_ADDRESS);
    expect(typeof prepare.slippage_bps).toBe("number");
    expect(prepare.route?.hops).toHaveLength(1);
    expect(prepare.route?.hops?.[0].source).toBe("sdex");

    const submit = recorder.submitBodies[0] as {
      quote_id?: string;
      signed_xdr?: string;
    };
    expect(submit.quote_id).toBe(QUOTE_ID);
    expect(submit.signed_xdr).toBe(SIGNED_XDR);

    // Confirmation hash from the mocked Horizon poll reaches the success UI.
    await expect(page.getByText(TX_HASH)).toBeVisible({ timeout: 10_000 });
  });

  test("prepare is requested once and submit reuses its quote_id", async ({
    page,
  }) => {
    test.setTimeout(150_000);

    await installHorizonGuard(page, recorder);
    await setupSwapE2E(page, { horizonSubmitHash: TX_HASH });
    await installClassicSwapSurface(page);
    await installFreighterExtensionShim(page);
    await installQuoteMocks(page);
    await installSwapApiMocks(page, recorder);

    await page.goto(swapPageUrl("25"), { waitUntil: "domcontentloaded" });
    await dismissSessionRecovery(page);
    await ensureWalletConnected(page);

    const reviewButton = page.getByRole("button", { name: /review swap/i });
    await expect(reviewButton).toBeEnabled({ timeout: 20_000 });
    await reviewButton.click();

    await expect(
      page.getByRole("heading", { name: /swap confirmed/i }),
    ).toBeVisible({ timeout: 20_000 });

    const prepare = recorder.prepareBodies[0] as { amount?: string };
    expect(prepare.amount).toBe("25");
    expect(recorder.prepareBodies).toHaveLength(1);
    expect(recorder.submitBodies).toHaveLength(1);
  });
});
