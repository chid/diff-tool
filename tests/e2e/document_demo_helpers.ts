import type { Page } from '@playwright/test';

export async function openDemo(page: Page, title: string) {
  await page.goto('./?tab=documents');
  await page.getByRole('link', { name: title, exact: true }).click();
}
