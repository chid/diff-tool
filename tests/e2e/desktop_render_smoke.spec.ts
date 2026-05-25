/**
 * E2E Scenario 5 — Desktop render smoke test using a static DiffResult.
 *
 * Navigates to /?test=smoke which renders NativeDiffRenderer with the
 * text-basic DiffResult baked into App.tsx. Asserts the rendered diff
 * matches the expected output from the CLI fixture.
 */
import { test, expect } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const SMOKE_URL = '/?test=smoke';

const HUNK_HEADER = '.hunk-header';
const DIFF_LINE = '.diff-line-split';
const DIFF_GRID = '.diff-grid';
const INTRA_HIGHLIGHT = '.intra-highlight';
const FILE_HEADER = '.file-header';

test.describe('Desktop render smoke test — static DiffResult', () => {
  test.beforeEach(async ({ page }) => {
    // Ensure screenshot directory exists
    try { mkdirSync('target/e2e', { recursive: true }); } catch { /* exists */ }

    await page.goto(SMOKE_URL);
    await expect(page.locator('h1')).toContainText('Desktop Render Smoke Test');
  });

  // ── Page structure ─────────────────────────────────────────────────

  test('renders the smoke test page with header and stats', async ({ page }) => {
    await expect(page.locator('h1')).toHaveText('Desktop Render Smoke Test');
    await expect(page.locator('.smoke-header p')).toContainText('examples/text-basic');

    await expect(page.getByTestId('smoke-additions')).toHaveText('+2');
    await expect(page.getByTestId('smoke-deletions')).toHaveText('−1');
    await expect(page.getByTestId('smoke-unchanged')).toContainText('2 unchanged');
  });

  // ── Hunk header ────────────────────────────────────────────────────

  test('shows the correct hunk header', async ({ page }) => {
    const hunkHeader = page.locator(HUNK_HEADER);
    await expect(hunkHeader).toHaveCount(1);
    await expect(hunkHeader).toContainText(/@@ -1,3 \+1,4 @@/);
  });

  // ── File headers ───────────────────────────────────────────────────

  test('shows file headers with left and right paths', async ({ page }) => {
    // Split view renders two .file-header elements (left + right columns)
    const headers = page.locator(FILE_HEADER);
    await expect(headers).toHaveCount(2);
    await expect(headers.first()).toContainText('examples/text-basic/left.txt');
    await expect(headers.nth(1)).toContainText('examples/text-basic/right.txt');
  });

  // ── Line kinds and content ─────────────────────────────────────────

  test('renders all five lines with correct kinds and text', async ({ page }) => {
    const lines = page.locator(DIFF_LINE);
    await expect(lines).toHaveCount(5);

    // Line 1 — context
    const line1 = lines.nth(0);
    await expect(line1).toHaveClass(/context/);
    await expect(line1).toContainText('alpha');

    // Line 2 — deletion
    const line2 = lines.nth(1);
    await expect(line2).toHaveClass(/deletion/);
    await expect(line2).toContainText('welcome');

    // Line 3 — addition
    const line3 = lines.nth(2);
    await expect(line3).toHaveClass(/addition/);
    await expect(line3).toContainText('hello');

    // Line 4 — context
    const line4 = lines.nth(3);
    await expect(line4).toHaveClass(/context/);
    await expect(line4).toContainText('gamma');

    // Line 5 — addition
    const line5 = lines.nth(4);
    await expect(line5).toHaveClass(/addition/);
    await expect(line5).toContainText('delta');
  });

  // ── Line numbers ───────────────────────────────────────────────────

  test('shows correct line numbers for each line', async ({ page }) => {
    const lines = page.locator(DIFF_LINE);

    // Line 1 (context): both left and right show line 1
    const line1Nos = lines.nth(0).locator('.line-no');
    await expect(line1Nos.nth(0)).toHaveText('1');
    await expect(line1Nos.nth(1)).toHaveText('1');

    // Line 2 (deletion): left shows 2, right is empty
    const line2Nos = lines.nth(1).locator('.line-no');
    await expect(line2Nos.nth(0)).toHaveText('2');
    await expect(line2Nos.nth(1)).toHaveText('');

    // Line 3 (addition): left is empty, right shows 2
    const line3Nos = lines.nth(2).locator('.line-no');
    await expect(line3Nos.nth(0)).toHaveText('');
    await expect(line3Nos.nth(1)).toHaveText('2');

    // Line 4 (context): both show 3
    const line4Nos = lines.nth(3).locator('.line-no');
    await expect(line4Nos.nth(0)).toHaveText('3');
    await expect(line4Nos.nth(1)).toHaveText('3');

    // Line 5 (addition): left is empty, right shows 4
    const line5Nos = lines.nth(4).locator('.line-no');
    await expect(line5Nos.nth(0)).toHaveText('');
    await expect(line5Nos.nth(1)).toHaveText('4');
  });

  // ── Intraline highlights ───────────────────────────────────────────

  test('renders intraline highlights on welcome → hello change', async ({ page }) => {
    const highlights = page.locator(INTRA_HIGHLIGHT);
    await expect(highlights).toHaveCount(2);

    // Deletion "welcome" → highlight on whole word (bytes 0–7)
    // Addition "hello" → highlight on whole word (bytes 0–5)
    const texts = await highlights.allTextContents();
    expect(texts).toContain('welcome');
    expect(texts).toContain('hello');
  });

  // ── Screenshot ─────────────────────────────────────────────────────

  test('takes a screenshot of the rendered diff', async ({ page }) => {
    await expect(page.locator(DIFF_GRID)).toBeVisible();

    await page.locator('.diff-pane').screenshot({
      path: 'target/e2e/smoke-render-screenshot.png',
    });

    await page.screenshot({
      path: 'target/e2e/smoke-fullpage-screenshot.png',
      fullPage: true,
    });
  });

  // ── No console errors ──────────────────────────────────────────────

  test('has no console errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    await expect(page.locator(DIFF_GRID)).toBeVisible();
    await expect(page.locator(HUNK_HEADER)).toBeVisible();
    await expect(page.locator(DIFF_LINE)).toHaveCount(5);

    const relevantErrors = errors.filter(
      (e) => !e.includes('404') && !e.includes('Failed to load'),
    );
    expect(relevantErrors).toEqual([]);
  });
});
