import { expect, test } from '@playwright/test';
import { openDemo } from './document_demo_helpers';

test('the reading room shows the two table moves while its text stays the same', async ({ page }) => {
  await openDemo(page, 'Reading room floor plan');
  const visualMode = page.getByLabel('Compare rendered PDF pages');
  await expect(visualMode).toBeChecked();
  await expect(page.getByRole('heading', { name: 'Page 1 · Changed' })).toBeVisible();
  const overlay = page.getByRole('img', { name: 'Page 1 visual comparison' });
  const highlighted = await overlay.evaluate(async (img: HTMLImageElement) => {
    await img.decode();
    const canvas = document.createElement('canvas');
    canvas.width = img.naturalWidth;
    canvas.height = img.naturalHeight;
    const ctx = canvas.getContext('2d')!;
    ctx.drawImage(img, 0, 0);
    // Sample the old and new table centres, then the fixed stacks and help desk.
    // Coordinates are PDF points measured from the upper left of the 720 × 480 sheet.
    return [[250, 262], [312, 262], [250, 218], [312, 309], [94, 255], [440, 329]].map(([x, y]) => {
      const [r, g, b] = ctx.getImageData(Math.floor(x * canvas.width / 720), Math.floor(y * canvas.height / 480), 1, 1).data;
      return r > 160 && r > g * 2 && b > g * 2;
    });
  });
  expect(highlighted).toEqual([true, true, true, true, false, false]);
  await visualMode.uncheck();
  await expect(page.getByText('No differences in the compared text')).toBeVisible();
  await page.getByLabel('Include unchanged sections').check();
  await expect(page.locator('.document-prose')).toContainText('12 seats');
});

test('adding and removing a PDF page preserves the first page in visual review', async ({ page }) => {
  await page.setContent(`<style>@page { size: A5; } section + section { break-before: page; }</style>
    <section><h1>Furniture schedule</h1><p>Room A has twelve seats.</p></section>
    <section><h1>Delivery note</h1><p>Bring six more chairs to Room B.</p></section>`);
  const original = { name: 'schedule.pdf', mimeType: 'application/pdf',
    buffer: await page.pdf({ preferCSSPageSize: true, pageRanges: '1' }) };
  const expanded = { name: 'schedule-and-note.pdf', mimeType: 'application/pdf',
    buffer: await page.pdf({ preferCSSPageSize: true, pageRanges: '1-2' }) };

  await page.goto('./?tab=documents');
  const left = page.getByLabel('Choose original file');
  const right = page.getByLabel('Choose revised file');
  await left.setInputFiles(original);
  await right.setInputFiles(expanded);
  await expect(page.locator('.document-prose ins')).toContainText('Bring six more chairs to Room B.');
  const visualMode = page.getByLabel('Compare rendered PDF pages');
  await visualMode.check();
  await expect(page.getByRole('heading', { name: 'Page 2 · Added' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Page 1 · Unchanged' })).toBeVisible();
  await expect(page.getByRole('img', { name: 'Page 2 visual comparison' })).toBeVisible();

  await left.setInputFiles(expanded);
  await right.setInputFiles(original);
  await expect(page.getByRole('heading', { name: 'Page 2 · Removed' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Page 1 · Unchanged' })).toBeVisible();
  await expect(page.getByRole('img', { name: 'Page 2 visual comparison' })).toBeVisible();
  await visualMode.uncheck();
  await expect(page.locator('.document-prose del')).toContainText('Bring six more chairs to Room B.');
});
