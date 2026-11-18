import { expect, test, type Page } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { strFromU8, strToU8, unzipSync, zipSync } from 'fflate';

const sample = (kind: string, side: string, ext: string) =>
  join(process.cwd(), 'fixtures/cases', 'office-' + kind + '-basic', side + '.' + ext);
const pdf = (name: string) => join(process.cwd(), 'tests/e2e/fixtures/documents', name + '.pdf');
const left = sample('word', 'left', 'docx');
const right = sample('word', 'right', 'docx');
type Upload = string | { name: string; mimeType: string; buffer: Buffer };

function edited(path: string, name: string, part: string, patch: (s: string) => string): Upload {
  const archive = unzipSync(readFileSync(path));
  archive[part] = strToU8(patch(strFromU8(archive[part])));
  return { name, mimeType: 'application/octet-stream', buffer: Buffer.from(zipSync(archive)) };
}

async function open(page: Page, a: Upload, b: Upload) {
  await page.goto('/');
  const first = page.waitForEvent('filechooser');
  await page.getByRole('button', { name: 'Open left...' }).click();
  await (await first).setFiles(a);
  const second = page.waitForEvent('filechooser');
  await page.getByRole('button', { name: /Revised · Choose file/ }).click();
  await (await second).setFiles(b);
}

test('opens Word from the regular file pickers and downloads real tracked changes', async ({ page }, info) => {
  await open(page, left, right);
  const review = page.getByRole('article', { name: 'Document comparison' });
  await expect(review.locator('del')).toHaveText('left');
  await expect(review.locator('ins')).toHaveText('right');
  const pending = page.waitForEvent('download');
  await review.getByRole('button', { name: 'Download tracked Word report' }).click();
  const file = await pending;
  const out = info.outputPath(file.suggestedFilename());
  await file.saveAs(out);
  const xml = strFromU8(unzipSync(readFileSync(out))['word/document.xml']);
  expect(xml).toContain('<w:del '); expect(xml).toContain('<w:ins ');
  const projections = await page.evaluate(xml => {
    const doc = new DOMParser().parseFromString(xml, 'application/xml');
    return ['ins', 'del'].map(exclude => Array.from(doc.getElementsByTagNameNS('*', 'r'))
      .filter(r => r.parentElement?.localName !== exclude).map(r => r.textContent).join(''));
  }, xml);
  expect(projections).toEqual(['Hello from the left file.', 'Hello from the right file.']);
});

test('compares direct formatting and headers when body text is identical', async ({ page }) => {
  const zip = unzipSync(readFileSync(left));
  zip['word/document.xml'] = strToU8(strFromU8(zip['word/document.xml']).replace('<w:r>', '<w:r><w:rPr><w:b/></w:rPr>'));
  zip['word/header1.xml'] = strToU8('<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:p><w:r><w:t>Draft only</w:t></w:r></w:p></w:hdr>');
  await open(page, left, { name: 'format.docx', mimeType: 'application/octet-stream', buffer: Buffer.from(zipSync(zip)) });
  await expect(page.getByRole('heading', { name: /Word body formatting/ })).toBeVisible();
  await expect(page.locator('ins').filter({ hasText: 'Draft only' })).toBeVisible();
  await expect(page.getByText('2 changed sections')).toBeVisible();
});

test('resolves split words across Word runs without a spurious text change', async ({ page }) => {
  const chunked = edited(left, 'repacked.docx', 'word/document.xml',
    s => s.replace('Hello from', 'Hel</w:t></w:r><w:r><w:t>lo from'));
  await open(page, left, chunked);
  await expect(page.getByText('No differences in the compared text')).toBeVisible();
});

