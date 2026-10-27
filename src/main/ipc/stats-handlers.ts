import type { StatsRoute } from '../traffic-service';
import { ipcMain } from 'electron';
import { apiGet, cache, dedupe } from '../http-client';
import { dbg } from '../logger';

import { getStatsRoute } from '../traffic-service';

type StatsNetwork = 'pika' | 'jartex';

interface FetchResult {
  profile: unknown;
  stats: unknown;
  notFound: boolean;
  rateLimit: boolean;
  statsDisabled: boolean;
}

export function registerStatsHandlers(): void {
  ipcMain.handle('pika:fetch', createFetchHandler('pika'));
  ipcMain.handle('pika:stats', createStatsHandler('pika'));
  ipcMain.handle('pika:clan', createClanHandler('pika'));
  ipcMain.handle('jartex:fetch', createFetchHandler('jartex'));
  ipcMain.handle('jartex:stats', createStatsHandler('jartex'));
  ipcMain.handle('jartex:clan', createClanHandler('jartex'));
}

function createFetchHandler(network: StatsNetwork) {
  return async (
    _: Electron.IpcMainInvokeEvent,
    username: string,
    interval = 'total',
    mode = 'ALL_MODES',
    concurrent = false,
  ): Promise<FetchResult> => {
    const route = getStatsRoute(network);
    const { base } = route;
    const key = `${route.cacheScope}:${username.toLowerCase()}:${interval}:${mode}`;
    dbg.ipc(
      `${network}:fetch - user="${username}", interval=${interval}, mode=${mode}, concurrent=${concurrent}`,
    );

    const cached = cache.get<FetchResult>(key);
    if (cached !== null) return Promise.resolve(cached);

    return dedupe(key, async () => {
      const cachedAfterDedupe = cache.get<FetchResult>(key);
      if (cachedAfterDedupe !== null) return cachedAfterDedupe;

      const profileUrl = `${base}/profile/${encodeURIComponent(username)}`;
      const directStatsUrl = `${base}/profile/${encodeURIComponent(username)}/leaderboard?type=bedwars&interval=${interval}&mode=${mode}`;

      if (concurrent) {
        return fetchProfileAndStatsConcurrently(
          network,
          key,
          username,
          profileUrl,
          directStatsUrl,
          route,
        );
      }
      return fetchProfileThenStats(
        network,
        key,
        username,
        profileUrl,
        interval,
        mode,
        route,
      );
    });
  };
}

async function fetchProfileAndStatsConcurrently(
  network: StatsNetwork,
  key: string,
  username: string,
  profileUrl: string,
  statsUrl: string,
  route: StatsRoute,
): Promise<FetchResult> {
  const [profileResponse, statsResponse] = await Promise.all([
    apiGet(profileUrl, route.headers).catch((error: Error) => {
      dbg.error(`${network}:fetch - profile "${username}" - ${error.message}`);
      return null;
    }),
    apiGet(statsUrl, route.headers).catch((error: Error) => {
      dbg.error(`${network}:fetch - stats "${username}" - ${error.message}`);
      return null;
    }),
  ]);

  const notFound =
    profileResponse !== null &&
    (profileResponse.status === 404 || profileResponse.status === 400);
  const rateLimit = profileResponse !== null && profileResponse.status === 429;

  if (rateLimit) {
    dbg.ipc(`${network}:fetch - "${username}" -> RATE LIMITED`);
    return {
      profile: null,
      stats: null,
      notFound: false,
      rateLimit: true,
      statsDisabled: false,
    };
  }

  const profile = profileResponse?.status === 200 ? profileResponse.data : null;
  const statsDisabled = statsResponse !== null && statsResponse.status === 204;
  const stats = statsResponse?.status === 200 ? statsResponse.data : null;
  dbg.ipc(
    `${network}:fetch - "${username}" -> OK - profile=${profile !== null}, stats=${stats !== null}, statsDisabled=${statsDisabled}`,
  );
  const result: FetchResult = {
    profile,
    stats,
    notFound,
    rateLimit: false,
    statsDisabled,
  };
  if (!notFound) cache.set(key, result);
  return result;
}

