import type { Page } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { test } from '@playwright/test';

const api = 'https://craftigames.kyizl.is-a.dev/api/staff';

interface StaffApiResponse {
  groups: { members: { username: string }[] }[];
}

async function fetchStaffUsernames(network: 'pika' | 'jartex'): Promise<string[]> {
  const response = await fetch(`${api}/${network}`);
  if (!response.ok) {
    throw new Error(
      `[showcase] staff API request failed for ${network}: ${response.status}`,
    );
  }

  const data = (await response.json()) as StaffApiResponse;
  return data.groups.map((group) => group.members[0].username);
}

const NON_STAFF_USERNAMES = {
  pika: [
    'Climbby', // Partner
    'avised', // Champion
    'resuns', // Titan
    'izoo_', // Elite
    'Si1ent_', // VIP
    'Hiqhest', // NON
    'IStayKittens', // Nicked
  ],
  jartex: [
    'Climbby', // Partner
    'Lexi58', // Crystal
    'Meelb', // Diamond
    'DARKpeveresh', // Gold
    'Sandy07', // Iron
    'Si1ent_', // NON
    'IStayKittens', // Nicked
  ],
};

const columns = [
  'NAME',
  'LEVEL',
  'FKDR',
  'WLR',
  'WINS',
  'LOSSES',
  'FINAL_KILLS',
  'FINAL_DEATHS',
  'KILLS',
  'DEATHS',
  'KDR',
  'BEDS_BROKEN',
  'BBLR',
  'WIN_STREAK',
  'PLAYED',
];

const base = {
  activeColumns: columns,
  columnLabels: 'FULL',
  sortBy: 'NAME',
  sortAscending: true,
  interval: 'total',
  mode: 'ALL_MODES',
  integratedMode: false,
  missingPlayersWarning: false,
  fontSize: 18,
  roundedCorners: false,
  textShadow: true,
  autoDetectNetwork: false,
  pikaProxyPort: 25566,
  jartexProxyPort: 25567,
  proxyBindHost: '127.0.0.1',
  proxyBannerDismissed: true,
  paletteId: 'ember',
  theme: {
    bgType: 'solid',
    bgColor: '#130508',
    bgGradientStops: [
      { color: '#ff2d55', position: 0 },
      { color: '#130508', position: 100 },
    ],
    bgGradientDir: 'to bottom right',
    bgImageUrl: '',
    bgImageOpacity: 0.3,
    opacity: 1,
    dynamicColors: false,
    colors: {
      accent: '#ff2d55',
      accentLight: '#ff5c7c',
      border: '#3a1523',
      ink1: '#ffe9ef',
      ink2: '#ff4d73',
      ink3: '#b87586',
      nick: '#fde68a',
      good: '#34d399',
      bad: '#f87171',
      rankOwner: '#BC4141',
      rankDeveloper: '#FF5555',
      rankManager: '#AA0000',
      rankAdmin: '#FF5555',
      rankSrmod: '#00AAAA',
      rankModerator: '#00AA00',
      rankHelper: '#5555FF',
      rankTrial: '#55FFFF',
      rankYoutuber: '#FF5555',
      rankChampion: '#FF5555',
      rankTitan: '#FFD700',
      rankElite: '#55FFFF',
      rankVip: '#55FF55',
    },
  },
};

const nicks = {
  nicks: [{ id: 'showcase-nick-1', nick: 'IStayKittens', realName: 'harshil_mc' }],
};

interface ShowcaseApiResult {
  notFound?: boolean;
  rateLimit?: boolean;
  profile?: Record<string, unknown> | null;
  stats?: unknown;
}

interface ShowcaseStatsApi {
  fetch: (username: string, interval: string, mode: string) => Promise<ShowcaseApiResult>;
}

