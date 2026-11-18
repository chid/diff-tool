import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { strFromU8, strToU8, unzipSync, zipSync } from 'fflate';
import { openDemo } from './document_demo_helpers';

const original = join(process.cwd(), 'apps/desktop/public/document-demos/launch-original.pptx');

test('launch review separates the retailer order, label artwork and unchanged delivery checklist', async ({ page }) => {
  await openDemo(page, 'Launch review');
  const review = page.getByRole('article', { name: 'Document comparison' });
  const section = (name: string) => review.locator('section').filter({
    has: page.getByRole('heading', { name, exact: true }),
  });

  const order = section('Slide 1 modified');
  await expect(order.locator('del')).toHaveText(['12', '240']);
  await expect(order.locator('ins')).toHaveText(['18', '360']);

  const packaging = section('Slide 2 modified');
  await expect(packaging.locator('del')).toContainText(['plain text']);
  await expect(packaging.locator('ins')).toContainText(['a coral strip']);
  const image = section('ppt/media/image.png modified');
  await expect(image.locator('del')).toContainText('Original media');
  await expect(image.locator('ins')).toContainText('Revised media');
  await expect(review.getByText('3 changed sections', { exact: true })).toBeVisible();

  await review.getByLabel('Include unchanged sections').check();
  await expect(section('Slide 3 unchanged')).toContainText('Confirm the refrigerated pickup window.');
});

test('PowerPoint uses presentation order when slide filenames and contents stay the same', async ({ page }) => {
  const files = unzipSync(readFileSync(original));
  const part = 'ppt/presentation.xml';
  const xml = strFromU8(files[part]);
  const reordered = await page.evaluate(source => {
    const doc = new DOMParser().parseFromString(source, 'application/xml');
    const list = doc.getElementsByTagNameNS('*', 'sldIdLst')[0];
    list.insertBefore(list.children[1], list.children[0]);
    return new XMLSerializer().serializeToString(doc);
  }, xml);
  files[part] = strToU8(reordered);

  await page.goto('./?tab=documents');
  await page.getByLabel('Choose original file').setInputFiles(original);
  await page.getByLabel('Choose revised file').setInputFiles({
    name: 'launch-reordered.pptx',
    mimeType: 'application/vnd.openxmlformats-officedocument.presentationml.presentation',
    buffer: Buffer.from(zipSync(files)),
  });
  const review = page.getByRole('article', { name: 'Document comparison' });
  await expect(review.getByText('2 changed sections', { exact: true })).toBeVisible();
  const restoredText = (slide: number, exclude: 'ins' | 'del') =>
    review.locator('section').filter({ has: page.getByRole('heading', { name: `Slide ${slide} modified`, exact: true }) })
      .locator(`.document-prose > :not(${exclude})`).allTextContents().then(parts => parts.join(''));
  expect(await restoredText(1, 'ins')).toContain('Cold brew reaches the shelf');
  expect(await restoredText(1, 'del')).toContain('The bottle label');
  expect(await restoredText(2, 'ins')).toContain('The bottle label');
  expect(await restoredText(2, 'del')).toContain('Cold brew reaches the shelf');
});