async function fetchProfileThenStats(
  network: StatsNetwork,
  key: string,
  username: string,
  profileUrl: string,
  interval: string,
  mode: string,
  route: StatsRoute,
): Promise<FetchResult> {
  const profileResponse = await apiGet(profileUrl, route.headers).catch(
    (error: Error) => {
      dbg.error(`${network}:fetch - profile "${username}" - ${error.message}`);
      return null;
    },
  );

  const notFound =
    profileResponse !== null &&
    (profileResponse.status === 404 || profileResponse.status === 400);
  const rateLimit = profileResponse !== null && profileResponse.status === 429;

  if (notFound) {
    dbg.ipc(`${network}:fetch - "${username}" -> NOT FOUND`);
    return {
      profile: null,
      stats: null,
      notFound: true,
      rateLimit: false,
      statsDisabled: false,
    };
  }

  if (rateLimit) {
    dbg.ipc(`${network}:fetch - "${username}" -> RATE LIMITED`);
    return {
      profile: null,
      stats: null,
      notFound: false,
      rateLimit: true,
      statsDisabled: false,
    };
  }

  const profile = profileResponse?.status === 200 ? profileResponse.data : null;
  const canonicalUsername =
    (profile as { username?: string } | null)?.username ?? username;
  const canonicalStatsUrl = `${route.base}/profile/${encodeURIComponent(canonicalUsername)}/leaderboard?type=bedwars&interval=${interval}&mode=${mode}`;

  const statsResponse = await apiGet(canonicalStatsUrl, route.headers).catch(
    (error: Error) => {
      dbg.error(`${network}:fetch - stats "${canonicalUsername}" - ${error.message}`);
      return null;
    },
  );

  const statsDisabled = statsResponse !== null && statsResponse.status === 204;
  const stats = statsResponse?.status === 200 ? statsResponse.data : null;
  dbg.ipc(
    `${network}:fetch - "${canonicalUsername}" -> OK - profile=${profile !== null}, stats=${stats !== null}, statsDisabled=${statsDisabled}`,
  );
  const result: FetchResult = {
    profile,
    stats,
    notFound: false,
    rateLimit: false,
    statsDisabled,
  };
  cache.set(key, result);
  return result;
}

function createStatsHandler(network: StatsNetwork) {
  return async (
    _: Electron.IpcMainInvokeEvent,
    username: string,
    interval: string,
    mode: string,
  ): Promise<unknown> => {
    const route = getStatsRoute(network);
    const { base } = route;
    const key = `${route.cacheScope}:stats:${username.toLowerCase()}:${interval}:${mode}`;
    dbg.ipc(`${network}:stats - user="${username}", interval=${interval}, mode=${mode}`);

    const cached = cache.get<unknown>(key);
    if (cached !== null) return Promise.resolve(cached);

    return dedupe(key, async () => {
      const cachedAfterDedupe = cache.get<unknown>(key);
      if (cachedAfterDedupe !== null) return cachedAfterDedupe;

      const url = `${base}/profile/${encodeURIComponent(username)}/leaderboard?type=bedwars&interval=${interval}&mode=${mode}`;
      dbg.http(`${network}:stats request for "${username}"`);

      const response = await apiGet(url, route.headers).catch((error: Error) => {
        dbg.error(`${network}:stats - "${username}" - ${error.message}`);
        return null;
      });

      const data = response?.status === 200 ? response.data : null;
      if (data !== null) cache.set(key, data);
      return data;
    });
  };
}

function createClanHandler(network: StatsNetwork) {
  return async (_: Electron.IpcMainInvokeEvent, name: string): Promise<unknown> => {
    const route = getStatsRoute(network);
    const { base } = route;
    const key = `${route.cacheScope}:clan:${name.toLowerCase()}`;
    dbg.ipc(`${network}:clan - name="${name}"`);

    const cached = cache.get<unknown>(key);
    if (cached !== null) return Promise.resolve(cached);

    return dedupe(key, async () => {
      const cachedAfterDedupe = cache.get<unknown>(key);
      if (cachedAfterDedupe !== null) return cachedAfterDedupe;

      const url = `${base}/clans/${encodeURIComponent(name)}`;
      dbg.http(`${network}:clan request for "${name}"`);

      const response = await apiGet(url, route.headers).catch((error: Error) => {
        dbg.error(`${network}:clan - "${name}" - ${error.message}`);
        return null;
      });

      if (!response) throw new Error('Network error');
      if (response.status === 404 || response.status === 400) return { notFound: true };
      if (response.status !== 200) throw new Error(`HTTP ${response.status}`);

      cache.set(key, response.data);
      dbg.ipc(`${network}:clan - "${name}" -> OK`);
      return response.data;
    });
  };
}