async function captureShowcase(
  page: Page,
  players: string[],
  configSeed: Record<string, unknown>,
  outputFilename: string,
): Promise<void> {
  await page.setViewportSize({ width: 1400, height: 800 });
  await page.goto('/');

  const network = (configSeed.network as string) ?? 'pikanetwork';

  await page.evaluate(
    ({ cfg, nicks }) => {
      localStorage.setItem('kyra-config', JSON.stringify(cfg));
      localStorage.setItem('kyra-config-version', '4');
      localStorage.setItem('nicks', JSON.stringify(nicks));
      localStorage.setItem('skip-loading', '1');
      localStorage.setItem('skip-announcements', '1');
      localStorage.setItem('skip-remove-btn', '1');
      localStorage.setItem('skip-discord-link', '1');
    },
    { cfg: configSeed, nicks },
  );

  await page.reload({ waitUntil: 'domcontentloaded' });

  await page.waitForSelector('header', { timeout: 15_000 });

  await page.waitForFunction(
    () => (window as unknown as Record<string, unknown>).__pinia !== undefined,
    {
      timeout: 10000,
    },
  );

  await page.evaluate((net: string) => {
    const pinia = (window as unknown as Record<string, unknown>).__pinia as {
      state?: { value?: Record<string, unknown> };
    };
    const state = pinia.state?.value as
      Record<string, Record<string, unknown>> | undefined;
    if (!state) return;
    const playersState = state.players;
    if (playersState !== undefined) {
      playersState.logPathValid = null;
      playersState.proxyConnectedNetwork = net;
    }
  }, network);

  await page.evaluate(
    async ({ names, net }: { names: string[]; net: string }) => {
      const pinia = (window as unknown as Record<string, unknown>).__pinia as
        { state?: { value?: Record<string, unknown> } } | undefined;
      if (pinia === undefined) throw new Error('[showcase] window.__pinia not found');
      const state = pinia.state?.value as
        Record<string, Record<string, unknown>> | undefined;
      if (!state) throw new Error('[showcase] pinia state not found');
      const api = (
        window as unknown as {
          api: { jartex: ShowcaseStatsApi; pika: ShowcaseStatsApi };
        }
      ).api[net === 'jartexnetwork' ? 'jartex' : 'pika'];

      const normalizeProfile = (
        profile: Record<string, unknown> | null,
      ): Record<string, unknown> | null => {
        if (!profile) return profile;
        const rank = profile.rank as { rankDisplay?: string } | undefined;
        if (!rank?.rankDisplay?.includes('Partner')) return profile;
        const ranks = profile.ranks as { name: string }[];
        if (ranks.some((r) => r.name === 'partner')) return profile;
        return {
          ...profile,
          ranks: [
            ...ranks,
            {
              name: 'partner',
              displayName: 'Partner',
              server: '',
              season: null,
              expiry: -1,
            },
          ],
        };
      };

      for (const name of names) {
        (state.players.players as unknown[]).push({
          name,
          realName: name,
          uuid: null,
          loading: true,
          error: null,
          nicked: false,
          profile: null,
          stats: null,
          source: 'manual' as const,
          team: null,
          teamColor: null,
        });
      }

      await Promise.allSettled(
        names.map(async (username, i) => {
          await new Promise((r) => setTimeout(r, i * 350));

          try {
            const result = await api.fetch(username, 'total', 'ALL_MODES');
            const playerList = state.players.players as Array<Record<string, unknown>>;
            const idx = playerList.findIndex(
              (p) => (p.realName as string).toLowerCase() === username.toLowerCase(),
            );
            if (idx === -1) return;

            const p = playerList[idx];
            if (result.notFound) {
              p.nicked = true;
              if (p.profile === null && p.stats === null) p.error = 'not_found';
            } else if (result.rateLimit) {
              p.error = 'rate_limited';
            } else {
              p.profile = normalizeProfile(
                result.profile as Record<string, unknown> | null,
              );
              p.stats = result.stats;
              const apiName =
                typeof result.profile === 'object' && result.profile !== null
                  ? (result.profile.username as string | undefined)
                  : undefined;
              if (typeof apiName === 'string' && apiName.length > 0) {
                p.name = apiName;
                p.realName = apiName;
              }
            }
            p.loading = false;
          } catch {
            const playerList = state.players.players as Array<Record<string, unknown>>;
            const idx = playerList.findIndex(
              (p) => (p.realName as string).toLowerCase() === username.toLowerCase(),
            );
            if (idx !== -1) {
              playerList[idx].loading = false;
              playerList[idx].error = 'network';
            }
          }
        }),
      );
    },
    { names: players, net: network },
  );

  await page.waitForFunction(
    (count) => {
      const rows = document.querySelectorAll('tbody tr');
      return rows.length >= count;
    },
    players.length,
    { timeout: 10_000 },
  );

  await page.waitForTimeout(800);

  await page.setViewportSize({ width: 2000, height: 800 });

  await page.waitForTimeout(300);

  const measured = await page.evaluate((): { contentW: number; contentH: number } => {
    const header = document.querySelector('header');
    const titleBar = header instanceof HTMLElement ? header.offsetHeight : 42;
    const thead = document.querySelector('thead');
    const tbody = document.querySelector('tbody');
    const footer = document.querySelector('.border-t');

    const theadH = thead instanceof HTMLElement ? thead.offsetHeight : 35;
    const tbodyH = tbody instanceof HTMLElement ? tbody.scrollHeight : 0;
    const footerH = footer instanceof HTMLElement ? footer.offsetHeight : 34;

    const table = document.querySelector('table') as HTMLElement | null;
    const w = table?.scrollWidth ?? document.body.scrollWidth;

    return {
      contentW: w,
      contentH: titleBar + theadH + tbodyH + footerH + 20,
    };
  });
  const { contentW, contentH } = measured;

  await page.setViewportSize({ width: contentW, height: contentH });

  await page.waitForTimeout(400);

  await page.addStyleTag({
    content: `
      ::-webkit-scrollbar { display: none !important; width: 0 !important; height: 0 !important; }
      * { scrollbar-width: none !important; }
      `,
  });

  const outDir = path.join(process.cwd(), 'assets');
  fs.mkdirSync(outDir, { recursive: true });

  await page.screenshot({
    path: path.join(outDir, outputFilename),
    fullPage: false,
    clip: {
      x: 0,
      y: 0,
      width: contentW,
      height: contentH,
    },
  });
}

test.setTimeout(180_000);

let pika: string[] = [];
let jartex: string[] = [];

test.beforeAll(async () => {
  const [pikaStaff, jartexStaff] = await Promise.all([
    fetchStaffUsernames('pika'),
    fetchStaffUsernames('jartex'),
  ]);

  pika = [...pikaStaff, ...NON_STAFF_USERNAMES.pika];
  jartex = [...jartexStaff, ...NON_STAFF_USERNAMES.jartex];
});

test('capture pika showcase', async ({ page }) => {
  await captureShowcase(
    page,
    pika,
    { ...base, network: 'pikanetwork' },
    'showcase-pika.png',
  );
});

test('capture jartex showcase', async ({ page }) => {
  await captureShowcase(
    page,
    jartex,
    { ...base, network: 'jartexnetwork' },
    'showcase-jartex.png',
  );
});
