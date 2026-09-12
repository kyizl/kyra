import { getActivePinia } from 'pinia';

export type Interval = 'total' | 'weekly' | 'monthly' | 'yearly';
export type BedwarsMode = 'ALL_MODES' | 'SOLO' | 'DOUBLES' | 'TRIPLES' | 'QUAD';
export type Network = 'pikanetwork' | 'jartexnetwork';

export interface PikaStatEntry {
  place?: number;
  value: string | number;
}

export interface PikaStatGroup {
  entries: PikaStatEntry[] | null;
}

export interface PikaBedwarsStats {
  Wins?: PikaStatGroup;
  Losses?: PikaStatGroup;
  Kills?: PikaStatGroup;
  Deaths?: PikaStatGroup;
  'Final kills'?: PikaStatGroup;
  'Final deaths'?: PikaStatGroup;
  'Beds destroyed'?: PikaStatGroup;
  'Highest winstreak reached'?: PikaStatGroup;
  'Games played'?: PikaStatGroup;
  'Bow kills'?: PikaStatGroup;
  'Melee kills'?: PikaStatGroup;
  'Void kills'?: PikaStatGroup;
}

export interface PikaClan {
  name: string;
  tag: string;
  color?: string;
}

export interface PikaLevelInfo {
  level: number;
  experience: number;
  percentage: number;
  rankDisplay: string;
}

export interface PikaRankEntry {
  name: string;
  displayName: string;
  server: string;
  season: string | null;
  expiry: number;
}

export interface PikaProfile {
  username: string;
  ranks: PikaRankEntry[];
  rank: PikaLevelInfo | null;
  clan: PikaClan | null;
  lastSeen?: number;
  online?: boolean;
}

export type PlayerSource = 'manual' | 'auto';
export type LogFilePreset =
  | 'STANDARD'
  | 'LUNAR_CLIENT'
  | 'TLAUNCHER'
  | 'SILENT_CLIENT'
  | 'FEATHER_CLIENT'
  | 'SK_CLIENT'
  | 'CM_CLIENT'
  | 'SALWYRR'
  | 'BADLION_CLIENT'
  | 'PVPLOUNGE'
  | 'CUSTOM';

export interface Player {
  name: string;
  realName: string;
  uuid: string | null;
  loading: boolean;
  error: 'not_found' | 'rate_limited' | 'network' | 'stats_disabled' | null;
  nicked: boolean;
  profile: PikaProfile | null;
  stats: PikaBedwarsStats | null;
  source: PlayerSource;
  team: string | null;
  teamColor: string | null;
}

export interface Nick {
  id: string;
  nick: string;
  realName: string;
}

export enum Column {
  NAME = 'NAME',
  LEVEL = 'LEVEL',
  FKDR = 'FKDR',
  WLR = 'WLR',
  WINS = 'WINS',
  LOSSES = 'LOSSES',
  FINAL_KILLS = 'FINAL_KILLS',
  FINAL_DEATHS = 'FINAL_DEATHS',
  KILLS = 'KILLS',
  DEATHS = 'DEATHS',
  KDR = 'KDR',
  BEDS_BROKEN = 'BEDS_BROKEN',
  BBLR = 'BBLR',
  WIN_STREAK = 'WIN_STREAK',
  PLAYED = 'PLAYED',
}

export function statVal(group?: PikaStatGroup | null): number {
  const raw = group?.entries?.[0]?.value;
  if (raw === undefined || raw === null) return 0;
  return typeof raw === 'number' ? raw : Number(raw) || 0;
}

export function ratio(a: number, b: number): number {
  return b === 0 ? a : a / b;
}

