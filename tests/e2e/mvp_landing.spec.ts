import { expect, test } from '@playwright/test';

test.describe('MVP landing and examples', () => {
  test('welcome page presents MVP product claims and roadmap boundaries', async ({ page }) => {
    await page.goto('/?demo=welcome');

    await expect(page.getByRole('heading', { name: 'Diff Tool' })).toBeVisible();
    await expect(page.getByText('Fast, accurate text diffing')).toBeVisible();
    await expect(page.getByText('Compare text files and scan local folders')).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Text diff' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Folder scan' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Roadmap modes' })).toBeVisible();
    await expect(page.getByText('Image, merge, Git, Office, and AI workflows stay post-MVP.')).toBeVisible();

    await expect(page.getByRole('heading', { name: 'Folder compare' })).toHaveCount(0);
    await expect(page.getByRole('heading', { name: 'Image diff' })).toHaveCount(0);
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
  });
});
