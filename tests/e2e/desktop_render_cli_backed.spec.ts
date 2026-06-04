/**
 * E2E Scenario 6 — Desktop render test backed by CLI output.
 *
 * Runs `dt diff examples/text-basic/{left,right}.txt --json`, feeds the
 * resulting JSON to the desktop renderer via `/?test=cli-smoke`, and
 * asserts the rendered diff matches the CLI output exactly.
 */
import { test, expect } from '@playwright/test';
import { execSync } from 'node:child_process';

const SMOKE_URL = '/?test=cli-smoke';

const HUNK_HEADER = '.hunk-header';
const DIFF_LINE = '.diff-line-split';
const DIFF_GRID = '.diff-grid';
const INTRA_HIGHLIGHT = '.intra-highlight';
const FILE_HEADER = '.file-header';

/** Run `dt diff` on the text-basic fixture and return parsed JSON.
 *  Handles exit code 1 (which the CLI returns when diffs are found). */
function runCliDiff() {
  try {
    return JSON.parse(
      execSync(
        'cargo run -p diff-cli -- diff examples/text-basic/left.txt examples/text-basic/right.txt --json 2>/dev/null',
        { encoding: 'utf-8', timeout: 120_000 },
      ),
    );
  } catch (err: any) {
    // execSync throws on non-zero exit. diff-cli exits 1 when diffs exist,
    // but stdout is still valid JSON. Extract it from the error object.
    if (err.stdout && typeof err.stdout === 'string' && err.stdout.trim()) {
      return JSON.parse(err.stdout);
    }
    throw new Error(
      `cargo run -p diff-cli failed: ${err.message ?? err}`,
    );
  }
}