export function fmt(value: number): string {
  if (!Number.isFinite(value) || value === 0) return '0';
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(2)}M`;
  if (Number.isInteger(value)) return value.toLocaleString();
  return value.toFixed(2);
}

const PikaRankPriority: string[] = [
  'owner',
  'developer',
  'manager',
  'admin',
  'srmod',
  'moderator',
  'helper',
  'trial',
  'youtuber',
  'partner',
  'games4',
  'games3',
  'games2',
  'games1',
];

const JartexRankPriority: string[] = [
  'owner',
  'manager',
  'developer',
  'srmod',
  'moderator',
  'jrmod',
  'helper',
  'trial',
  'youtube',
  'youtuber',
  'partner',
  'games4',
  'games3',
  'games2',
  'games1',
];

const PartnerColor = '#AA00AA';

export function normalizeProfile(profile: PikaProfile | null): PikaProfile | null {
  if (!profile) return profile;
  if (!profile.rank?.rankDisplay?.includes('Partner')) return profile;
  if (profile.ranks.some((rank) => rank.name === 'partner')) return profile;
  return {
    ...profile,
    ranks: [
      ...profile.ranks,
      { name: 'partner', displayName: 'Partner', server: '', season: null, expiry: -1 },
    ],
  };
}

const PikaStaffSet = new Set([
  'owner',
  'developer',
  'manager',
  'admin',
  'srmod',
  'moderator',
  'helper',
  'trial',
]);

const JartexStaffSet = new Set([
  'owner',
  'manager',
  'developer',
  'srmod',
  'moderator',
  'jrmod',
  'helper',
  'trial',
]);

const JartexRankColors: Record<string, string> = {
  owner: '#ff8800',
  manager: '#ff242b',
  developer: '#ffcc00',
  srmod: '#5555ff',
  moderator: '#00AAAA',
  jrmod: '#1dced5',
  helper: '#00AA00',
  trial: '#32cd32',
  youtube: '#FF5555',
  youtuber: '#FF5555',
  partner: PartnerColor,
  games4: '#FF55FF',
  games3: '#55FFFF',
  games2: '#FFFF55',
  games1: '#FFFFFF',
};

function tryGetConfigStore(): {
  network: Network;
  theme: { colors: Record<string, string> };
} | null {
  try {
    const pinia = getActivePinia();
    if (!pinia) return null;
    return (
      (pinia.state.value.config as {
        network: Network;
        theme: { colors: Record<string, string> };
      }) ?? null
    );
  } catch {
    return null;
  }
}

function getActiveNetwork(): Network {
  return tryGetConfigStore()?.network ?? 'pikanetwork';
}

function getActivePriority(): string[] {
  return getActiveNetwork() === 'jartexnetwork' ? JartexRankPriority : PikaRankPriority;
}

function getActiveStaffSet(): Set<string> {
  return getActiveNetwork() === 'jartexnetwork' ? JartexStaffSet : PikaStaffSet;
}

function getRankColorMap(): Record<string, string> {
  const config = tryGetConfigStore();
  if (!config) {
    return {
      owner: '#BC4141',
      developer: '#FF5555',
      manager: '#AA0000',
      admin: '#FF5555',
      srmod: '#00AAAA',
      moderator: '#00AA00',
      helper: '#5555FF',
      trial: '#55FFFF',
      youtuber: '#FF5555',
      partner: PartnerColor,
      games4: '#FF5555',
      games3: '#FFD700',
      games2: '#55FFFF',
      games1: '#55FF55',
    };
  }
  if (config.network === 'jartexnetwork') return JartexRankColors;
  const c = config.theme.colors;
  return {
    owner: c.rankOwner,
    developer: c.rankDeveloper,
    manager: c.rankManager,
    admin: c.rankAdmin,
    srmod: c.rankSrmod,
    moderator: c.rankModerator,
    helper: c.rankHelper,
    trial: c.rankTrial,
    youtuber: c.rankYoutuber,
    partner: PartnerColor,
    games4: c.rankChampion,
    games3: c.rankTitan,
    games2: c.rankElite,
    games1: c.rankVip,
  };
}

export function getRankSortIndex(profile: PikaProfile | null): number {
  const priority = getActivePriority();
  const ranks = profile?.ranks ?? [];
  if (ranks.length === 0) return priority.length;
  for (let i = 0; i < priority.length; i++) {
    if (ranks.some((rank) => rank.name === priority[i])) return i;
  }
  return priority.length;
}

export function getTopRankName(profile: PikaProfile | null): string | null {
  const priority = getActivePriority();
  const ranks = profile?.ranks ?? [];
  if (ranks.length === 0) return null;
  for (const name of priority) {
    if (ranks.some((rank) => rank.name === name)) return name;
  }
  return null;
}

const PikaDisplayOverrides: Record<string, string> = {
  srmod: 'Sr. Mod',
  games1: 'VIP',
};

const JartexDisplayOverrides: Record<string, string> = {
  jrmod: 'Jr. Mod',
  srmod: 'Sr. Mod',
};

const DisplayOverrides: Record<string, Record<string, string>> = {
  pikanetwork: PikaDisplayOverrides,
  jartexnetwork: JartexDisplayOverrides,
};

export function getTopRankDisplay(profile: PikaProfile | null): string | null {
  const priority = getActivePriority();
  const overrides = DisplayOverrides[getActiveNetwork()] ?? {};
  const ranks = profile?.ranks ?? [];
  if (ranks.length === 0) return null;
  for (const name of priority) {
    const entry = ranks.find((rank) => rank.name === name);
    if (entry) return overrides[name] ?? entry.displayName;
  }
  return null;
}

export function playerNameColor(profile: PikaProfile | null): string {
  const rankName = getTopRankName(profile);
  if (rankName === null) return '#AAAAAA';
  return getRankColorMap()[rankName] ?? '#AAAAAA';
}

export function isStaff(profile: PikaProfile | null): boolean {
  const ranks = profile?.ranks ?? [];
  if (ranks.length === 0) return false;
  return ranks.some((rank) => getActiveStaffSet().has(rank.name));
}

export const StatColors = [
  '#4b5563',
  '#94a3b8',
  '#34d399',
  '#22d3ee',
  '#fbbf24',
  '#f97316',
  '#f87171',
  '#e879f9',
] as const;

export function colorIndex(value: number, thresholds: readonly number[]): number {
  let matchedIndex = 0;
  for (let i = 0; i < thresholds.length; i++) {
    if (value >= thresholds[i]) matchedIndex = i;
    else break;
  }
  return matchedIndex;
}

export function statColor(value: number, thresholds: readonly number[]): string {
  return StatColors[colorIndex(value, thresholds)];
}

export interface ColumnDef {
  label: string;
  shortLabel: string;
  sortable: boolean;
  fromProfile?: boolean;
  thresholds?: readonly number[];
  getNum?: (player: Player) => number;
  getStr?: (player: Player) => string | null;
  getColor?: (player: Player) => string;
}

export function levelColor(level: number): string {
  if (level >= 200) return '#ff5555';
  if (level >= 160) return '#55ffff';
  if (level >= 120) return '#55ff55';
  if (level >= 100) return '#ff5555';
  if (level >= 75) return '#ffff55';
  if (level >= 60) return '#ffaa00';
  if (level >= 50) return '#ff55ff';
  if (level >= 45) return '#55ffff';
  if (level >= 40) return '#55ff55';
  if (level >= 35) return '#ffffff';
  if (level >= 30) return '#ff5555';
  if (level >= 25) return '#ffff55';
  if (level >= 20) return '#ffaa00';
  if (level >= 15) return '#ff55ff';
  if (level >= 10) return '#55ffff';
  if (level >= 5) return '#55ff55';
  return '#aaaaaa';
}

export const COLUMNS: Record<Column, ColumnDef> = {
  [Column.NAME]: {
    label: 'Name',
    shortLabel: 'N',
    sortable: true,
    getNum: (player) => getRankSortIndex(player.profile),
    getStr: (player) => player.realName || player.name,
  },
  [Column.LEVEL]: {
    label: 'Level',
    shortLabel: 'LVL',
    sortable: true,
    fromProfile: true,
    getNum: (player) => player.profile?.rank?.level ?? 0,
    getColor: (player) => levelColor(player.profile?.rank?.level ?? 0),
  },
  [Column.FKDR]: {
    label: 'FKDR',
    shortLabel: 'FK',
    sortable: true,
    thresholds: [0, 1, 2, 4, 7, 12, 20, 35],
    getNum: (player) =>
      ratio(
        statVal(player.stats?.['Final kills']),
        statVal(player.stats?.['Final deaths']),
      ),
  },
  [Column.WINS]: {
    label: 'Wins',
    shortLabel: 'W',
    sortable: true,
    thresholds: [0, 50, 200, 500, 1000, 2500, 5000, 10000],
    getNum: (player) => statVal(player.stats?.Wins),
  },
  [Column.LOSSES]: {
    label: 'Losses',
    shortLabel: 'L',
    sortable: true,
    thresholds: [0, 50, 200, 500, 1000, 2500, 5000, 10000],
    getNum: (player) => statVal(player.stats?.Losses),
  },
  [Column.WLR]: {
    label: 'WLR',
    shortLabel: 'WL',
    sortable: true,
    thresholds: [0, 0.5, 1, 2, 3.5, 5, 8, 12],
    getNum: (player) => ratio(statVal(player.stats?.Wins), statVal(player.stats?.Losses)),
  },
  [Column.FINAL_KILLS]: {
    label: 'FK',
    shortLabel: 'FK',
    sortable: true,
    thresholds: [0, 100, 500, 1500, 3000, 7500, 15000, 30000],
    getNum: (player) => statVal(player.stats?.['Final kills']),
  },
  [Column.FINAL_DEATHS]: {
    label: 'FD',
    shortLabel: 'FD',
    sortable: true,
    thresholds: [0, 100, 500, 1500, 3000, 7500, 15000, 30000],
    getNum: (player) => statVal(player.stats?.['Final deaths']),
  },
  [Column.KILLS]: {
    label: 'Kills',
    shortLabel: 'K',
    sortable: true,
    thresholds: [0, 200, 1000, 3000, 7500, 15000, 30000, 60000],
    getNum: (player) => statVal(player.stats?.Kills),
  },
  [Column.DEATHS]: {
    label: 'Deaths',
    shortLabel: 'D',
    sortable: true,
    thresholds: [0, 200, 1000, 3000, 7500, 15000, 30000, 60000],
    getNum: (player) => statVal(player.stats?.Deaths),
  },
  [Column.KDR]: {
    label: 'KDR',
    shortLabel: 'KD',
    sortable: true,
    thresholds: [0, 1, 2, 4, 7, 12, 20, 35],
    getNum: (player) =>
      ratio(statVal(player.stats?.Kills), statVal(player.stats?.Deaths)),
  },
  [Column.BEDS_BROKEN]: {
    label: 'BB',
    shortLabel: 'BB',
    sortable: true,
    thresholds: [0, 50, 200, 500, 1200, 2500, 5000, 10000],
    getNum: (player) => statVal(player.stats?.['Beds destroyed']),
  },
  [Column.BBLR]: {
    label: 'BBLR',
    shortLabel: 'BB',
    sortable: true,
    thresholds: [0, 0.5, 1, 2, 3.5, 5, 8, 12],
    getNum: (player) =>
      ratio(statVal(player.stats?.['Beds destroyed']), statVal(player.stats?.Losses)),
  },
  [Column.WIN_STREAK]: {
    label: 'WS',
    shortLabel: 'WS',
    sortable: true,
    thresholds: [0, 3, 7, 15, 30, 50, 80, 120],
    getNum: (player) => statVal(player.stats?.['Highest winstreak reached']),
  },
  [Column.PLAYED]: {
    label: 'Played',
    shortLabel: 'GP',
    sortable: true,
    thresholds: [0, 100, 500, 1500, 3000, 7000, 15000, 30000],
    getNum: (player) => statVal(player.stats?.['Games played']),
  },
};

export const NETWORKS: { value: Network; label: string }[] = [
  { value: 'pikanetwork', label: 'PikaNetwork' },
  { value: 'jartexnetwork', label: 'JartexNetwork' },
];

export interface ProxyStatusEntry {
  running: boolean;
  port: number;
  bindHost: string;
  clientCount: number;
  error: string | null;
}

export interface ProxyStatusAll {
  pika: ProxyStatusEntry;
  jartex: ProxyStatusEntry;
}

export type ProxyEventPayload =
  | { type: 'player-join'; network: string; username: string }
  | { type: 'player-quit'; network: string; username: string }
  | { type: 'teams-update'; network: string; teams: TeamInfo[] }
  | { type: 'client-connect'; network: string; clientName: string }
  | { type: 'client-disconnect'; network: string; clientName: string }
  | {
      type: 'auth-code';
      network: string;
      userCode: string;
      verificationUri: string;
      expiresInSeconds: number;
    }
  | { type: 'auth-success'; network: string }
  | { type: 'auth-error'; network: string; message: string }
  | {
      type: 'status';
      network: string;
      running: boolean;
      port: number;
      error: string | null;
    };

export type TelemetryEventPayload =
  { type: 'linking' } | { type: 'linked' } | { type: 'error'; message: string };

export interface SupportConversationSummary {
  id: string;
  subject: string;
  status: string;
  createdAt: number;
  updatedAt: string | null;
}

export interface SupportMessage {
  id: string;
  body: string;
  fromSupport: boolean;
  senderName: string | null;
  senderAvatarUrl: string | null;
  createdAt: string;
}

export interface SupportConversationDetail {
  id: string;
  subject: string;
  status: string;
  messages: SupportMessage[];
}

export type SupportSocketEvent =
  | { type: 'connected' }
  | { type: 'disconnected' }
  | { type: 'message'; conversationId: string; message: SupportMessage };

export type SupportResult<T> = { ok: true; data: T } | { ok: false; error: string };

export interface TeamInfo {
  name: string;
  displayName: string;
  color: string;
  players: string[];
}

declare global {
  interface Window {
    api: {
      platform: string;
      pika: {
        fetch: (
          username: string,
          interval?: string,
          mode?: string,
        ) => Promise<{
          profile: PikaProfile | null;
          stats: PikaBedwarsStats | null;
          notFound?: boolean;
          rateLimit?: boolean;
        }>;
        stats: (
          username: string,
          interval: string,
          mode: string,
        ) => Promise<PikaBedwarsStats | null>;
        clan: (name: string) => Promise<unknown>;
      };
      jartex: {
        fetch: (
          username: string,
          interval?: string,
          mode?: string,
        ) => Promise<{
          profile: PikaProfile | null;
          stats: PikaBedwarsStats | null;
          notFound?: boolean;
          rateLimit?: boolean;
        }>;
        stats: (
          username: string,
          interval: string,
          mode: string,
        ) => Promise<PikaBedwarsStats | null>;
        clan: (name: string) => Promise<unknown>;
      };
      win: {
        minimize: () => void;
        close: () => void;
        toggleMinimize: () => void;
        openExternal: (url: string) => void;
        screenshot: () => Promise<void>;
        fitContentWidth: (width: number) => void;
        setAlwaysOnTop: (enabled: boolean) => void;
        setIgnoreMouse: (ignore: boolean) => void;
        onForwardedMove: (
          callback: (x: number | null, y: number | null) => void,
        ) => () => void;
      };
      app: {
        getPath: (name: string) => Promise<string>;
        findLunarLog: () => Promise<string>;
        openImageDialog: () => Promise<{
          canceled: boolean;
          filePaths: string[];
          bookmarks?: string[];
        }>;
        readFileBase64: (filePath: string) => Promise<string>;
      };
      log: {
        setPath: (path: string | null) => void;
        checkPath: (path: string) => Promise<boolean>;
        openDialog: () => Promise<{ canceled: boolean; filePaths: string[] }>;
        onLine: (callback: (line: string) => void) => () => void;
      };
      shortcuts: {
        register: (shortcuts: string[]) => Promise<void>;
        onFired: (callback: (shortcut: string) => void) => () => void;
      };
      rpc: {
        setEnabled: (enabled: boolean) => void;
        setActive: (active: boolean) => void;
        setNetwork: (network: string) => void;
        destroy: () => void;
      };
      updater: {
        check: () => void;
        install: () => void;
        onStatus: (
          callback: (payload: {
            status: string;
            version?: string;
            percent?: number;
            error?: string;
          }) => void,
        ) => () => void;
      };
      proxy: {
        getStatus: () => Promise<ProxyStatusAll | null>;
        setPort: (
          network: 'pikanetwork' | 'jartexnetwork',
          port: number,
        ) => Promise<void>;
        setBindHost: (bindHost: '0.0.0.0' | '127.0.0.1') => Promise<void>;
        onEvent: (callback: (event: ProxyEventPayload) => void) => () => void;
      };
      telemetry: {
        isLinked: () => Promise<boolean>;
        startLink: () => void;
        onEvent: (callback: (event: TelemetryEventPayload) => void) => () => void;
      };
      support: {
        list: () => Promise<
          SupportResult<{ conversations: SupportConversationSummary[] }>
        >;
        create: (
          subject: string,
          message: string,
        ) => Promise<SupportResult<SupportConversationSummary>>;
        get: (id: string) => Promise<SupportResult<SupportConversationDetail>>;
        reply: (id: string, message: string) => Promise<SupportResult<SupportMessage>>;
        connectSocket: () => void;
        disconnectSocket: () => void;
        onSocketEvent: (callback: (event: SupportSocketEvent) => void) => () => void;
      };
      perf: {
        dump: (rendererSnapshot: unknown) => Promise<string>;
        startTrace: () => Promise<boolean>;
        stopTrace: () => Promise<string | null>;
        openLogDir: () => Promise<string>;
      };
    };
  }
}
