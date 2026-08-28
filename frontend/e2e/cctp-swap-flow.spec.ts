/**
 * CCTP cross-chain swap E2E — mocked API + fake wallets (no real network).
 */
import { test, expect, type Page } from '@playwright/test';
import { E2E_WALLET_ADDRESS } from './fixtures/freighter-mock';

const STELLAR_G = E2E_WALLET_ADDRESS;
const USDC_SEPOLIA = '0x1c7d4b196cb0c7b01d743fbc6116a902379c7238';
const APPROVE_CALLDATA =
  '0x095ea7b3000000000000000000000000ab583c48284244c440797b756cad4614310b7489000000000000000000000000000000000000000000000000000000000000000a';
const BURN_CALLDATA = '0x00000001';

function jsonData(payload: unknown) {
  return JSON.stringify({ data: payload });
}

const EVM_ORIGINAL = '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0';
const EVM_WRONG = '0x1111111111111111111111111111111111111111';

function installFakeWallets(page: Page) {
  return page.addInitScript(
    ({ stellarG, evmOriginal }: { stellarG: string; evmOriginal: string }) => {
      (window as unknown as { __STELLAR_ROUTE_FLAGS__?: Record<string, boolean> }).__STELLAR_ROUTE_FLAGS__ =
        { swap_ui_v2: true };
      localStorage.setItem('stellarroute:onboarding:dismissed', 'true');
      localStorage.setItem('stellarroute.onboarding.seen', 'true');
      localStorage.setItem('stellarroute.onboarding.completed', 'true');
      localStorage.setItem('stellarroute.wallet.address', stellarG);
      localStorage.setItem('stellarroute.wallet.walletId', 'freighter');
      localStorage.setItem('stellarroute.wallet.autoReconnect', 'true');
      const clearVaultUnlessKept = () => {
        if (!sessionStorage.getItem('__cctp_test_keep_session')) {
          sessionStorage.removeItem('stellarroute:cctp:v1');
        }
        sessionStorage.removeItem('__cctp_test_keep_session');
      };
      clearVaultUnlessKept();

      let evmSendCount = 0;
      let stellarSignCount = 0;
      let evmAddress = evmOriginal;
      (window as unknown as { __cctpSetEvmAddress?: (addr: string) => void }).__cctpSetEvmAddress =
        (addr: string) => {
          evmAddress = addr;
        };
      (window as unknown as { __cctpWalletSendCount?: () => number }).__cctpWalletSendCount =
        () => evmSendCount;
      (window as unknown as { __cctpStellarSignCount?: () => number }).__cctpStellarSignCount =
        () => stellarSignCount;

      const ethereum = {
        isMetaMask: true,
        request: async ({ method }: { method: string }) => {
          if (method === 'eth_requestAccounts' || method === 'eth_accounts') {
            return [evmAddress];
          }
          if (method === 'eth_chainId') return '0xaa36a7';
          if (method === 'wallet_switchEthereumChain') return null;
          if (method === 'eth_sendTransaction') {
            evmSendCount += 1;
            return '0xdeadbeef';
          }
          if (method === 'eth_getTransactionReceipt') {
            return { status: '0x1', transactionHash: '0xdeadbeef' };
          }
          if (method === 'personal_sign') return '0x' + 'ab'.repeat(32);
          return null;
        },
      };
      Object.defineProperty(window, 'ethereum', { value: ethereum, configurable: true });

      (window as unknown as { freighter?: unknown }).freighter = {
        isConnected: async () => true,
        isAllowed: async () => true,
        getPublicKey: async () => stellarG,
        getNetwork: async () => 'TESTNET',
        signTransaction: async () => {
          stellarSignCount += 1;
          return 'signed-xdr-mock';
        },
      };
    },
    { stellarG: STELLAR_G, evmOriginal: EVM_ORIGINAL },
  );
}