test('reads a searchable PDF against Word and replaces stale results on error', async ({ page }) => {
  await open(page, left, pdf('right'));
  await expect(page.locator('.document-prose ins')).toHaveText('right');
  await page.getByLabel('Choose revised file').setInputFiles({ name: 'bad.docx', mimeType: 'application/zip', buffer: Buffer.from('broken') });
  await expect(page.getByRole('alert')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Download tracked Word report' })).toHaveCount(0);
  await page.getByLabel('Choose revised file').setInputFiles(left);
  await expect(page.getByText('No differences in the compared text')).toBeVisible();
});

test('runs English OCR locally on an image-only PDF', async ({ page }) => {
  const external: string[] = [];
  page.context().on('request', req => {
    const u = new URL(req.url());
    if (u.protocol.startsWith('http') && u.hostname !== 'localhost' && u.hostname !== '127.0.0.1') external.push(req.url());
  });
  await open(page, left, pdf('scanned-right'));
  await expect(page.getByText(/Revised used English OCR on page\(s\) 1/)).toBeVisible({ timeout: 30000 });
  await expect(page.locator('.document-prose ins')).toHaveText('right');
  expect(external).toEqual([]);
});

test('compares rendered PDF pages independently of matching text', async ({ page }) => {
  await open(page, pdf('right'), pdf('scanned-right'));
  await page.getByLabel('Compare rendered PDF pages').check();
  await expect(page.getByRole('heading', { name: 'Page 1 · Changed' })).toBeVisible();
  await expect(page.getByRole('img', { name: 'Page 1 visual comparison' })).toBeVisible();
  await page.getByLabel('Choose revised file').setInputFiles(pdf('right'));
  await expect(page.getByRole('heading', { name: 'Page 1 · Unchanged' })).toBeVisible();
});

test('compares workbook cells and formulas even when cached results agree', async ({ page }) => {
  const fixture = sample('xlsx', 'left', 'xlsx');
  const formula = (f: string) => edited(fixture, 'financials.xlsx', 'xl/worksheets/sheet1.xml',
    s => s.replace(/<c r="A1"[^>]*>[\s\S]*?<\/c>/, '<c r="A1"><f>' + f + '</f><v>100</v></c>'));
  await open(page, formula('50+50'), formula('10*10'));
  await expect(page.getByRole('heading', { name: /Sheet1!A1 formula/ })).toBeVisible();
  await expect(page.getByText('1 changed section', { exact: true })).toBeVisible();
  await page.getByLabel('Choose original file').setInputFiles(fixture);
  await page.getByLabel('Choose revised file').setInputFiles(sample('xlsx', 'right', 'xlsx'));
  await expect(page.getByRole('heading', { name: 'Sheet1!A1 modified' })).toBeVisible();
});

test('reads presentation slide text and rejects incompatible formats', async ({ page }) => {
  await open(page, sample('pptx', 'left', 'pptx'), sample('pptx', 'right', 'pptx'));
  await expect(page.getByRole('heading', { name: /Slide 1 modified/ })).toBeVisible();
  await expect(page.locator('.document-prose del').first()).toContainText('Left');
  await page.getByLabel('Choose original file').setInputFiles(left);
  await expect(page.getByRole('alert')).toContainText('presentation');
});

test('dropping a document pair enters document review', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Documents', exact: true }).waitFor();
  await page.evaluate(([a, b]) => {
    const data = new DataTransfer();
    data.items.add(new File([Uint8Array.from(a)], 'old.docx'));
    data.items.add(new File([Uint8Array.from(b)], 'new.docx'));
    window.dispatchEvent(new DragEvent('drop', { dataTransfer: data }));
  }, [Array.from(readFileSync(left)), Array.from(readFileSync(right))]);
  await expect(page.locator('.document-prose ins')).toHaveText('right');
});

// Reconstructed excerpt from a published Litera redline. See docs/document-comparison.md.
test('reproduces both word replacements marked by Litera in PTE 2020-02 section I(c)', async ({ page }) => {
  const clause = (text: string) => edited(left, 'clause.docx', 'word/document.xml',
    xml => xml.replace('Hello from the left file.', text));
  await open(page, clause('This exemption does not apply if:'), clause('This exemption is not available if:'));
  const prose = page.locator('.document-prose');
  await expect(prose.locator('del')).toHaveText(['does', 'apply']);
  await expect(prose.locator('ins')).toHaveText(['is', 'available']);
  await page.getByRole('button', { name: 'Next edit' }).click();
  await expect(prose.locator('del').first()).toBeFocused();
  await page.getByRole('button', { name: 'Next edit' }).click();
  await expect(prose.locator('del').last()).toBeFocused();
});

test('reports a relocated clause and a table-only structure change', async ({ page }) => {
  const doc = (paras: string[], cells: string) => edited(left, 'agreement.docx', 'word/document.xml',
    xml => xml.replace(/<w:body>[\s\S]*?<\/w:body>/, '<w:body>' +
      paras.map(t => '<w:p><w:r><w:t>' + t + '</w:t></w:r></w:p>').join('') +
      '<w:tbl><w:tr>' + cells + '</w:tr></w:tbl></w:body>'));
  const cell = '<w:tc><w:p/></w:tc>';
  await open(page, doc(['Alpha', 'Beta', 'Gamma'], cell), doc(['Beta', 'Gamma', 'Alpha'], cell + cell));
  await expect(page.getByRole('list', { name: 'Moved paragraphs' })).toContainText('Paragraph 1 moved to 3 · Alpha');
  await expect(page.getByRole('heading', { name: /Table 1 row 1 structure/ })).toBeVisible();
});

test('changing one file reuses the other scan and rereads a replacement with the same filename', async ({ page }) => {
  const reads: string[] = [];
  await page.exposeFunction('recordDocumentRead', (name: string) => reads.push(name));
  await page.addInitScript(() => {
    const read = File.prototype.arrayBuffer;
    File.prototype.arrayBuffer = function () {
      void (window as unknown as { recordDocumentRead(name: string): Promise<void> }).recordDocumentRead(this.name);
      return read.call(this);
    };
  });
  await open(page, left, pdf('scanned-right'));
  await expect(page.locator('.document-prose ins')).toHaveText('right', { timeout: 30_000 });
  const count = () => reads.filter(n => n === 'scanned-right.pdf').length;
  const initialReads = count();
  expect(initialReads).toBeGreaterThan(0);
  await page.getByLabel('Choose original file').setInputFiles(right);
  await expect(page.getByText('No differences in the compared text')).toBeVisible();
  expect(count()).toBe(initialReads);
  await page.getByLabel('Choose revised file').setInputFiles({
    name: 'scanned-right.pdf', mimeType: 'application/pdf', buffer: Buffer.from('broken'),
  });
  await expect(page.getByRole('alert')).toBeVisible();
  expect(count()).toBeGreaterThan(initialReads);
});
