import { test, expect } from '@playwright/test';

test('app loads and shows the UI', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('body')).toBeAttached();
  await expect(page.locator('#root')).not.toBeEmpty();
});
