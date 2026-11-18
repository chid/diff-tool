import { expect, test, type Page, type TestInfo } from '@playwright/test';
import { copyFile, mkdir } from 'node:fs/promises';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { openDemo } from './document_demo_helpers';

const reportButton = (page: Page) => page.getByRole('button', { name: 'Download review report', exact: true });

async function openSavedReport(page: Page, info: TestInfo, original: string, revised: string,
  expectedDownload = revised.replace(/\.[^.]+$/, '') + '-review-report.html') {
  const pending = page.waitForEvent('download');
  await reportButton(page).click();
  const download = await pending;
  expect(download.suggestedFilename()).toBe(expectedDownload);
  const path = info.outputPath('review.html');
  await download.saveAs(path);

  // Leaving the app releases its object URLs. The saved file must stand on its own.
  await page.goto('about:blank');
  await page.context().setOffline(true);
  await page.goto(pathToFileURL(path).href);
  await page.emulateMedia({ media: 'print' });
  await expect(page.getByRole('heading', { level: 1, name: 'Document comparison report' })).toBeVisible();
  await expect(page.locator('dt').filter({ hasText: /^Original$/i }).locator('+ dd')).toHaveText(original);
  await expect(page.locator('dt').filter({ hasText: /^Revised$/i }).locator('+ dd')).toHaveText(revised);
  await expect(page.locator('script')).toHaveCount(0);
  return path;
}

// Pages publishes only verified demo downloads. Tests for user inputs are excluded.
async function copyDemoReport(path: string, name: 'supplier-report.html' | 'launch-report.html' | 'reading-room-report.html') {
  const destination = process.env.DEMO_REPORT_DIR;
  if (!destination) return;
  await mkdir(destination, { recursive: true });
  await copyFile(path, join(destination, name));
}

function evidenceWith(page: Page, phrase: RegExp) {
  return page.locator('section:has(> h3)').filter({
    has: page.locator('blockquote').filter({ hasText: phrase }),
  });
}

test('the supplier report keeps the old and new delivery and payment terms together', async ({ page }, info) => {
  await openDemo(page, 'Supplier agreement');
  const report = await openSavedReport(page, info, 'supplier-original.docx', 'supplier-revised.docx');
  await expect(page.getByRole('heading', { name: 'Summary' }).locator('+ p')).toContainText(/1 section changed/i);
  const delivery = evidenceWith(page, /Wednesday mornings/);
  await expect(delivery.locator('blockquote').first()).toContainText('Wednesday mornings');
  await expect(delivery.locator('blockquote').last()).toContainText('Friday mornings');
  const payment = evidenceWith(page, /30 days of delivery/);
  await expect(payment.locator('blockquote').first()).toContainText('30 days of delivery');
  await expect(payment.locator('blockquote').last()).toContainText('14 days of delivery');
  const moved = page.locator('section').filter({ has: page.getByRole('heading', { name: /moved paragraph/i }) });
  await expect(moved).toContainText(/paragraph 5 moved to paragraph 8/i);
  await expect(moved.locator('blockquote')).toContainText('Each crate must carry the order number');
  await copyDemoReport(report, 'supplier-report.html');
});

test('the launch report preserves slide and artwork evidence with the order quantities', async ({ page }, info) => {
  await openDemo(page, 'Launch review');
  const report = await openSavedReport(page, info, 'launch-original.pptx', 'launch-revised.pptx');
  await expect(page.getByRole('heading', { name: 'Summary' }).locator('+ p')).toContainText(/3 sections changed/i);
  const order = evidenceWith(page, /240 cases/);
  await expect(order.locator('h3')).toContainText(/slide 1/i);
  await expect(order.locator('blockquote').first()).toContainText('240 cases');
  await expect(order.locator('blockquote').last()).toContainText('360 cases');
  const label = evidenceWith(page, /plain text/);
  await expect(label.locator('h3')).toContainText(/slide 2/i);
  await expect(label.locator('blockquote').last()).toContainText(/coral strip/);
  await expect(page.getByRole('heading', { level: 3, name: /image\.png/i })).toBeVisible();
  await copyDemoReport(report, 'launch-report.html');
});

