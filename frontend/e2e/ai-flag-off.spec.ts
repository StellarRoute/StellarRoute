import { test, expect } from '@playwright/test';

test.describe('AI agent flag-off baseline (AI-38)', () => {
  test.describe.configure({ mode: 'serial' });

  test('nav has no Agent link when flag is unset', async ({ page }) => {
    await page.goto('/');

    const nav = page.locator('header').getByRole('navigation', { name: 'Main navigation' });
    await expect(nav.getByRole('link', { name: /agent/i })).toHaveCount(0);
  });

  test('/swap primary control is present with flag unset', async ({ page }) => {
    await page.goto('/swap');

    await expect(page.getByTestId('swap-card')).toBeVisible();
  });

  test('/ai shows disabled state when flag is unset', async ({ page }) => {
    await page.goto('/ai');

    await expect(page.getByTestId('ai-page-disabled')).toBeVisible();
    await expect(page.getByTestId('agent-chat-container')).toHaveCount(0);
  });

  test('spec does not set AI_AGENT_ENABLED or NEXT_PUBLIC_AI_AGENT to true', () => {
    expect(process.env['AI_AGENT_ENABLED']).not.toBe('true');
    expect(process.env['AI_AGENT_ENABLED']).not.toBe('1');
    expect(process.env['NEXT_PUBLIC_AI_AGENT']).not.toBe('true');
    expect(process.env['NEXT_PUBLIC_AI_AGENT']).not.toBe('1');
  });
});
