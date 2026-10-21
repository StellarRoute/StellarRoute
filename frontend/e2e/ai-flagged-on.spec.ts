import { test, expect } from '@playwright/test';

const FIXTURE_ADDRESS = 'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN';

test.describe('AI agent flagged-on: parse, preview, cancel (AI-39)', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    // Inject the feature flag into the page before navigation so the React
    // hydration sees it. Does not touch any .env file or server configuration.
    await page.addInitScript(() => {
      Object.defineProperty(window, '__STELLAR_ROUTE_FLAGS__', {
        value: { ai_agent: true },
        writable: false,
      });
    });

    // Stub /api/v1/agent/health so the status chip shows "available" without
    // a live API, and stub /api/v1/agent/intents/validate so parse succeeds.
    await page.route('/api/v1/agent/health', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ enabled: true }) }),
    );
    await page.route('/api/v1/agent/intents/validate', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ type: 'send', amount: '1' }) }),
    );
  });

  test('preview text includes the amount after submitting a send intent', async ({ page }) => {
    await page.goto('/ai');

    await expect(page.getByTestId('ai-page-shell')).toBeVisible();

    const input = page.getByTestId('agent-chat-input');
    await input.fill(`send 1 XLM to ${FIXTURE_ADDRESS}`);
    await page.getByTestId('agent-chat-submit').click();

    const preview = page.getByTestId('intent-preview-card');
    await expect(preview).toBeVisible();

    const description = page.getByTestId('intent-description');
    await expect(description).toContainText('1');
  });

  test('cancel removes the Confirm button', async ({ page }) => {
    await page.goto('/ai');

    await expect(page.getByTestId('ai-page-shell')).toBeVisible();

    const input = page.getByTestId('agent-chat-input');
    await input.fill(`send 1 XLM to ${FIXTURE_ADDRESS}`);
    await page.getByTestId('agent-chat-submit').click();

    await expect(page.getByTestId('intent-preview-card')).toBeVisible();
    await page.getByTestId('cancel-btn').click();

    await expect(page.getByTestId('intent-preview-card')).toHaveCount(0);
    await expect(page.getByTestId('confirm-btn')).toHaveCount(0);
  });

  test('cancel does not trigger a /swap/prepare request', async ({ page }) => {
    const prepareRequests: string[] = [];
    page.on('request', (req) => {
      if (req.url().includes('/swap/prepare')) {
        prepareRequests.push(req.url());
      }
    });

    await page.goto('/ai');

    const input = page.getByTestId('agent-chat-input');
    await input.fill(`send 1 XLM to ${FIXTURE_ADDRESS}`);
    await page.getByTestId('agent-chat-submit').click();

    await expect(page.getByTestId('intent-preview-card')).toBeVisible();
    await page.getByTestId('cancel-btn').click();

    // Allow one microtask cycle for any potential requests to fire
    await page.waitForTimeout(200);

    expect(prepareRequests).toHaveLength(0);
  });
});
