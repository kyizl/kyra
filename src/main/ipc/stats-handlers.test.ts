import type { StatsRoute } from '../traffic-service';
import { beforeEach, expect, it, vi } from 'vitest';
import { registerStatsHandlers } from './stats-handlers';

const mocks = vi.hoisted(() => {
  const route: StatsRoute = {
    base: 'https://stats.pika-network.net/api',
    headers: { 'X-Api-Key': 'test-key' },
    cacheScope: 'pika:1',
  };
  return {
    handlers: new Map<string, (...args: unknown[]) => Promise<unknown>>(),
    get: vi.fn(),
    route,
  };
});

vi.mock('electron', () => ({
  ipcMain: {
    handle: (channel: string, handler: (...args: unknown[]) => Promise<unknown>) =>
      mocks.handlers.set(channel, handler),
  },
}));
vi.mock('../logger', () => ({ dbg: { ipc: vi.fn(), http: vi.fn(), error: vi.fn() } }));
vi.mock('../traffic-service', () => ({ getStatsRoute: () => mocks.route }));
vi.mock('../http-client', () => ({
  apiGet: mocks.get,
  cache: { get: () => null, set: vi.fn() },
  dedupe: async (_key: string, action: () => Promise<unknown>) => action(),
}));

beforeEach(() => {
  mocks.get
    .mockReset()
    .mockResolvedValue({ status: 200, data: { username: 'Canonical' } });
  mocks.route = {
    base: 'https://stats.pika-network.net/api',
    headers: { 'X-Api-Key': 'test-key' },
    cacheScope: 'pika:1',
  };
  registerStatsHandlers();
});

it.each([false, true])(
  'authenticates both profile and leaderboard requests (concurrent: %s)',
  async (concurrent) => {
    await mocks.handlers.get('pika:fetch')!(
      null,
      'player',
      'total',
      'ALL_MODES',
      concurrent,
    );
    expect(mocks.get).toHaveBeenCalledTimes(2);
    for (const [url, headers] of mocks.get.mock.calls) {
      expect(url).toMatch(/^https:\/\/stats\.pika-network\.net\/api\/profile\//);
      expect(headers).toEqual({ 'X-Api-Key': 'test-key' });
    }
    expect(mocks.get.mock.calls[1][0]).toContain(
      `/profile/${concurrent ? 'player' : 'Canonical'}/leaderboard`,
    );
  },
);

it('routes stats and clans through the selected endpoint without middleware credentials', async () => {
  await mocks.handlers.get('pika:stats')!(null, 'player', 'total', 'ALL_MODES');
  await mocks.handlers.get('pika:clan')!(null, 'my clan');
  expect(mocks.get.mock.calls[1]).toEqual([
    'https://stats.pika-network.net/api/clans/my%20clan',
    { 'X-Api-Key': 'test-key' },
  ]);
  mocks.route = {
    base: 'https://proxy.kyizl.is-a.dev/api',
    headers: {},
    cacheScope: 'pika:2',
  };
  await mocks.handlers.get('pika:fetch')!(null, 'player');
  await mocks.handlers.get('pika:stats')!(null, 'player', 'total', 'ALL_MODES');
  await mocks.handlers.get('pika:clan')!(null, 'clan');
  for (const [url, headers] of mocks.get.mock.calls.slice(2)) {
    expect(url).toMatch(/^https:\/\/proxy\.kyizl\.is-a\.dev\/api\//);
    expect(headers).toEqual({});
  }
});
