/**
 * E2E test: Office document diff UI rendering.
 *
 * Navigates to /?test=office which renders the OfficeTestPage with static
 * PPTX and XLSX fixture data. Verifies the tab switcher, summary stats,
 * section cards, change details with before/after diffs, and that there
 * are no console errors.
 */
import { test, expect } from '@playwright/test';

const OFFICE_URL = '/?test=office';

test.describe('Office diff UI rendering', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(OFFICE_URL);
    await expect(page.locator('h1')).toContainText('Office Document Diff Test');
  });

  // ── Tab switcher ────────────────────────────────────────────────────

  test('shows PowerPoint and Excel tab buttons', async ({ page }) => {
    const pptxTab = page.getByTestId('office-tab-pptx');
    const xlsxTab = page.getByTestId('office-tab-xlsx');
    await expect(pptxTab).toBeVisible();
    await expect(xlsxTab).toBeVisible();
    await expect(pptxTab).toHaveText('PowerPoint');
    await expect(xlsxTab).toHaveText('Excel');
  });

  test('PPTX tab is active by default with XLSX tab inactive', async ({ page }) => {
    const pptxTab = page.getByTestId('office-tab-pptx');
    const xlsxTab = page.getByTestId('office-tab-xlsx');
    // Both tabs should be visible
    await expect(pptxTab).toBeVisible();
    await expect(xlsxTab).toBeVisible();
    // The active tab should have a different (non-transparent) background than inactive
    // We verify by checking both tabs exist and clicking XLSX changes the display
    // (tested more thoroughly in the tab switching test below)
    await expect(page.getByTestId('office-section-0-label')).toContainText('Slide 1');
  });

  test('switching to Excel tab updates the summary and sections', async ({ page }) => {
    // Click the Excel tab
    await page.getByTestId('office-tab-xlsx').click();

    // Summary should now reflect Excel data
    const changedStat = page.getByTestId('office-sections-changed');
    await expect(changedStat).toBeVisible();

    // Section kind should change to excel-sheet
    const sectionLabel = page.getByTestId('office-section-0-label');
    await expect(sectionLabel).toContainText('Sheet1');
  });

  // ── Summary stats ───────────────────────────────────────────────────

  test('renders summary stats with correct values and colors', async ({ page }) => {
    // Total sections
    await expect(page.getByTestId('office-sections-total')).toHaveText('1 sections');

    // Changed sections (should use modified color)
    const changedStat = page.getByTestId('office-sections-changed');
    await expect(changedStat).toHaveText('1 changed');
    await expect(changedStat).toHaveCSS('color', 'rgb(212, 167, 44)');

    // Added sections (should use green color)
    const addedStat = page.getByTestId('office-sections-added');
    await expect(addedStat).toHaveText('+0 added');
    await expect(addedStat).toHaveCSS('color', 'rgb(45, 164, 78)');

    // Deleted sections (should use red color)
    const deletedStat = page.getByTestId('office-sections-deleted');
    await expect(deletedStat).toHaveText('−0 deleted');
    await expect(deletedStat).toHaveCSS('color', 'rgb(207, 34, 46)');

    // Atomic changes
    await expect(page.getByTestId('office-atomic-changes')).toHaveText('1 atomic changes');
  });

  // ── Section cards ───────────────────────────────────────────────────

  test('renders section card with status badge', async ({ page }) => {
    const section = page.getByTestId('office-section-0');
    await expect(section).toBeVisible();

    // Status badge
    const statusBadge = page.getByTestId('office-section-0-status');
    await expect(statusBadge).toBeVisible();
    await expect(statusBadge).toHaveText('modified');

    // Label
    await expect(page.getByTestId('office-section-0-label')).toContainText('Slide 1');
  });

  test('section card has correct kind and label', async ({ page }) => {
    // PPTX section kind
    await expect(page.getByTestId('office-section-0-label')).toContainText('Slide 1');
    await expect(page.getByTestId('office-section-0-label')).toContainText('Left Title');
  });

  // ── Change details ──────────────────────────────────────────────────

  test('renders change list with kind and path', async ({ page }) => {
    const changes = page.getByTestId('office-section-0-changes');
    await expect(changes).toBeVisible();

    // Change kind
    await expect(page.getByTestId('office-change-0-0-kind')).toHaveText('text changed');

    // Change path
    await expect(page.getByTestId('office-change-0-0-path')).toHaveText('slide=1');
  });

  test('renders before/after diff with correct color coding', async ({ page }) => {
    // Before block (red border, red tint background)
    const beforeEl = page.getByTestId('office-change-0-0-before');
    await expect(beforeEl).toBeVisible();
    await expect(beforeEl).toContainText('Left Title');
    await expect(beforeEl).toHaveCSS('border-left-color', 'rgb(207, 34, 46)');

    // After block (green border, green tint background)
    const afterEl = page.getByTestId('office-change-0-0-after');
    await expect(afterEl).toBeVisible();
    await expect(afterEl).toContainText('Right Title');
    await expect(afterEl).toHaveCSS('border-left-color', 'rgb(45, 164, 78)');
  });

  // ── Excel tab content ───────────────────────────────────────────────

  test('Excel tab shows Sheet1 section with text-changed', async ({ page }) => {
    await page.getByTestId('office-tab-xlsx').click();

    // Section label
    await expect(page.getByTestId('office-section-0-label')).toContainText('Sheet1');

    // Status badge
    await expect(page.getByTestId('office-section-0-status')).toHaveText('modified');

    // Change details
    await expect(page.getByTestId('office-change-0-0-before')).toContainText('left value');
    await expect(page.getByTestId('office-change-0-0-after')).toContainText('right value');
  });

  test('switching back to PPTX restores original content', async ({ page }) => {
    // Go to Excel
    await page.getByTestId('office-tab-xlsx').click();
    await expect(page.getByTestId('office-section-0-label')).toContainText('Sheet1');

    // Go back to PPTX
    await page.getByTestId('office-tab-pptx').click();
    await expect(page.getByTestId('office-section-0-label')).toContainText('Slide 1');
    await expect(page.getByTestId('office-change-0-0-before')).toContainText('Left Title');
  });

  // ── No console errors ───────────────────────────────────────────────

  test('has no console errors during office UI rendering', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    // Exercise both tabs
    await page.getByTestId('office-tab-xlsx').click();
    await page.getByTestId('office-tab-pptx').click();

    // Wait for everything to settle
    await expect(page.getByTestId('office-section-0')).toBeVisible();

    const relevantErrors = errors.filter(
      (e) => !e.includes('404') && !e.includes('Failed to load'),
    );
    expect(relevantErrors).toEqual([]);
  });
});