function mockNetworkIsolation(page: Page) {
  return Promise.all([
    page.route('**/horizon-testnet.stellar.org/**', async (route) => {
      const method = route.request().method();
      if (method === 'POST') {
        return route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ hash: 'stellar-tx-hash-mock' }),
        });
      }
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ hash: 'stellar-tx-hash-mock' }),
      });
    }),
    page.route('**/horizon.stellar.org/**', (route) =>
      route.fulfill({ status: 404, body: '{}' }),
    ),
    page.route('**/api/health**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ status: 'ok' }),
      }),
    ),
  ]);
}

function mockCctpApi(
  page: Page,
  opts: {
    direction?: 'evm_to_stellar' | 'stellar_to_evm';
    transferId?: string;
  } = {},
) {
  const transferId = opts.transferId ?? 'transfer-e2e-1';
  const direction = opts.direction ?? 'evm_to_stellar';
  let burnPhase: 'approval' | 'burn' = 'approval';
  let status = 'burn_prepared';
  let getTransferCount = 0;

  return page.route('**/api/v2**', async (route) => {
    const url = route.request().url();
    const method = route.request().method();

    if (url.endsWith('/api/v2') && method === 'GET') {
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: jsonData({
          version: 2,
          chain_aware_assets: true,
          bridge_venues_metadata_only: false,
          bridge_settlement_executable: true,
          supported_chain_namespaces: ['stellar', 'eip155'],
          supported_corridors: [
            {
              corridor_id: 'circle-cctp:usdc:stellar-testnet:ethereum-sepolia',
              provider: 'circle-cctp',
              direction: 'evm_to_stellar',
              source_chain_id: 'eip155:11155111',
              destination_chain_id: 'stellar:testnet',
              source_asset: {
                chain_id: 'eip155:11155111',
                asset: `erc20:${USDC_SEPOLIA}`,
                canonical: `eip155:11155111/erc20:${USDC_SEPOLIA}`,
                symbol: 'USDC',
              },
              destination_asset: {
                chain_id: 'stellar:testnet',
                asset: 'erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA',
                canonical:
                  'stellar:testnet/erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA',
                symbol: 'USDC',
              },
              executable: true,
            },
            {
              corridor_id: 'circle-cctp:usdc:stellar-testnet:ethereum-sepolia',
              provider: 'circle-cctp',
              direction: 'stellar_to_evm',
              source_chain_id: 'stellar:testnet',
              destination_chain_id: 'eip155:11155111',
              source_asset: {
                chain_id: 'stellar:testnet',
                asset: 'erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA',
                canonical:
                  'stellar:testnet/erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA',
                symbol: 'USDC',
              },
              destination_asset: {
                chain_id: 'eip155:11155111',
                asset: `erc20:${USDC_SEPOLIA}`,
                canonical: `eip155:11155111/erc20:${USDC_SEPOLIA}`,
                symbol: 'USDC',
              },
              executable: true,
            },
          ],
        }),
      });
    }

    if (url.includes('/bridge/cctp/quote') && method === 'POST') {
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: jsonData({
          transfer_id: transferId,
          access_token: 'access-mock-token',
          corridor_id: 'stellar-testnet-sepolia',
          provider: 'circle-cctp',
          direction,
          source_amount: '10',
          destination_amount: '9.99',
          fee_quote: {},
          expires_at: Math.floor(Date.now() / 1000) + 600,
          finality: 'standard',
        }),
      });
    }

    if (url.includes('/prepare-burn') && method === 'POST') {
      const approval = burnPhase === 'approval';
      const evmPayload = {
        type: 'evm_transaction' as const,
        chain_id: 'eip155:11155111',
        to: USDC_SEPOLIA,
        data: approval ? APPROVE_CALLDATA : BURN_CALLDATA,
        value: '0',
      };
      const stellarPayload = {
        type: 'stellar_xdr' as const,
        network_passphrase: 'Test SDF Network ; September 2015',
        xdr_envelope: approval ? 'AAAAapproval' : 'AAAAburn',
      };
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: jsonData({
          transfer_id: transferId,
          status: 'burn_prepared',
          approval_required: approval,
          expires_at: Math.floor(Date.now() / 1000) + 300,
          payload: direction === 'stellar_to_evm' ? stellarPayload : evmPayload,
        }),
      });
    }

    if (url.includes('/submit-burn') && method === 'POST') {
      if (burnPhase === 'approval') burnPhase = 'burn';
      else status = 'awaiting_attestation';
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: jsonData({
          transfer_id: transferId,
          status,
          source_tx_hash: '0xdeadbeef',
        }),
      });
    }

    if (url.includes(`/bridge/cctp/${transferId}`) && method === 'GET') {
      getTransferCount += 1;
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: jsonData({
          transfer_id: transferId,
          corridor_id: 'stellar-testnet-sepolia',
          provider: 'circle-cctp',
          direction,
          status,
          retryable: false,
          _get_count: getTransferCount,
        }),
      });
    }

    return route.fallback();
  });
}