test('the floor plan report embeds the highlighted page and still prints offline', async ({ page }, info) => {
  await openDemo(page, 'Reading room floor plan');
  await expect(page.getByRole('heading', { name: 'Page 1 · Changed' })).toBeVisible();
  const report = await openSavedReport(page, info, 'reading-room-original.pdf', 'reading-room-revised.pdf');
  await expect(page.getByRole('heading', { name: 'Summary' }).locator('+ p')).toContainText(/1 changed/i);
  await expect(page.getByRole('heading', { name: 'Comparison notes' }).locator('+ ul')).toContainText(/paired by position/i);
  const section = page.locator('section').filter({
    has: page.getByRole('heading', { level: 3, name: /page 1/i }),
  }).last();
  await expect(section).toContainText(/changed/i);
  const image = section.getByRole('img');
  await expect(image).toBeVisible();
  await expect(image).toHaveAttribute('src', /^data:image\/png;base64,/);
  const hasOverlay = await image.evaluate(async (img: HTMLImageElement) => {
    await img.decode();
    const canvas = document.createElement('canvas');
    canvas.width = img.naturalWidth;
    canvas.height = img.naturalHeight;
    const context = canvas.getContext('2d')!;
    context.drawImage(img, 0, 0);
    const { data } = context.getImageData(0, 0, canvas.width, canvas.height);
    for (let i = 0; i < data.length; i += 4) {
      if (data[i] === 209 && data[i + 1] === 28 && data[i + 2] === 101) return true;
    }
    return false;
  });
  expect(hasOverlay).toBe(true);
  await copyDemoReport(report, 'reading-room-report.html');
});

test('filenames and source markup remain literal text when the report is opened', async ({ page }, info) => {
  const original = 'before <b>.txt';
  const revised = 'after & new.txt';
  const before = 'Shelf card: <mark data-demo="word">amber</mark> & cream.';
  const after = 'Shelf card: <mark data-demo="word">coral</mark> & cream.';
  await page.goto('./?tab=documents');
  await page.getByLabel('Choose original file').setInputFiles({ name: original, mimeType: 'text/plain', buffer: Buffer.from(before) });
  await page.getByLabel('Choose revised file').setInputFiles({ name: revised, mimeType: 'text/plain', buffer: Buffer.from(after) });
  await openSavedReport(page, info, original, revised, 'after-new-review-report.html');
  const quotes = page.locator('blockquote');
  await expect(quotes.first()).toContainText(before);
  await expect(quotes.last()).toContainText(after);
  await expect(page.locator('[data-demo="word"], dd b')).toHaveCount(0);
});

test('an identical pair exports an empty change list and a corrupt replacement clears the export action', async ({ page }, info) => {
  const original = join(process.cwd(), 'apps/desktop/public/document-demos/supplier-original.docx');
  await page.goto('./?tab=documents');
  await page.getByLabel('Choose original file').setInputFiles(original);
  await page.getByLabel('Choose revised file').setInputFiles(original);
  await expect(page.getByText('No differences in the compared text')).toBeVisible();
  await openSavedReport(page, info, 'supplier-original.docx', 'supplier-original.docx');
  await expect(page.locator('blockquote')).toHaveCount(0);
  await expect(page.locator('body')).toContainText(/no (?:differences|changes)|0 changed/i);
  await page.context().setOffline(false);
  await page.emulateMedia({ media: 'screen' });
  await openDemo(page, 'Supplier agreement');
  await expect(reportButton(page)).toBeEnabled();
  await page.getByLabel('Choose revised file').setInputFiles({
    name: 'broken.docx', mimeType: 'application/octet-stream', buffer: Buffer.from('unreadable'),
  });
  await expect(page.getByRole('alert')).toBeVisible();
  await expect(reportButton(page)).toHaveCount(0);
});
