import { expect, test } from '@playwright/test';

test.describe('MVP landing and examples', () => {
  test('welcome page opens document comparison alongside text and folder features', async ({ page }) => {
    await page.goto('/?demo=welcome');

    await expect(page.getByRole('heading', { name: 'Diff Tool' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Text diff & Intraline' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Folder scan' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'GitHub PR & Commit Review' })).toBeVisible();

    await expect(page.getByRole('heading', { name: 'Folder compare' })).toHaveCount(0);
    await expect(page.getByRole('heading', { name: 'Image diff' })).toHaveCount(0);
    await page.getByRole('link', { name: 'Compare documents' }).click();
    await expect(page.getByRole('heading', { name: 'Compare documents' })).toBeVisible();
  });

  test('example gallery opens with real text-diff examples first', async ({ page }) => {
    await page.goto('/?demo=gallery');

    await expect(page.getByRole('heading', { name: 'Live Examples' })).toBeVisible();
    await expect(page.getByRole('button', { name: /Single-line change/ })).toBeVisible();
    await expect(page.getByRole('button', { name: /Multi-hunk diff/ })).toBeVisible();
    await expect(page.getByRole('button', { name: /Unicode & accents/ })).toBeVisible();
    await expect(page.locator('.diff-grid')).toBeVisible();

    await page.getByRole('button', { name: /Unicode & accents/ }).click();
    await expect(page.locator('.diff-pane')).toContainText('café');
    await expect(page.locator('.intra-highlight').filter({ hasText: 'é' })).toHaveCount(1);
    await page.getByRole('link', { name: 'Try document examples' }).click();
    const examples = page.getByRole('region', { name: 'Document examples' });
    await expect(examples.getByRole('link')).toHaveCount(3);
    for (const preview of await examples.locator('img').all()) {
      await preview.scrollIntoViewIfNeeded();
      await expect.poll(() => preview.evaluate(img => (img as HTMLImageElement).naturalWidth)).toBeGreaterThan(0);
    }
  });
});