test.describe('Desktop render smoke test — CLI-backed DiffResult', () => {
  /** CLI output written before any tests and read by the smoke page. */
  let cliDiff: Record<string, unknown>;

  test.beforeAll(() => {
    cliDiff = runCliDiff();
  });

  test.beforeEach(async ({ page }) => {
    await page.route('**/cli-fixture.json', async (route) => {
      await route.fulfill({
        contentType: 'application/json',
        body: JSON.stringify(cliDiff),
      });
    });
    await page.goto(SMOKE_URL);
    // Wait for the diff grid to be visible (handles both loading and immediate-load cases).
    await expect(page.locator(DIFF_GRID)).toBeVisible({ timeout: 15_000 });
    await expect(page.locator('h1')).toContainText('CLI-Backed Render Smoke Test');
  });

  // ── Page structure ─────────────────────────────────────────────────

  test('renders the smoke test page with header and stats', async ({ page }) => {
    await expect(page.locator('h1')).toHaveText('CLI-Backed Render Smoke Test');
    await expect(page.locator('.smoke-header p')).toContainText('examples/text-basic');

    await expect(page.getByTestId('cli-smoke-additions')).toHaveText('+2');
    await expect(page.getByTestId('cli-smoke-deletions')).toHaveText('−1');
    await expect(page.getByTestId('cli-smoke-unchanged')).toContainText('2 unchanged');
  });

  // ── Hunk header ────────────────────────────────────────────────────

  test('shows the correct hunk header', async ({ page }) => {
    const hunkHeader = page.locator(HUNK_HEADER);
    await expect(hunkHeader).toHaveCount(1);
    await expect(hunkHeader).toContainText(/@@ -1,3 \+1,4 @@/);
  });

  // ── File headers ───────────────────────────────────────────────────

  test('shows file headers with left and right paths', async ({ page }) => {
    const headers = page.locator(FILE_HEADER);
    await expect(headers).toHaveCount(2);
    await expect(headers.first()).toContainText('examples/text-basic/left.txt');
    await expect(headers.nth(1)).toContainText('examples/text-basic/right.txt');
  });

  // ── Line kinds and content ─────────────────────────────────────────

  test('renders all five lines with correct kinds and text from CLI output', async ({ page }) => {
    const lines = page.locator(DIFF_LINE);
    await expect(lines).toHaveCount(5);

    // Line 1 — context: alpha
    const line1 = lines.nth(0);
    await expect(line1).toHaveClass(/context/);
    await expect(line1).toContainText('alpha');

    // Line 2 — deletion: welcome
    const line2 = lines.nth(1);
    await expect(line2).toHaveClass(/deletion/);
    await expect(line2).toContainText('welcome');

    // Line 3 — addition: hello
    const line3 = lines.nth(2);
    await expect(line3).toHaveClass(/addition/);
    await expect(line3).toContainText('hello');

    // Line 4 — context: gamma
    const line4 = lines.nth(3);
    await expect(line4).toHaveClass(/context/);
    await expect(line4).toContainText('gamma');

    // Line 5 — addition: delta
    const line5 = lines.nth(4);
    await expect(line5).toHaveClass(/addition/);
    await expect(line5).toContainText('delta');
  });

  // ── Line numbers ───────────────────────────────────────────────────

  test('shows correct line numbers for each line', async ({ page }) => {
    const lines = page.locator(DIFF_LINE);

    const line1Nos = lines.nth(0).locator('.line-no');
    await expect(line1Nos.nth(0)).toHaveText('1');
    await expect(line1Nos.nth(1)).toHaveText('1');

    const line2Nos = lines.nth(1).locator('.line-no');
    await expect(line2Nos.nth(0)).toHaveText('2');
    await expect(line2Nos.nth(1)).toHaveText('');

    const line3Nos = lines.nth(2).locator('.line-no');
    await expect(line3Nos.nth(0)).toHaveText('');
    await expect(line3Nos.nth(1)).toHaveText('2');

    const line4Nos = lines.nth(3).locator('.line-no');
    await expect(line4Nos.nth(0)).toHaveText('3');
    await expect(line4Nos.nth(1)).toHaveText('3');

    const line5Nos = lines.nth(4).locator('.line-no');
    await expect(line5Nos.nth(0)).toHaveText('');
    await expect(line5Nos.nth(1)).toHaveText('4');
  });

  // ── Intraline highlights ───────────────────────────────────────────

  test('renders intraline highlights from CLI-produced intra_spans', async ({ page }) => {
    const highlights = page.locator(INTRA_HIGHLIGHT);
    await expect(highlights).toHaveCount(2);

    // welcome (deletion, 7 bytes) and hello (addition, 5 bytes) —
    // the words share no common prefix/suffix so the entire words are highlighted.
    const texts = await highlights.allTextContents();
    expect(texts).toContain('welcome');
    expect(texts).toContain('hello');
  });

  // ── Verify CLI output matches render (key shape check) ─────────────

  test('the CLI JSON matches the expected test-basic fixture shape', async () => {
    const stats = cliDiff.stats as Record<string, number>;
    expect(stats.additions).toBe(2);
    expect(stats.deletions).toBe(1);
    expect(stats.unchanged).toBe(2);
    expect(cliDiff.truncated).toBe(false);
    expect(Array.isArray(cliDiff.files)).toBe(true);
    expect((cliDiff.files as unknown[]).length).toBeGreaterThanOrEqual(1);
  });

  // ── Screenshot ─────────────────────────────────────────────────────

  test('takes a screenshot of the CLI-backed rendered diff', async ({ page }) => {
    await expect(page.locator(DIFF_GRID)).toBeVisible();

    await page.locator('.diff-pane').screenshot({
      path: 'target/e2e/cli-smoke-render-screenshot.png',
    });

    await page.screenshot({
      path: 'target/e2e/cli-smoke-fullpage-screenshot.png',
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
    await expect(page.locator(INTRA_HIGHLIGHT)).toHaveCount(2);

    const relevantErrors = errors.filter(
      (e) => !e.includes('404') && !e.includes('Failed to load'),
    );
    expect(relevantErrors).toEqual([]);
  });
});
