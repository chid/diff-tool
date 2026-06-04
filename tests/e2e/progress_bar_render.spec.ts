/**
 * E2E test: Progress bar UI rendering during diff-progress events.
 *
 * Navigates to /?test=progress which simulates the Tauri diff-progress event
 * lifecycle: reading ×2 → normalizing → computing_diff →
 * computing_intraline_spans → building_hunks → loaded (NativeDiffRenderer).
 *
 * Verifies the loader, progress track, fill bar, stage labels, file detail,
 * aria attributes, and CSS transitions.
 */
import { test, expect } from '@playwright/test';

const PROGRESS_URL = '/?test=progress';

const LOADER_ICON = '.diff-loader-icon';
const PROGRESS_TRACK = '.diff-progress-track';
const PROGRESS_FILL = '.diff-progress-fill';
const STATUS_TEXT = '.diff-status-text';
const STATUS_DETAIL = '.diff-status-detail';
const STATUS_HINT = '.diff-status-hint';
const PROGRESS_PHASE = '[data-testid="progress-phase"]';

test.describe('Progress bar UI rendering', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(PROGRESS_URL);
    await expect(page.locator('h1')).toContainText('Progress Bar Simulation Test');
  });

  // ── Initial state ───────────────────────────────────────────────────

  test('shows loader icon on initial load', async ({ page }) => {
    // The loader should be visible immediately
    const loader = page.locator(LOADER_ICON);
    await expect(loader).toBeVisible();
    // It should be an SVG icon from lucide-react
    await expect(loader).toHaveAttribute('class', /diff-loader-icon/);
  });

  test('shows progress bar track with correct ARIA attributes', async ({ page }) => {
    const track = page.locator(PROGRESS_TRACK);
    await expect(track).toBeVisible();
    await expect(track).toHaveAttribute('role', 'progressbar');
    await expect(track).toHaveAttribute('aria-valuemin', '0');
    await expect(track).toHaveAttribute('aria-valuemax', '100');
  });

  test('progress bar fill has gradient background and transition', async ({ page }) => {
    const fill = page.locator(PROGRESS_FILL);
    await expect(fill).toBeVisible();
    // Verify the gradient background
    await expect(fill).toHaveCSS('background', /linear-gradient/);
    // Verify width transition is set
    await expect(fill).toHaveCSS('transition-property', /width/);
  });

  // ── Stage transitions ───────────────────────────────────────────────

  test('transitions through all six progress stages', async ({ page }) => {
    const phaseEl = page.locator(PROGRESS_PHASE);
    const detail = page.locator(STATUS_DETAIL);
    const statusText = page.locator(STATUS_TEXT);
    const hint = page.locator(STATUS_HINT);

    // Stage 0: reading left.txt (sync on detail text, not phase text)
    await expect(detail).toContainText('left.txt', { timeout: 2000 });
    await expect(phaseEl).toContainText('reading');
    await expect(statusText).toContainText('Reading files…');
    await expect(hint).toHaveText(/17\s*B/);

    // Stage 1: reading right.txt (sync on detail change)
    await expect(detail).toContainText('right.txt', { timeout: 2000 });
    await expect(phaseEl).toContainText('reading');
    await expect(hint).toHaveText(/22\s*B/);

    // Stage 2: normalizing
    await expect(phaseEl).toContainText('normalizing', { timeout: 2000 });
    await expect(statusText).toContainText('Normalizing…');

    // Stage 3: computing_diff
    await expect(phaseEl).toContainText('computing_diff', { timeout: 2000 });
    await expect(statusText).toContainText('Computing diff…');

    // Stage 4: computing_intraline_spans
    await expect(phaseEl).toContainText('computing_intraline_spans', { timeout: 2000 });
    await expect(statusText).toContainText('Computing intraline spans…');

    // Stage 5: building_hunks
    await expect(phaseEl).toContainText('building_hunks', { timeout: 2000 });
    await expect(statusText).toContainText('Building hunks…');
  });

  // ── Progress bar width increases through stages ─────────────────────

  test('progress bar fill width increases monotonically through stages', async ({ page }) => {
    const fill = page.locator(PROGRESS_FILL);
    const phaseEl = page.locator(PROGRESS_PHASE);
    let lastWidth = 0;

    // Use one entry per distinct progress percentage (both reading stages map to 10%)
    const stages = [
      'reading',
      'normalizing',
      'computing_diff',
      'computing_intraline_spans',
      'building_hunks',
    ];

    for (const stage of stages) {
      // Wait for the stage to appear
      await expect(phaseEl).toContainText(stage, { timeout: 2000 });

      // Read the current fill width
      const widthPx = await fill.evaluate((el) => {
        return parseFloat(getComputedStyle(el).width);
      });

      // Width should be > previous stage (strictly increasing since each stage has a different PCT)
      expect(widthPx).toBeGreaterThan(lastWidth);
      lastWidth = widthPx;
    }

    // Final stage should have significant width (> 50% of track)
    await expect(phaseEl).toContainText('building_hunks', { timeout: 2000 });
    const finalWidth = await fill.evaluate((el) => parseFloat(getComputedStyle(el).width));
    const trackWidth = await page.locator(PROGRESS_TRACK).evaluate((el) =>
      parseFloat(getComputedStyle(el).width),
    );
    expect(finalWidth).toBeGreaterThan(trackWidth * 0.5);
  });

  // ── ARIA valuenow updates with stages ───────────────────────────────

  test('aria-valuenow increases with each progress stage', async ({ page }) => {
    const track = page.locator(PROGRESS_TRACK);
    const phaseEl = page.locator(PROGRESS_PHASE);

    await expect(phaseEl).toContainText('reading', { timeout: 2000 });
    let lastVal = parseInt(await track.getAttribute('aria-valuenow') ?? '0', 10);
    expect(lastVal).toBeGreaterThan(0);

    const stages = ['normalizing', 'computing_diff', 'computing_intraline_spans', 'building_hunks'];
    for (const stage of stages) {
      await expect(phaseEl).toContainText(stage, { timeout: 2000 });
      const val = parseInt(await track.getAttribute('aria-valuenow') ?? '0', 10);
      expect(val).toBeGreaterThanOrEqual(lastVal);
      lastVal = val;
    }

    // Final value should be 95 (building_hunks)
    expect(lastVal).toBe(95);
  });

  // ── Transitions to loaded state ─────────────────────────────────────

  test('transitions to loaded state showing NativeDiffRenderer', async ({ page }) => {
    const phaseEl = page.locator(PROGRESS_PHASE);

    // Wait for final stage then transition to loaded
    await expect(phaseEl).toContainText('building_hunks', { timeout: 5000 });
    await expect(phaseEl).toContainText('Done', { timeout: 3000 });

    // After loading, the diff grid should be visible with the sample data
    const diffGrid = page.locator('.diff-grid');
    await expect(diffGrid).toBeVisible({ timeout: 2000 });

    // Stats should be visible
    await expect(page.getByTestId('progress-additions')).toHaveText('+2');
    await expect(page.getByTestId('progress-deletions')).toHaveText('−1');

    // The progress bar should be gone
    await expect(page.locator(PROGRESS_TRACK)).toHaveCount(0);
  });

  // ── No console errors ───────────────────────────────────────────────

  test('has no console errors during the full progress lifecycle', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    // Wait for the full lifecycle to complete
    const phaseEl = page.locator(PROGRESS_PHASE);
    await expect(phaseEl).toContainText('Done', { timeout: 10000 });

    // Verify the diff grid rendered
    await expect(page.locator('.diff-grid')).toBeVisible();

    // No JS errors
    const relevantErrors = errors.filter(
      (e) => !e.includes('404') && !e.includes('Failed to load'),
    );
    expect(relevantErrors).toEqual([]);
  });
});
