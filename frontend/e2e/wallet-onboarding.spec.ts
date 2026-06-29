import { expect, test } from '@playwright/test';

function freighterMockInit(network: 'testnet' | 'mainnet') {
  return () => {
    const state = {
      address: 'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ',
      network,
    };

    (window as Record<string, unknown>).freighterApi = {
      isAllowed: async () => ({ isAllowed: true }),
      requestAccess: async () => ({ address: state.address }),
      getAddress: async () => ({ address: state.address }),
      getNetworkDetails: async () => ({
        network: state.network,
        networkUrl:
          state.network === 'mainnet'
            ? 'https://horizon.stellar.org'
            : 'https://horizon-testnet.stellar.org',
        networkPassphrase:
          state.network === 'mainnet'
            ? 'Public Global Stellar Network ; September 2015'
            : 'Test SDF Network ; September 2015',
      }),
      signTransaction: async () => ({ signedTxXdr: 'AAAA', signerAddress: state.address }),
      isConnected: async () => ({ isConnected: true }),
      getNetwork: async () => ({
        network: state.network,
        networkPassphrase:
          state.network === 'mainnet'
            ? 'Public Global Stellar Network ; September 2015'
            : 'Test SDF Network ; September 2015',
      }),
      setAllowed: async () => ({ isAllowed: true }),
      WatchWalletChanges: () => undefined,
    };
  };
}

async function startOnboarding(page: import('@playwright/test').Page) {
  await page.goto('/swap');
  await page.getByRole('button', { name: /connect wallet/i }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.getByRole('button', { name: /^continue$/i }).click();
}

test('wallet onboarding connects mocked Freighter and allows disconnect flow', async ({ page }) => {
  await page.addInitScript(freighterMockInit('testnet'));
  await startOnboarding(page);

  await page.getByRole('button', { name: /freighter/i }).click();
  await expect(page.getByText(/wallet connected!/i)).toBeVisible({ timeout: 6000 });
  await page.getByRole('button', { name: /start trading/i }).click();

  await expect(page.getByRole('button', { name: /disconnect/i })).toBeVisible();
  await page.getByRole('button', { name: /disconnect/i }).click();
  await expect(page.getByRole('button', { name: /connect wallet/i })).toBeVisible();
});

test('wallet onboarding shows network mismatch step when mocked wallet is on mainnet', async ({ page }) => {
  await page.addInitScript(freighterMockInit('mainnet'));
  await startOnboarding(page);

  await page.getByRole('button', { name: /freighter/i }).click();
  await expect(page.getByText(/network mismatch/i)).toBeVisible({ timeout: 6000 });
  await expect(page.getByText(/Wallet Network: mainnet/i)).toBeVisible();
  await page.getByRole('button', { name: /proceed anyway/i }).click();
  await expect(page.getByRole('dialog')).not.toBeVisible();
});