async function dismissWalletOverlay(page: Page) {
  await page.keyboard.press('Escape');
  await page.locator('[data-slot="dialog-overlay"]').waitFor({ state: 'hidden', timeout: 3000 }).catch(() => {});
}

async function setupEvmCorridor(page: Page) {
  const tab = page.getByTestId('corridor-tab-evm-to-stellar');
  await tab.waitFor({ state: 'visible' });
  await tab.click();
  await page.waitForSelector('[data-testid="cctp-source-amount"]', { timeout: 20_000 });
  await page.getByTestId('cctp-source-amount').fill('10');
  await page.getByTestId('wallet-chip-ethereum-sepolia').click();
  await page.getByRole('button', { name: /EVM Wallet/i }).click();
  await dismissWalletOverlay(page);
  await page.getByTestId('wallet-chip-stellar-mint-submitter').click();
  await page.getByRole('button', { name: /Freighter/i }).click();
  await dismissWalletOverlay(page);
}

async function setupStellarCorridor(page: Page) {
  await page.evaluate(() => sessionStorage.removeItem('stellarroute:cctp:v1'));
  await page.getByTestId('corridor-tab-stellar-to-evm').click();
  await page.waitForSelector('[data-testid="cctp-source-amount"]', { timeout: 20_000 });
  await page.getByTestId('cctp-source-amount').fill('10');
  await page.getByTestId('wallet-chip-stellar').click();
  await page.getByRole('button', { name: /Freighter/i }).click();
  await dismissWalletOverlay(page);
  await page.getByTestId('wallet-chip-ethereum-sepolia').click();
  await page.getByRole('button', { name: /EVM Wallet/i }).click();
  await dismissWalletOverlay(page);
  await expect(page.getByTestId('wallet-chip-stellar')).toContainText(/GAKC/i, {
    timeout: 15_000,
  });
}

function attachConsoleGuards(page: Page) {
  const errors: string[] = [];
  page.on('console', (msg) => {
    if (msg.type() === 'error') errors.push(msg.text());
  });
  page.on('pageerror', (err) => errors.push(err.message));
  return errors;
}

async function startSwapFresh(page: Page) {
  await page.goto('/swap');
  await page.evaluate(
    ({ evmOriginal }: { evmOriginal: string }) => {
      sessionStorage.removeItem('stellarroute:cctp:v1');
      (
        window as unknown as { __cctpSetEvmAddress?: (addr: string) => void }
      ).__cctpSetEvmAddress?.(evmOriginal);
    },
    { evmOriginal: EVM_ORIGINAL },
  );
  await page.reload();
  await page.waitForSelector('[data-testid="cross-chain-swap-deck"]', {
    timeout: 20_000,
  });
}

