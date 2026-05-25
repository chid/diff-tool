import { test, expect } from '@playwright/test';

const HARNESS_URL = '/?test=diff';
const NEXT_BTN = 'button[aria-label="Next test case"]';
const PREV_BTN = 'button[aria-label="Previous test case"]';
const COUNTER = '.harness-counter';
const CASE_HEADING = '.harness-case-info h2';
const RENDER_AREA = '.harness-render-area';
const STATUS_BADGE_LABEL = '#test-label';
const STATUS_BADGE_INDEX = '#test-index';
const INTRA_HIGHLIGHT = '.intra-highlight';
const HUNK_HEADER = '.hunk-header';
const HUNK_SEPARATOR = '.hunk-separator';
const DIFF_STATUS_TEXT = '.diff-status-text';

test.describe('Browser test harness — NativeDiffRenderer E2E', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(HARNESS_URL);
    // Wait until the harness header is visible
    await expect(page.locator('h1')).toContainText('NativeDiffRenderer');
  });

  // ── Case 1: Empty ──────────────────────────────────────────────────

  test('Case 1 — Empty: shows "No changes" and no diff grid', async ({ page }) => {
    // Status bar confirms we're on case 1
    await expect(page.locator(STATUS_BADGE_LABEL)).toHaveText('Empty');
    await expect(page.locator(STATUS_BADGE_INDEX)).toHaveText('1/7');
    await expect(page.locator(CASE_HEADING)).toHaveText('Empty');

    // Empty state
    await expect(page.locator(DIFF_STATUS_TEXT)).toHaveText('No changes');
    // No diff grid rendered
    await expect(page.locator('.diff-grid')).toHaveCount(0);
    // No intraline highlights
    await expect(page.locator(INTRA_HIGHLIGHT)).toHaveCount(0);
  });

  // ── Case 2: Identical ─────────────────────────────────────────────

  test('Case 2 — Identical: shows "Files are identical"', async ({ page }) => {
    await page.click(NEXT_BTN);
    await expect(page.locator(COUNTER)).toContainText('2 / 7');
    await expect(page.locator(CASE_HEADING)).toHaveText('Identical');

    await expect(page.locator(DIFF_STATUS_TEXT)).toHaveText('Files are identical');
    await expect(page.locator(INTRA_HIGHLIGHT)).toHaveCount(0);
  });

  // ── Case 3: ASCII spans ───────────────────────────────────────────

  test('Case 3 — ASCII spans: renders intraline highlights', async ({ page }) => {
    // Navigate to case 3
    await page.click(NEXT_BTN);
    await page.click(NEXT_BTN);
    await expect(page.locator(COUNTER)).toContainText('3 / 7');
    await expect(page.locator(CASE_HEADING)).toHaveText('ASCII spans');

    // At least 2 intra-highlight spans (one for "old", one for "new")
    const highlights = page.locator(INTRA_HIGHLIGHT);
    await expect(highlights).toHaveCount(2);

    // Verify highlight text content covers the changed portions
    const texts = await highlights.allTextContents();
    expect(texts).toContain('old');
    expect(texts).toContain('new');

    // Stats should show 1 addition / 1 deletion
    await expect(page.locator('#stats-additions')).toHaveText('+1');
    await expect(page.locator('#stats-deletions')).toHaveText('−1');
  });

  // ── Case 4: Unicode (café → cafe) ─────────────────────────────────

  test('Case 4 — Unicode: highlights accented character correctly', async ({ page }) => {
    // Navigate to case 4
    await page.click(NEXT_BTN);
    await page.click(NEXT_BTN);
    await page.click(NEXT_BTN);
    await expect(page.locator(COUNTER)).toContainText('4 / 7');
    await expect(page.locator(CASE_HEADING)).toHaveText('Unicode (café → cafe)');

    // The render area should contain the accented é character (not corrupted)
    const renderArea = page.locator(RENDER_AREA);
    await expect(renderArea).toContainText('é');

    // Should have intraline highlights for the changed character
    const highlights = page.locator(INTRA_HIGHLIGHT);
    await expect(highlights).toHaveCount(2); // é in deletion, e in addition

    // One highlight should be the é, the other plain "e"
    const highlightTexts = await highlights.allTextContents();
    expect(highlightTexts).toContain('é');
    expect(highlightTexts).toContain('e');
  });

  // ── Case 5: Emoji surrogate pairs ─────────────────────────────────

  test('Case 5 — Emoji: renders emoji without corruption and highlights them', async ({ page }) => {
    // Navigate to case 5
    for (let i = 0; i < 4; i++) await page.click(NEXT_BTN);
    await expect(page.locator(COUNTER)).toContainText('5 / 7');
    await expect(page.locator(CASE_HEADING)).toHaveText('Emoji surrogate pairs');

    // Emoji should render correctly — no replacement characters (� or empty boxes)
    const renderArea = page.locator(RENDER_AREA);
    const renderedText = await renderArea.textContent();

    // Should contain the emoji (🎉 = U+1F389, ✨ = U+2728)
    // Playwright's innerText should preserve emoji characters
    expect(renderedText).toContain('🎉');
    expect(renderedText).toContain('✨');

    // Intraline highlights should exist for the emoji characters
    const highlights = page.locator(INTRA_HIGHLIGHT);
    await expect(highlights).toHaveCount(2);
  });

  // ── Case 6: Multi-hunk ────────────────────────────────────────────

  test('Case 6 — Multi-hunk: shows hunk headers, separator, and context', async ({ page }) => {
    // Navigate to case 6
    for (let i = 0; i < 5; i++) await page.click(NEXT_BTN);
    await expect(page.locator(COUNTER)).toContainText('6 / 7');
    await expect(page.locator(CASE_HEADING)).toHaveText('Multi-hunk');

    // Hunk headers with @@ range notation
    const hunkHeaders = page.locator(HUNK_HEADER);
    await expect(hunkHeaders).toHaveCount(2);
    await expect(hunkHeaders.first()).toContainText(/@@ -1,3 \+1,3 @@/);

    // Hunk separator (foldable context between hunks)
    await expect(page.locator(HUNK_SEPARATOR)).toHaveCount(1);
    await expect(page.locator(HUNK_SEPARATOR)).toContainText('···');

    // Context, addition, and deletion lines should all be present
    const renderArea = page.locator(RENDER_AREA);
    await expect(renderArea).toContainText('fn main()');
    await expect(renderArea).toContainText('let x = 1');
    await expect(renderArea).toContainText('let x = 42');

    // Stats
    await expect(page.locator('#stats-additions')).toHaveText('+3');
    await expect(page.locator('#stats-deletions')).toHaveText('−2');
  });

  // ── Case 7: Warnings ──────────────────────────────────────────────

  test('Case 7 — Warnings: shows warning badge and identical file message', async ({ page }) => {
    // Navigate to case 7
    for (let i = 0; i < 6; i++) await page.click(NEXT_BTN);
    await expect(page.locator(COUNTER)).toContainText('7 / 7');
    await expect(page.locator(CASE_HEADING)).toHaveText('Warnings');

    // Identical file state (binary placeholder)
    await expect(page.locator(DIFF_STATUS_TEXT)).toHaveText('Files are identical');

    // Warning badge in status bar
    await expect(page.locator('#warning-count')).toBeVisible();
    await expect(page.locator('#warning-count')).toContainText('⚠ 1');
  });

  // ── Navigation wraparound ─────────────────────────────────────────

  test('wraps around from last to first via Next', async ({ page }) => {
    // Navigate to last case
    for (let i = 0; i < 6; i++) await page.click(NEXT_BTN);
    await expect(page.locator(COUNTER)).toContainText('7 / 7');

    // Click Next — should wrap to case 1
    await page.click(NEXT_BTN);
    await expect(page.locator(COUNTER)).toContainText('1 / 7');
    await expect(page.locator(CASE_HEADING)).toHaveText('Empty');
  });

  test('wraps around from first to last via Prev', async ({ page }) => {
    await expect(page.locator(COUNTER)).toContainText('1 / 7');

    // Click Prev — should wrap to case 7
    await page.click(PREV_BTN);
    await expect(page.locator(COUNTER)).toContainText('7 / 7');
    await expect(page.locator(CASE_HEADING)).toHaveText('Warnings');
  });

  // ── No console errors ─────────────────────────────────────────────

  test('has no console errors across all 7 cases', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    // Cycle through all cases, pausing on each one
    for (let i = 0; i < 7; i++) {
      if (i > 0) await page.click(NEXT_BTN);
      // Let the page settle after React re-render
      await expect(page.locator(STATUS_BADGE_INDEX)).toContainText(`${i + 1}/7`);
      // Confirm the case heading matches
      await expect(page.locator(CASE_HEADING)).not.toBeEmpty();
    }



    // Report any page errors (excluding 404s from missing resources unrelated to harness)
    const relevantErrors = errors.filter((e) => !e.includes('404') && !e.includes('Failed to load'));
    expect(relevantErrors).toEqual([]);
  });
});
