import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { strFromU8, strToU8, unzipSync, zipSync } from 'fflate';
import { openDemo } from './document_demo_helpers';

test('supplier agreement marks the payment and delivery changes and identifies the relocated packing clause', async ({ page }) => {
  await openDemo(page, 'Supplier agreement');
  const body = page.locator('.document-review section').filter({
    has: page.getByRole('heading', { name: 'Document body modified' }),
  });
  await expect(body.locator('del').filter({ hasText: /^Wednesday$/ })).toBeVisible();
  await expect(body.locator('ins').filter({ hasText: /^Friday$/ })).toBeVisible();
  await expect(body.locator('del').filter({ hasText: /^30$/ })).toBeVisible();
  await expect(body.locator('ins').filter({ hasText: /^14$/ })).toBeVisible();
  await expect(body.getByRole('listitem')).toHaveText([
    'Paragraph 5 moved to 8 · Each crate must carry the order number and the number of cups packed inside.',
  ]);
});

test('a Word file with pending insertions, deletions and moves compares in its revised state', async ({ page }) => {
  const folder = join(process.cwd(), 'fixtures/cases/office-word-basic');
  const plain = join(folder, 'right.docx');
  const archive = unzipSync(readFileSync(plain));
  const part = 'word/document.xml';
  const tracked = [
    '<w:moveToRangeStart w:id="3" w:name="word1" w:author="Reviewer" w:date="2026-10-01T09:00:00Z"/>',
    '<w:moveTo w:id="4" w:author="Reviewer"><w:r><w:t>Hello</w:t></w:r></w:moveTo>',
    '<w:moveToRangeEnd w:id="3"/>',
    '<w:r><w:t xml:space="preserve"> from the </w:t></w:r>',
    '<w:del w:id="5" w:author="Reviewer"><w:r><w:delText>obsolete</w:delText></w:r></w:del>',
    '<w:ins w:id="6" w:author="Reviewer"><w:r><w:t>right</w:t></w:r></w:ins>',
    '<w:r><w:t xml:space="preserve"> file.</w:t></w:r>',
    '<w:moveFromRangeStart w:id="1" w:name="word1" w:author="Reviewer" w:date="2026-10-01T09:00:00Z"/>',
    '<w:moveFrom w:id="2" w:author="Reviewer"><w:r><w:delText>Hello</w:delText></w:r></w:moveFrom>',
    '<w:moveFromRangeEnd w:id="1"/>',
  ].join('');
  archive[part] = strToU8(strFromU8(archive[part]).replace(/<w:r>[\s\S]*?<\/w:r>/, tracked));
  await page.goto('/?tab=documents');
  await page.getByLabel('Choose original file').setInputFiles({
    name: 'tracked.docx', mimeType: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    buffer: Buffer.from(zipSync(archive)),
  });
  await page.getByLabel('Choose revised file').setInputFiles(plain);
  await expect(page.getByText('No differences in the compared text')).toBeVisible();
  await page.getByLabel('Choose revised file').setInputFiles(join(folder, 'left.docx'));
  await expect(page.locator('.document-prose del')).toHaveText('right');
  await expect(page.locator('.document-prose ins')).toHaveText('left');
});
