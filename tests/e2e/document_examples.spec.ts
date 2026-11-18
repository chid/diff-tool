import { expect, test } from '@playwright/test';
import { join } from 'node:path';
import { openDemo } from './document_demo_helpers';

test('a missing example file shows a retry and opens the correct pair on retry', async ({ page }) => {
  const asset = '**/document-demos/supplier-revised.docx';
  await page.route(asset, route => route.fulfill({ status: 404 }));
  await openDemo(page, 'Supplier agreement');
  await expect(page.getByRole('alert')).toContainText('could not be loaded');
  await expect(page.getByRole('button', { name: 'Download tracked Word report' })).toHaveCount(0);
  await page.unroute(asset);
  await page.getByRole('link', { name: 'Supplier agreement', exact: true }).click();
  await expect(page.locator('.document-prose ins').filter({ hasText: 'Friday' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Download original' })).toHaveAttribute('href', /supplier-original\.docx$/);
});

test('choosing files while an example loads keeps the user-selected comparison', async ({ page }) => {
  let release!: () => void;
  const paused = new Promise<void>(resolve => { release = resolve; });
  const asset = '**/document-demos/supplier-revised.docx';
  await page.route(asset, async route => { await paused; await route.continue(); });
  const pending = page.waitForRequest(asset);
  await page.goto('./?tab=documents&example=supplier');
  await pending;
  try {
    const fixtures = join(process.cwd(), 'fixtures/cases/office-word-basic');
    await page.getByLabel('Choose original file').setInputFiles(join(fixtures, 'left.docx'));
    await page.getByLabel('Choose revised file').setInputFiles(join(fixtures, 'right.docx'));
    await expect(page.locator('.document-prose ins')).toHaveText('right');
  } finally {
    release();
  }
  await page.waitForLoadState('networkidle');
  await expect(page.locator('.document-prose del')).toHaveText('left');
  await expect(page.getByRole('link', { name: 'Download original' })).toHaveCount(0);
});