test.describe('CCTP swap flow (mocked)', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await installFakeWallets(page);
    await mockNetworkIsolation(page);
    await mockCctpApi(page);
  });

  test('desktop corridor shows deck and hides secrets in DOM', async ({ page }) => {
    const consoleErrors = attachConsoleGuards(page);
    const networkBodies: string[] = [];
    page.on('requestfinished', async (req) => {
      if (req.url().includes('/bridge/cctp/')) {
        networkBodies.push((await req.postData()) ?? '');
      }
    });
    await startSwapFresh(page);
    await page.getByTestId('corridor-tab-evm-to-stellar').click();
    const html = await page.content();
    expect(html).not.toMatch(/access-mock-token/);
    expect(html).not.toMatch(/signed-xdr-mock/);
    expect(html).not.toMatch(APPROVE_CALLDATA);
    expect(networkBodies.join('\n')).not.toMatch(/signed-xdr-mock/);
    expect(consoleErrors.join('\n')).not.toMatch(/Maximum update depth exceeded/i);
    await page.screenshot({ path: 'test-results/cctp-deck-desktop.png' });
  });

  test('mobile viewport renders cross-chain deck', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    const consoleErrors = attachConsoleGuards(page);
    await startSwapFresh(page);
    await page.waitForSelector('[data-testid="paired-chain-selectors"]', { timeout: 20_000 });
    await page.getByTestId('corridor-tab-evm-to-stellar').click({ force: true });
    await page.waitForSelector('[data-testid="cctp-source-amount"]', { timeout: 20_000 });
    expect(consoleErrors.join('\n')).not.toMatch(/Maximum update depth exceeded/i);
    await page.screenshot({ path: 'test-results/cctp-deck-mobile.png' });
  });

  test('EVM: prepare → approve uses exactly one wallet send', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 });
    await startSwapFresh(page);
    await setupEvmCorridor(page);

    const cta = page.getByTestId('cross-chain-review-cta');
    await cta.click();
    await expect(cta).toContainText(/Prepare/i, { timeout: 15_000 });
    await cta.click();
    await expect(cta).toContainText(/Approve/i, { timeout: 15_000 });
    await cta.click();

    const sendCount = await page.evaluate(() =>
      (window as unknown as { __cctpWalletSendCount?: () => number }).__cctpWalletSendCount?.(),
    );
    expect(sendCount).toBe(1);
    await page.screenshot({ path: 'test-results/cctp-evm-approve.png' });
  });

  test('Stellar→EVM shows server-driven Approve staging after prepare', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 });
    await mockCctpApi(page, { direction: 'stellar_to_evm', transferId: 'transfer-stellar-e2e' });
    await startSwapFresh(page);
    await setupStellarCorridor(page);

    const cta = page.getByTestId('cross-chain-review-cta');
    await cta.click();
    await expect(cta).toContainText(/Prepare/i, { timeout: 15_000 });
    await cta.click();
    await expect(cta).toContainText(/Approve USDC spend/i, { timeout: 15_000 });
    await page.screenshot({ path: 'test-results/cctp-stellar-approve-stage.png' });
  });

  test('reload → reconcile → re-prepare → approve uses one wallet send', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 });
    const consoleErrors = attachConsoleGuards(page);
    const getTransferUrls: string[] = [];
    let countGetsAfterReload = false;
    page.on('requestfinished', (req) => {
      if (
        countGetsAfterReload &&
        req.url().includes('/bridge/cctp/transfer-e2e-1') &&
        req.method() === 'GET'
      ) {
        getTransferUrls.push(req.url());
      }
    });

    await startSwapFresh(page);
    await setupEvmCorridor(page);
    const cta = page.getByTestId('cross-chain-review-cta');
    await cta.click();
    await cta.click();
    await expect(cta).toContainText(/Approve/i, { timeout: 15_000 });

    await expect
      .poll(async () =>
        page.evaluate(() => {
          const raw = sessionStorage.getItem('stellarroute:cctp:v1');
          if (!raw) return false;
          const record = JSON.parse(raw) as {
            version?: number;
            recovery?: { walletBindings?: unknown };
          };
          return record.version === 2 && Boolean(record.recovery?.walletBindings);
        }),
      )
      .toBe(true);

    await page.evaluate(() =>
      sessionStorage.setItem('__cctp_test_keep_session', '1'),
    );
    await page.reload();
    countGetsAfterReload = true;
    await page.waitForResponse(
      (response) =>
        response.url().includes('/api/v2') && response.status() === 200,
      { timeout: 30_000 },
    );
    await expect
      .poll(async () =>
        page.evaluate(
          () => sessionStorage.getItem('stellarroute:cctp:v1') !== null,
        ),
      )
      .toBe(true);
    await page.waitForSelector('[data-testid="cctp-execution-panel"]', {
      timeout: 20_000,
    });
    await page.getByTestId('wallet-chip-ethereum-sepolia').click();
    await page.getByRole('button', { name: /EVM Wallet/i }).click();
    await dismissWalletOverlay(page);
    await expect(cta).toContainText(/Re-prepare transaction/i, { timeout: 15_000 });

    const sendsBeforeReprepare = await page.evaluate(() =>
      (window as unknown as { __cctpWalletSendCount?: () => number }).__cctpWalletSendCount?.(),
    );
    expect(sendsBeforeReprepare).toBe(0);

    await cta.click();
    await expect(cta).toContainText(/Approve USDC spend/i);

    await cta.click();
    const sendCount = await page.evaluate(() =>
      (window as unknown as { __cctpWalletSendCount?: () => number }).__cctpWalletSendCount?.(),
    );
    expect(sendCount).toBe(1);
    expect(getTransferUrls.length).toBeGreaterThan(0);
    expect(getTransferUrls.length).toBeLessThanOrEqual(2);

    const html = await page.content();
    expect(html).not.toMatch(/access-mock-token/);
    expect(html).not.toMatch(APPROVE_CALLDATA);
    expect(consoleErrors.join('\n')).not.toMatch(/access-mock-token/i);
    expect(consoleErrors.join('\n')).not.toMatch(/Maximum update depth exceeded/i);
    await page.screenshot({ path: 'test-results/cctp-reload-reprepare-approve.png' });
  });

  test('reload with different fake wallet shows mismatch then resumes with one send', async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1280, height: 720 });
    const consoleErrors = attachConsoleGuards(page);
    await startSwapFresh(page);
    await setupEvmCorridor(page);
    const cta = page.getByTestId('cross-chain-review-cta');
    await cta.click();
    await cta.click();
    await expect(cta).toContainText(/Approve/i, { timeout: 15_000 });

    await page.evaluate(() =>
      sessionStorage.setItem('__cctp_test_keep_session', '1'),
    );
    await page.reload();
    await page.waitForResponse(
      (response) =>
        response.url().includes('/api/v2') && response.status() === 200,
      { timeout: 30_000 },
    );
    await page.waitForSelector('[data-testid="cctp-execution-panel"]', {
      timeout: 20_000,
    });

    await page.evaluate((wrong) => {
      (window as unknown as { __cctpSetEvmAddress?: (addr: string) => void }).__cctpSetEvmAddress?.(
        wrong,
      );
    }, EVM_WRONG);
    await page.getByTestId('wallet-chip-ethereum-sepolia').click();
    await page.getByRole('button', { name: /EVM Wallet/i }).click();
    await dismissWalletOverlay(page);

    await expect(page.getByTestId('cctp-wallet-recovery-card')).toBeVisible();
    await expect(cta).toBeDisabled();

    await page.evaluate((original) => {
      (window as unknown as { __cctpSetEvmAddress?: (addr: string) => void }).__cctpSetEvmAddress?.(
        original,
      );
    }, EVM_ORIGINAL);
    await page.getByTestId('wallet-chip-ethereum-sepolia').click();
    await page.getByRole('button', { name: /EVM Wallet/i }).click();
    await dismissWalletOverlay(page);
    await expect(page.getByTestId('cctp-wallet-recovery-card')).toBeHidden();

    await expect(cta).toContainText(/Re-prepare transaction/i, { timeout: 15_000 });
    await cta.click();
    await expect(cta).toContainText(/Approve/i, { timeout: 15_000 });
    await cta.click();
    const sendCount = await page.evaluate(() =>
      (window as unknown as { __cctpWalletSendCount?: () => number }).__cctpWalletSendCount?.(),
    );
    expect(sendCount).toBe(1);

    const html = await page.content();
    expect(html).not.toMatch(/access-mock-token/);
    expect(html).not.toMatch(APPROVE_CALLDATA);
    expect(consoleErrors.join('\n')).not.toMatch(/Maximum update depth exceeded/i);
    await page.screenshot({ path: 'test-results/cctp-wallet-mismatch-recovery.png' });
  });
});
