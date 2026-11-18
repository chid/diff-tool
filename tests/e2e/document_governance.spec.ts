import { expect, test, type Locator, type Page, type TestInfo } from '@playwright/test';
import { copyFile, mkdir } from 'node:fs/promises';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import anthropic from '../../apps/desktop/src/documents/governance/anthropic';
import google from '../../apps/desktop/src/documents/governance/google';
import openai from '../../apps/desktop/src/documents/governance/openai';

const cases = [anthropic, google, openai];
type Case = typeof cases[number];

const escapePattern = (value: string) => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

async function checkSources(region: Locator, review: Case) {
  for (const source of [review.before, review.after]) {
    const link = region.getByRole('link', { name: new RegExp(escapePattern(source.label)) });
    await expect(link).toHaveAttribute('href', source.url);
    await expect(link).toHaveAccessibleName(new RegExp(escapePattern(source.date)));
  }
  if (review.reference) {
    await expect(region.getByRole('link', { name: new RegExp(escapePattern(review.reference.label)) }))
      .toHaveAttribute('href', review.reference.url);
  }
}

async function downloadAndOpenOffline(page: Page, info: TestInfo) {
  const pending = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Download review report', exact: true }).click();
  const download = await pending;
  expect(download.suggestedFilename()).toMatch(/\.html$/);
  const path = info.outputPath('review.html');
  await download.saveAs(path);
  await page.goto('about:blank');
  await page.context().setOffline(true);
  await page.goto(pathToFileURL(path).href);
  await page.emulateMedia({ media: 'print' });
  return path;
}

for (const review of cases) {
  test(`${review.company} governance review opens directly and exports the dated sources, evidence and client question`, async ({ page }, info) => {
    await page.goto('./?tab=documents');
    const chooser = page.getByRole('region', { name: 'AI governance reviews' });
    const link = chooser.getByRole('link', { name: review.company + ' governance review', exact: true });
    const directUrl = await link.evaluate((a: HTMLAnchorElement) => a.href);
    await link.click();
    const brief = page.getByRole('region', { name: 'Consultant brief' });
    await expect(brief.getByRole('heading', { level: 2, name: review.title })).toBeVisible();
    await expect(brief).toContainText(review.question);

    // A fresh navigation exercises a shared link without relying on the chooser's state.
    await page.goto('about:blank');
    await page.goto(directUrl);
    await expect(brief.getByRole('heading', { level: 2, name: review.title })).toBeVisible();
    await expect(brief).toContainText(review.finding);
    await checkSources(brief, review);
    const prose = page.locator('.document-review .document-prose');
    await expect(prose).toHaveCount(1);
    const readings = await prose.evaluate(element => ['INS', 'DEL'].map(excluded =>
      Array.from(element.children).filter(child => child.tagName !== excluded).map(child => child.textContent).join('')));
    expect(readings).toEqual([review.before.excerpt, review.after.excerpt]);

    const path = await downloadAndOpenOffline(page, info);
    await expect(page.getByRole('heading', { level: 1, name: review.title, exact: true })).toBeVisible();
    await expect(page.getByRole('heading', { level: 2, name: 'Client follow-up', exact: true })).toBeVisible();
    const body = page.locator('body');
    await expect(body).toContainText(review.finding);
    await expect(body).toContainText(review.question);
    for (const source of [review.before, review.after]) {
      await expect(page.locator('blockquote').and(page.getByText(source.excerpt, { exact: true }))).toBeVisible();
      await expect(page.getByText(source.text, { exact: true })).toBeVisible();
    }
    await checkSources(body, review);
    await expect(page.locator('script')).toHaveCount(0);

    // Every published sample must be the download that just passed the offline checks.
    const destination = process.env.DEMO_REPORT_DIR;
    if (destination) {
      await mkdir(destination, { recursive: true });
      await copyFile(path, join(destination, review.id + '-governance-report.html'));
    }
  });
}

for (const side of ['original', 'revised']) {
  test(`replacing the ${side} file removes consultant attribution from the UI and the saved report`, async ({ page }, info) => {
    await page.goto('./?tab=documents');
    await page.getByRole('link', { name: openai.company + ' governance review', exact: true }).click();
    await expect(page.getByRole('region', { name: 'Consultant brief' })).toContainText(openai.question);
    const input = 'Our team will review the employee onboarding guide in February.';
    await page.getByLabel(`Choose ${side} file`).setInputFiles({
      name: 'team-review.txt', mimeType: 'text/plain', buffer: Buffer.from(input),
    });
    await expect(page.getByRole('region', { name: 'Consultant brief' })).toHaveCount(0);
    await downloadAndOpenOffline(page, info);
    await expect(page.getByRole('heading', { level: 1, name: 'Document comparison report', exact: true })).toBeVisible();
    await expect(page.locator('body')).toContainText('team-review.txt');
    await expect(page.getByRole('heading', { name: 'Client follow-up' })).toHaveCount(0);
    await expect(page.getByText(openai.question, { exact: true })).toHaveCount(0);
    for (const source of [openai.before, openai.after, openai.reference]) {
      await expect(page.locator('a').filter({ hasText: source.label })).toHaveCount(0);
    }
  });
}
