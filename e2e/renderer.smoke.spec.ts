import { expect, test } from '@playwright/test';

test('loads the Kyra renderer shell', async ({ page }) => {
  await page.goto('/');
  await expect(page).toHaveTitle('Kyra Overlay');
  await expect(page.locator('#app')).toBeVisible();
});
