import { expect, test } from '@playwright/test';

type TrafficStatus = { mode: 'middleware' | 'direct'; hasApiKey: boolean };
interface TestWindow extends Window {
  api: {
    traffic: {
      get: () => Promise<TrafficStatus>;
      setMode: (mode: TrafficStatus['mode']) => Promise<TrafficStatus>;
      saveApiKey: (key: string) => Promise<TrafficStatus>;
      removeApiKey: () => Promise<TrafficStatus>;
    };
    updater: { onStatus: () => () => void };
    proxy: { getStatus: () => Promise<unknown> };
  };
}

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    for (const name of ['skip-loading', 'skip-announcements', 'skip-discord-link']) {
      localStorage.setItem(name, '1');
    }
  });
  await page.goto('/');
  await page.waitForFunction(() => !!(window as unknown as TestWindow).api);
  await page.evaluate(() => {
    const appWindow = window as unknown as TestWindow;
    let status: { mode: 'middleware' | 'direct'; hasApiKey: boolean } = {
      mode: 'middleware',
      hasApiKey: false,
    };
    appWindow.api.traffic = {
      get: async () => ({ ...status }),
      setMode: async (mode) => {
        status = { ...status, mode };
        return { ...status };
      },
      saveApiKey: async (key) => {
        if (key === 'unavailable')
          throw new Error(
            "Error invoking remote method 'traffic:save-api-key': Error: Secure storage is unavailable.",
          );
        status = { mode: 'direct', hasApiKey: true };
        return { ...status };
      },
      removeApiKey: async () => {
        status = { mode: 'middleware', hasApiKey: false };
        return { ...status };
      },
    };
    appWindow.api.updater.onStatus = () => () => {};
    appWindow.api.proxy.getStatus = async () => ({
      pika: {
        running: false,
        port: 25566,
        bindHost: '127.0.0.1',
        clientCount: 0,
        error: null,
      },
      jartex: {
        running: false,
        port: 25567,
        bindHost: '127.0.0.1',
        clientCount: 0,
        error: null,
      },
    });
    window.location.hash = '/settings?tab=traffic';
  });
});

test('saves, replaces, switches, and removes the API key', async ({ page }) => {
  const middleware = page.getByRole('radio', { name: /Route through middleware proxy/ });
  const direct = page.getByRole('radio', { name: /Use my own API key/ });
  await expect(middleware).toBeChecked();
  await direct.check();
  await expect(
    page.getByRole('button', { name: 'Save API key', exact: true }),
  ).toBeDisabled();
  const keyInput = page.locator('#traffic-api-key');
  await expect(keyInput).toHaveAttribute('type', 'password');
  await keyInput.fill('first-test-key');
  await page.getByRole('button', { name: 'Save API key', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('saved securely');
  await expect(direct).toBeChecked();
  await expect(keyInput).toHaveValue('');
  await keyInput.fill('replacement-test-key');
  await page.getByRole('button', { name: 'Replace API key', exact: true }).click();
  await expect(keyInput).toHaveValue('');
  await middleware.check();
  await expect(page.getByRole('status')).toContainText('preference saved');
  await expect(
    page.getByRole('button', { name: 'Remove API key', exact: true }),
  ).toBeVisible();
  await direct.check();
  await expect(page.getByRole('status')).toContainText('preference saved');
  await page.getByRole('button', { name: 'Remove API key', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('API key removed');
  await expect(middleware).toBeChecked();
  await expect(keyInput).toHaveCount(0);
  expect(await page.evaluate(() => JSON.stringify(localStorage))).not.toContain(
    'test-key',
  );
});

test('keeps the key entry available after a save failure and supports retry', async ({
  page,
}) => {
  await page.getByRole('radio', { name: /Use my own API key/ }).check();
  await page.locator('#traffic-api-key').fill('unavailable');
  await page.getByRole('button', { name: 'Save API key', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('Secure storage is unavailable');
  await expect(page.getByRole('alert')).not.toContainText('Error invoking remote method');
  await expect(page.getByRole('radio', { name: /Use my own API key/ })).toBeChecked();
  await expect(page.locator('#traffic-api-key')).toHaveValue('unavailable');
  expect(
    await page.evaluate(async () => (window as unknown as TestWindow).api.traffic.get()),
  ).toEqual({ mode: 'middleware', hasApiKey: false });
  await page.locator('#traffic-api-key').fill('retry-test-key');
  await page.getByRole('button', { name: 'Save API key', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('saved securely');
  await expect(page.locator('#traffic-api-key')).toHaveValue('');
});
