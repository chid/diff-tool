/**
 * E2E test: AI chat UI rendering.
 *
 * Navigates to /?test=ai which renders the AiTestPage with provider tabs,
 * configuration form, mock toggle, and response display.  All tests use
 * the staged mock data (no API key required).
 */
import { test, expect } from '@playwright/test';

const AI_URL = '/?test=ai';

test.describe('AI test page UI rendering', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(AI_URL);
    await expect(page.locator('h1')).toContainText('AI Chat Test');
  });

  // ── Provider tabs ────────────────────────────────────────────────────

  test('shows provider tab buttons for all three providers', async ({ page }) => {
    const openaiTab = page.getByTestId('ai-tab-openai');
    const anthropicTab = page.getByTestId('ai-tab-anthropic');
    const customTab = page.getByTestId('ai-tab-custom');
    await expect(openaiTab).toBeVisible();
    await expect(openaiTab).toHaveText('openai');
    await expect(anthropicTab).toBeVisible();
    await expect(anthropicTab).toHaveText('anthropic');
    await expect(customTab).toBeVisible();
    await expect(customTab).toHaveText('custom');
  });

  test('OpenAI tab is active by default', async ({ page }) => {
    // The header should show 'openai' as the provider
    await expect(page.locator('.smoke-header p strong')).toHaveText('openai');
  });

  test('switching to Anthropic tab updates the header', async ({ page }) => {
    await page.getByTestId('ai-tab-anthropic').click();
    await expect(page.locator('.smoke-header p strong')).toHaveText('anthropic');
  });

  test('switching to Custom tab shows base URL input', async ({ page }) => {
    // Base URL input should not be visible for OpenAI
    await expect(page.getByTestId('ai-base-url')).not.toBeVisible();

    // Switch to custom
    await page.getByTestId('ai-tab-custom').click();
    await expect(page.locator('.smoke-header p strong')).toHaveText('custom');
    // Base URL input should now be visible
    await expect(page.getByTestId('ai-base-url')).toBeVisible();
  });

  // ── Form fields ──────────────────────────────────────────────────────

  test('renders all form fields with data-testid attributes', async ({ page }) => {
    await expect(page.getByTestId('ai-use-mock')).toBeVisible();
    await expect(page.getByTestId('ai-api-key')).toBeVisible();
    await expect(page.getByTestId('ai-model')).toBeVisible();
    await expect(page.getByTestId('ai-temperature')).toBeVisible();
    await expect(page.getByTestId('ai-max-tokens')).toBeVisible();
    await expect(page.getByTestId('ai-system-prompt')).toBeVisible();
    await expect(page.getByTestId('ai-user-prompt')).toBeVisible();
    await expect(page.getByTestId('ai-submit')).toBeVisible();
  });

  test('API key input is type=password', async ({ page }) => {
    const apiKeyInput = page.getByTestId('ai-api-key');
    await expect(apiKeyInput).toHaveAttribute('type', 'password');
  });

  test('mock toggle is checked by default', async ({ page }) => {
    const mockToggle = page.getByTestId('ai-use-mock');
    await expect(mockToggle).toBeChecked();
  });

  test('model input shows provider-specific placeholder', async ({ page }) => {
    const modelInput = page.getByTestId('ai-model');
    // OpenAI default placeholder
    await expect(modelInput).toHaveAttribute('placeholder', 'gpt-4o-mini');

    // Switch to Anthropic
    await page.getByTestId('ai-tab-anthropic').click();
    await expect(modelInput).toHaveAttribute('placeholder', 'claude-sonnet-4-20250514');
  });

  // ── Submit (mock) ────────────────────────────────────────────────────

  test('submitting with mock data shows loading then response', async ({ page }) => {
    // Click submit
    await page.getByTestId('ai-submit').click();

    // Loading state should appear
    await expect(page.getByTestId('ai-loading')).toBeVisible();

    // Response should eventually appear (mock delay is 600ms)
    await expect(page.getByTestId('ai-response')).toBeVisible({ timeout: 5000 });
    await expect(page.getByTestId('ai-loading')).not.toBeVisible();
  });

  test('mock response shows token usage badges with correct values', async ({ page }) => {
    await page.getByTestId('ai-submit').click();
    await expect(page.getByTestId('ai-response')).toBeVisible({ timeout: 5000 });

    // Token usage badges
    await expect(page.getByTestId('ai-usage')).toBeVisible();
    await expect(page.getByTestId('ai-prompt-tokens')).toHaveText('Prompt: 42');
    await expect(page.getByTestId('ai-completion-tokens')).toHaveText('Completion: 48');
    await expect(page.getByTestId('ai-total-tokens')).toHaveText('Total: 90');
  });

  test('mock response content is displayed correctly', async ({ page }) => {
    await page.getByTestId('ai-submit').click();
    await expect(page.getByTestId('ai-response')).toBeVisible({ timeout: 5000 });

    const content = page.getByTestId('ai-content');
    await expect(content).toContainText('staged AI response');
    await expect(content).toContainText('token-usage badges');
  });

  // ── Empty state ──────────────────────────────────────────────────────

  test('shows empty state message before first submit', async ({ page }) => {
    await expect(page.getByTestId('ai-empty')).toBeVisible();
    await expect(page.getByTestId('ai-empty')).toContainText('Enter a prompt');
  });

  // ── No console errors ───────────────────────────────────────────────

  test('has no console errors during AI UI rendering', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    // Exercise tabs
    await page.getByTestId('ai-tab-anthropic').click();
    await page.getByTestId('ai-tab-custom').click();
    await page.getByTestId('ai-tab-openai').click();

    // Submit mock
    await page.getByTestId('ai-submit').click();
    await expect(page.getByTestId('ai-response')).toBeVisible({ timeout: 5000 });

    const relevantErrors = errors.filter(
      (e) => !e.includes('404') && !e.includes('Failed to load'),
    );
    expect(relevantErrors).toEqual([]);
  });
});
