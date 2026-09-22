<script setup lang="ts">
import type {
  PikaBedwarsStats,
  PikaProfile,
  PikaStatGroup,
  Player,
} from '@renderer/types';
import { Button } from '@renderer/components/ui/button';
import { useConfigStore } from '@renderer/store/config';
import { usePlayersStore } from '@renderer/store/players';
import { Column } from '@renderer/types';
import { Activity, RefreshCw } from 'lucide-vue-next';
import { computed, markRaw, nextTick, onUnmounted, ref } from 'vue';

const players = usePlayersStore();
const config = useConfigStore();

type PhaseStatus = 'idle' | 'running' | 'done' | 'error';

interface MetricRow {
  label: string;
  value: string;
  unit: string;
  rating: 'good' | 'warn' | 'bad' | 'neutral';
}

interface PhaseResult {
  id: string;
  label: string;
  status: PhaseStatus;
  metrics: MetricRow[];
  durationMs: number;
}

const phases = ref<PhaseResult[]>([]);
const running = ref(false);
const overallRating = ref<'good' | 'warn' | 'bad' | null>(null);
const savedPlayers = ref<Player[]>([]);
const rounds = 100;

function stat(value: number): PikaStatGroup {
  return { entries: [{ value }] };
}

function createProfile(rankName: string, level: number): PikaProfile {
  return {
    username: 'BenchPlayer',
    ranks: [
      {
        name: rankName,
        displayName: rankName.toUpperCase(),
        server: 'BED',
        season: null,
        expiry: 0,
      },
    ],
    rank: {
      level,
      experience: level * 1000,
      percentage: 0.5,
      rankDisplay: `[${level}✫]`,
    },
    clan: level > 50 ? { name: 'BenchClan', tag: '⚡BC' } : null,
    lastSeen: Date.now(),
    online: false,
  };
}

function createStats(
  finalKills: number,
  finalDeaths: number,
  wins: number,
  losses: number,
  kills: number,
  deaths: number,
): PikaBedwarsStats {
  return {
    'Final kills': stat(finalKills),
    'Final deaths': stat(finalDeaths),
    Wins: stat(wins),
    Losses: stat(losses),
    Kills: stat(kills),
    Deaths: stat(deaths),
    'Beds destroyed': stat(Math.floor(wins * 1.3)),
    'Highest winstreak reached': stat(Math.floor(wins / 20)),
    'Games played': stat(wins + losses),
  };
}

const mock: Player[] = [
  {
    name: 'Technoblade',
    realName: 'Technoblade',
    uuid: 'a',
    loading: false,
    error: null,
    nicked: false,
    source: 'auto',
    team: 'Red',
    teamColor: '#ff4455',
    profile: createProfile('srmod', 200),
    stats: createStats(82000, 1200, 15000, 800, 120000, 9000),
  },
  {
    name: 'Dream',
    realName: 'Dream',
    uuid: 'b',
    loading: false,
    error: null,
    nicked: false,
    source: 'auto',
    team: 'Red',
    teamColor: '#ff4455',
    profile: createProfile('moderator', 150),
    stats: createStats(45000, 3200, 8000, 1200, 70000, 12000),
  },
  {
    name: 'Skeppy',
    realName: 'Skeppy',
    uuid: 'c',
    loading: false,
    error: null,
    nicked: false,
    source: 'auto',
    team: 'Blue',
    teamColor: '#4488ff',
    profile: createProfile('helper', 100),
    stats: createStats(12000, 8000, 2000, 2500, 25000, 18000),
  },
  {
    name: 'BadBoyHalo',
    realName: 'BadBoyHalo',
    uuid: 'd',
    loading: false,
    error: null,
    nicked: false,
    source: 'auto',
    team: 'Blue',
    teamColor: '#4488ff',
    profile: createProfile('trial', 75),
    stats: createStats(5000, 6000, 800, 1800, 9000, 14000),
  },
  {
    name: 'xNestorio',
    realName: 'xNestorio',
    uuid: 'e',
    loading: false,
    error: null,
    nicked: true,
    source: 'manual',
    team: 'Green',
    teamColor: '#44dd66',
    profile: createProfile('youtuber', 120),
    stats: createStats(30000, 4000, 6000, 2000, 48000, 15000),
  },
  {
    name: 'n1ck3d',
    realName: 'GoodPlayer',
    uuid: 'f',
    loading: false,
    error: null,
    nicked: true,
    source: 'auto',
    team: 'Green',
    teamColor: '#44dd66',
    profile: createProfile('games4', 88),
    stats: createStats(18000, 5000, 3200, 1500, 32000, 13000),
  },
  {
    name: 'CrystalLegend',
    realName: 'CrystalLegend',
    uuid: 'g',
    loading: false,
    error: null,
    nicked: false,
    source: 'auto',
    team: 'Yellow',
    teamColor: '#ffdd33',
    profile: createProfile('games3', 60),
    stats: createStats(4000, 7000, 500, 2200, 7000, 16000),
  },
  {
    name: 'NovicePlayer99',
    realName: 'NovicePlayer99',
    uuid: 'h',
    loading: false,
    error: null,
    nicked: false,
    source: 'auto',
    team: 'Yellow',
    teamColor: '#ffdd33',
    profile: null,
    stats: createStats(120, 900, 10, 400, 300, 2200),
  },
  {
    name: 'ShadowBlade',
    realName: 'ShadowBlade',
    uuid: 'i',
    loading: false,
    error: null,
    nicked: false,
    source: 'auto',
    team: null,
    teamColor: null,
    profile: createProfile('admin', 180),
    stats: createStats(60000, 2100, 11000, 900, 90000, 8000),
  },
  {
    name: 'PrivateStats',
    realName: 'PrivateStats',
    uuid: 'j',
    loading: false,
    error: 'stats_disabled',
    nicked: false,
    source: 'auto',
    team: null,
    teamColor: null,
    profile: createProfile('games2', 45),
    stats: null,
  },
  {
    name: 'NotFound404',
    realName: 'NotFound404',
    uuid: 'k',
    loading: false,
    error: 'not_found',
    nicked: false,
    source: 'auto',
    team: null,
    teamColor: null,
    profile: null,
    stats: null,
  },
  {
    name: 'RateLimited',
    realName: 'RateLimited',
    uuid: 'l',
    loading: false,
    error: 'rate_limited',
    nicked: false,
    source: 'auto',
    team: null,
    teamColor: null,
    profile: null,
    stats: null,
  },
  {
    name: 'Loading1',
    realName: 'Loading1',
    uuid: 'm',
    loading: true,
    error: null,
    nicked: false,
    source: 'auto',
    team: null,
    teamColor: null,
    profile: null,
    stats: null,
  },
  {
    name: 'Loading2',
    realName: 'Loading2',
    uuid: 'n',
    loading: true,
    error: null,
    nicked: false,
    source: 'auto',
    team: null,
    teamColor: null,
    profile: null,
    stats: null,
  },
  {
    name: 'MidPlayer',
    realName: 'MidPlayer',
    uuid: 'o',
    loading: false,
    error: null,
    nicked: false,
    source: 'auto',
    team: null,
    teamColor: null,
    profile: createProfile('games1', 30),
    stats: createStats(800, 1200, 120, 380, 2200, 4100),
  },
  {
    name: 'NetworkErr',
    realName: 'NetworkErr',
    uuid: 'p',
    loading: false,
    error: 'network',
    nicked: false,
    source: 'auto',
    team: null,
    teamColor: null,
    profile: null,
    stats: null,
  },
];

function rateMs(ms: number, good: number, warn: number): MetricRow['rating'] {
  if (ms <= good) return 'good';
  if (ms <= warn) return 'warn';
  return 'bad';
}

function rateFps(fps: number): MetricRow['rating'] {
  if (fps >= 58) return 'good';
  if (fps >= 45) return 'warn';
  return 'bad';
}

function rateJank(pct: number): MetricRow['rating'] {
  if (pct <= 2) return 'good';
  if (pct <= 8) return 'warn';
  return 'bad';
}

function percentile(sorted: number[], percentileRank: number): number {
  const index = Math.max(0, Math.ceil((percentileRank / 100) * sorted.length) - 1);
  return sorted[index];
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function waitFrames(frameCount: number): Promise<void> {
  return new Promise((resolve) => {
    let count = 0;
    function tick() {
      if (++count >= frameCount) resolve();
      else requestAnimationFrame(tick);
    }
    requestAnimationFrame(tick);
  });
}

function addPhase(id: string, label: string): PhaseResult {
  const phase: PhaseResult = { id, label, status: 'running', metrics: [], durationMs: 0 };
  phases.value.push(phase);
  return phase;
}

function finishPhase(phase: PhaseResult, startTime: number): void {
  phase.durationMs = performance.now() - startTime;
  phase.status = 'done';
}

async function runFrameTimingTest(): Promise<PhaseResult> {
  const phase = addPhase('frametiming', 'Frame Timing');
  const start = performance.now();
  const sampleDuration = 750;
  const frames: number[] = [];
  let last = performance.now();

  await new Promise<void>((resolve) => {
    const deadline = performance.now() + sampleDuration;
    function tick() {
      const now = performance.now();
      frames.push(now - last);
      last = now;
      if (now < deadline) requestAnimationFrame(tick);
      else resolve();
    }
    requestAnimationFrame(tick);
  });

  frames.shift();
  const sorted = [...frames].sort((a, b) => a - b);
  const mean = frames.reduce((sum, duration) => sum + duration, 0) / frames.length;
  const avgFps = 1000 / mean;
  const p50 = percentile(sorted, 50);
  const p95 = percentile(sorted, 95);
  const p99 = percentile(sorted, 99);
  const jankFrames = frames.filter((frame) => frame > 33.33).length;
  const jankPct = (jankFrames / frames.length) * 100;
  const maxFrame = sorted[sorted.length - 1];

  phase.metrics = [
    {
      label: 'Average FPS',
      value: avgFps.toFixed(1),
      unit: 'fps',
      rating: rateFps(avgFps),
    },
    {
      label: 'Mean Frame',
      value: mean.toFixed(2),
      unit: 'ms',
      rating: rateMs(mean, 16.67, 22),
    },
    {
      label: 'p50 Frame',
      value: p50.toFixed(2),
      unit: 'ms',
      rating: rateMs(p50, 16.67, 22),
    },
    {
      label: 'p95 Frame',
      value: p95.toFixed(2),
      unit: 'ms',
      rating: rateMs(p95, 20, 33),
    },
    {
      label: 'p99 Frame',
      value: p99.toFixed(2),
      unit: 'ms',
      rating: rateMs(p99, 25, 50),
    },
    {
      label: 'Max Frame',
      value: maxFrame.toFixed(2),
      unit: 'ms',
      rating: rateMs(maxFrame, 33, 66),
    },
    {
      label: 'Jank Frames (>33ms)',
      value: jankFrames.toString(),
      unit: `/ ${frames.length}`,
      rating: rateJank(jankPct),
    },
    {
      label: 'Jank Rate',
      value: jankPct.toFixed(2),
      unit: '%',
      rating: rateJank(jankPct),
    },
    {
      label: 'Samples',
      value: frames.length.toString(),
      unit: 'frames',
      rating: 'neutral',
    },
  ];

  finishPhase(phase, start);
  return phase;
}

async function runLongTaskTest(): Promise<PhaseResult> {
  const phase = addPhase('longtask', 'Long Task Detection');
  const start = performance.now();
  const longTasks: PerformanceEntry[] = [];

  let observer: PerformanceObserver | null = null;
  const supported =
    'PerformanceObserver' in window &&
    PerformanceObserver.supportedEntryTypes?.includes('longtask');

  if (supported) {
    observer = new PerformanceObserver((list) => {
      longTasks.push(...list.getEntries());
    });
    observer.observe({ entryTypes: ['longtask'] });
  }

  players.players = markRaw([...mock]);
  await waitFrames(60);

  for (let i = 0; i < 5; i++) {
    const sorts = [Column.FKDR, Column.WINS, Column.LEVEL, Column.WLR, Column.FKDR];
    config.sortBy = sorts[i];
    await waitFrames(8);
  }

  await waitFrames(30);
  observer?.disconnect();

  const totalDur = longTasks.reduce((sum, entry) => sum + entry.duration, 0);
  const maxTask = longTasks.length
    ? Math.max(...longTasks.map((entry) => entry.duration))
    : 0;

  phase.metrics = [
    {
      label: 'Long Tasks Detected',
      value: longTasks.length.toString(),
      unit: 'tasks',
      rating: longTasks.length === 0 ? 'good' : longTasks.length <= 2 ? 'warn' : 'bad',
    },
    {
      label: 'Total Blocked Time',
      value: totalDur.toFixed(1),
      unit: 'ms',
      rating: rateMs(totalDur, 0, 50),
    },
    {
      label: 'Max Single Task',
      value: maxTask.toFixed(1),
      unit: 'ms',
      rating: rateMs(maxTask, 0, 50),
    },
    {
      label: 'Observer Support',
      value: supported ? 'Yes' : 'No',
      unit: '',
      rating: supported ? 'neutral' : 'warn',
    },
  ];

  finishPhase(phase, start);
  return phase;
}

async function runPlayerRenderTest(): Promise<PhaseResult> {
  const phase = addPhase('playerrender', 'Player List Render');
  const start = performance.now();

  players.players = [];
  await waitFrames(4);

  const t0 = performance.now();
  players.players = markRaw([...mock]);
  await waitFrames(1);
  const firstFrameMs = performance.now() - t0;

  await waitFrames(10);

  const t1 = performance.now();
  config.sortBy = Column.FKDR;
  config.sortAscending = false;
  await waitFrames(1);
  const sortRenderMs = performance.now() - t1;

  const t2 = performance.now();
  config.sortBy = Column.LEVEL;
  await waitFrames(1);
  const sort2Ms = performance.now() - t2;

  const t3 = performance.now();
  for (let i = 0; i < players.players.length; i++) {
    const player = players.players[i];
    players.players[i] = {
      ...player,
      stats: player.stats
        ? {
            ...player.stats,
            'Final kills': stat(
              ((player.stats['Final kills']?.entries?.[0]?.value as number) ?? 0) + 1,
            ),
          }
        : player.stats,
    };
  }
  await waitFrames(2);
  const batchUpdateMs = performance.now() - t3;

  const t4 = performance.now();
  players.players = markRaw([...mock].map((player) => ({ ...player, loading: true })));
  await waitFrames(1);
  const loadingStateMs = performance.now() - t4;

  players.players = markRaw([...mock]);
  await waitFrames(4);

  phase.metrics = [
    {
      label: '16-Player First Paint',
      value: firstFrameMs.toFixed(2),
      unit: 'ms',
      rating: rateMs(firstFrameMs, 8, 20),
    },
    {
      label: 'Sort Re-render (FKDR)',
      value: sortRenderMs.toFixed(2),
      unit: 'ms',
      rating: rateMs(sortRenderMs, 4, 12),
    },
    {
      label: 'Sort Re-render (Level)',
      value: sort2Ms.toFixed(2),
      unit: 'ms',
      rating: rateMs(sort2Ms, 4, 12),
    },
    {
      label: 'Batch Stat Update (16)',
      value: batchUpdateMs.toFixed(2),
      unit: 'ms',
      rating: rateMs(batchUpdateMs, 5, 16),
    },
    {
      label: 'Loading State Toggle',
      value: loadingStateMs.toFixed(2),
      unit: 'ms',
      rating: rateMs(loadingStateMs, 3, 10),
    },
  ];

  finishPhase(phase, start);
  return phase;
}

async function runAnimationStressTest(): Promise<PhaseResult> {
  const phase = addPhase('animation', 'Animation Stress Test');
  const start = performance.now();

  const sampleDuration = 600;
  const frames: number[] = [];
  let last = performance.now();

  await new Promise<void>((resolve) => {
    const deadline = performance.now() + sampleDuration;
    function tick() {
      const now = performance.now();
      frames.push(now - last);
      last = now;
      if (now < deadline) requestAnimationFrame(tick);
      else resolve();
    }
    requestAnimationFrame(tick);
  });

  frames.shift();
  const sorted = [...frames].sort((a, b) => a - b);
  const mean = frames.reduce((sum, value) => sum + value, 0) / frames.length;
  const p95 = percentile(sorted, 95);
  const p99 = percentile(sorted, 99);
  const dropped60 = frames.filter((frame) => frame > 16.67).length;
  const dropped30 = frames.filter((frame) => frame > 33.33).length;
  const droppedPct = (dropped60 / frames.length) * 100;
  const nickedCount = mock.filter((player) => player.nicked).length;

  phase.metrics = [
    {
      label: 'Avg FPS Under Load',
      value: (1000 / mean).toFixed(1),
      unit: 'fps',
      rating: rateFps(1000 / mean),
    },
    {
      label: 'Mean Frame Time',
      value: mean.toFixed(2),
      unit: 'ms',
      rating: rateMs(mean, 16.67, 22),
    },
    {
      label: 'p95 Frame Time',
      value: p95.toFixed(2),
      unit: 'ms',
      rating: rateMs(p95, 20, 33),
    },
    {
      label: 'p99 Frame Time',
      value: p99.toFixed(2),
      unit: 'ms',
      rating: rateMs(p99, 25, 50),
    },
    {
      label: 'Frames >16.67ms (60fps)',
      value: dropped60.toString(),
      unit: `/ ${frames.length}`,
      rating: rateJank(droppedPct),
    },
    {
      label: 'Frames >33.33ms (30fps)',
      value: dropped30.toString(),
      unit: `/ ${frames.length}`,
      rating: dropped30 === 0 ? 'good' : dropped30 < 5 ? 'warn' : 'bad',
    },
    {
      label: 'Nicked Rows (Shine)',
      value: nickedCount.toString(),
      unit: 'rows',
      rating: 'neutral',
    },
    {
      label: 'Low-End Mode',
      value: config.lowEndMode ? 'ON' : 'OFF',
      unit: '',
      rating: 'neutral',
    },
  ];

  finishPhase(phase, start);
  return phase;
}

async function runReactivityTest(): Promise<PhaseResult> {
  const phase = addPhase('reactivity', 'Reactivity Throughput');
  const start = performance.now();

  players.players = markRaw([...mock]);
  await waitFrames(4);

  const roundTimes: number[] = [];

  for (let round = 0; round < rounds; round++) {
    const roundStart = performance.now();
    const index = round % players.players.length;
    const player = players.players[index];
    players.players[index] = {
      ...player,
      stats: player.stats
        ? { ...player.stats, 'Final kills': stat(round) }
        : player.stats,
    };
    await nextTick();
    roundTimes.push(performance.now() - roundStart);
  }

  const sorted = [...roundTimes].sort((a, b) => a - b);
  const mean = roundTimes.reduce((sum, value) => sum + value, 0) / roundTimes.length;
  const p95 = percentile(sorted, 95);
  const p99 = percentile(sorted, 99);
  const max = sorted[sorted.length - 1];
  const throughput = 1000 / mean;

  const t0 = performance.now();
  players.players = markRaw(
    players.players.map((player, i) => ({
      ...player,
      stats: player.stats ? { ...player.stats, Wins: stat(i * 100) } : player.stats,
    })),
  );
  await nextTick();
  const bulkMs = performance.now() - t0;

  phase.metrics = [
    {
      label: 'Mean Update + Tick',
      value: mean.toFixed(2),
      unit: 'ms',
      rating: rateMs(mean, 1, 4),
    },
    {
      label: 'p95 Update Time',
      value: p95.toFixed(2),
      unit: 'ms',
      rating: rateMs(p95, 2, 8),
    },
    {
      label: 'p99 Update Time',
      value: p99.toFixed(2),
      unit: 'ms',
      rating: rateMs(p99, 5, 15),
    },
    {
      label: 'Max Update Time',
      value: max.toFixed(2),
      unit: 'ms',
      rating: rateMs(max, 5, 20),
    },
    {
      label: 'Update Throughput',
      value: throughput.toFixed(0),
      unit: 'updates/s',
      rating: throughput > 500 ? 'good' : throughput > 200 ? 'warn' : 'bad',
    },
    {
      label: 'Bulk 16-Player Update',
      value: bulkMs.toFixed(2),
      unit: 'ms',
      rating: rateMs(bulkMs, 5, 16),
    },
    { label: 'Rounds Measured', value: rounds.toString(), unit: '', rating: 'neutral' },
  ];

  finishPhase(phase, start);
  return phase;
}

async function runAvatarCacheTest(): Promise<PhaseResult> {
  const phase = addPhase('avatarcache', 'Avatar Cache Latency');
  const start = performance.now();

  const names = mock
    .filter((player) => !player.loading && !player.error)
    .map((player) => player.realName);

  const localStorageTimes: number[] = [];
  for (let i = 0; i < rounds; i++) {
    const name = names[i % names.length];
    const key = `mchead_${name.toLowerCase()}`;
    const sampleStart = performance.now();
    try {
      localStorage.getItem(key);
    } catch {}
    localStorageTimes.push(performance.now() - sampleStart);
  }

  const memCache = new Map<string, string>();
  names.forEach((name) =>
    memCache.set(`mchead_${name.toLowerCase()}`, `data:fake/${name}`),
  );

  const memTimes: number[] = [];
  for (let i = 0; i < rounds; i++) {
    const name = names[i % names.length];
    const key = `mchead_${name.toLowerCase()}`;
    const sampleStart = performance.now();
    memCache.get(key);
    memTimes.push(performance.now() - sampleStart);
  }

  const jsonSizes: number[] = [];
  for (const name of names) {
    const raw = localStorage.getItem(`mchead_${name.toLowerCase()}`);
    if (raw) jsonSizes.push(raw.length);
  }
  const avgJsonSize = jsonSizes.length
    ? jsonSizes.reduce((sum, value) => sum + value, 0) / jsonSizes.length
    : 0;

  const lsMean =
    localStorageTimes.reduce((sum, value) => sum + value, 0) / localStorageTimes.length;
  const memMean = memTimes.reduce((sum, value) => sum + value, 0) / memTimes.length;

  const lsSorted = [...localStorageTimes].sort((a, b) => a - b);

  const cachedCount = jsonSizes.length;

  phase.metrics = [
    {
      label: 'localStorage Mean Read',
      value: (lsMean * 1000).toFixed(1),
      unit: 'µs',
      rating: lsMean < 0.05 ? 'good' : lsMean < 0.2 ? 'warn' : 'bad',
    },
    {
      label: 'localStorage p99 Read',
      value: (percentile(lsSorted, 99) * 1000).toFixed(1),
      unit: 'µs',
      rating: 'neutral',
    },
    {
      label: 'Memory Cache Mean Read',
      value: (memMean * 1000).toFixed(2),
      unit: 'µs',
      rating: 'good',
    },
    {
      label: 'Speedup (ls → mem)',
      value: lsMean > 0 ? `${(lsMean / memMean).toFixed(0)}x` : 'N/A',
      unit: 'faster',
      rating: 'good',
    },
    {
      label: 'Avatars in localStorage',
      value: cachedCount.toString(),
      unit: `/ ${names.length}`,
      rating: 'neutral',
    },
    {
      label: 'Avg Cached Entry Size',
      value: avgJsonSize > 0 ? (avgJsonSize / 1024).toFixed(1) : '0',
      unit: 'KB',
      rating: 'neutral',
    },
    {
      label: 'Samples Per Method',
      value: rounds.toString(),
      unit: '',
      rating: 'neutral',
    },
  ];

  finishPhase(phase, start);
  return phase;
}

async function runMemoryTest(): Promise<PhaseResult> {
  const phase = addPhase('memory', 'Memory Delta');
  const start = performance.now();

  const mem = (performance as any).memory as
    | { usedJSHeapSize: number; totalJSHeapSize: number; jsHeapSizeLimit: number }
    | undefined;
  const supported = !!mem;

  if (!supported) {
    phase.metrics = [
      { label: 'performance.memory', value: 'Unavailable', unit: '', rating: 'warn' },
      {
        label: 'Note',
        value: 'Run in Chromium-based WebView',
        unit: '',
        rating: 'neutral',
      },
    ];
    finishPhase(phase, start);
    return phase;
  }

  players.players = [];
  await waitFrames(8);
  const heapBefore = mem.usedJSHeapSize;

  players.players = markRaw([...mock]);
  await waitFrames(16);
  const heapLoaded = mem.usedJSHeapSize;

  const heapDelta = heapLoaded - heapBefore;
  const heapPerPlayer = heapDelta / mock.length;

  players.players = [];
  await waitFrames(8);
  const heapAfterClear = mem.usedJSHeapSize;
  const heapRetained = heapAfterClear - heapBefore;

  phase.metrics = [
    {
      label: 'Heap Before Inject',
      value: (heapBefore / 1048576).toFixed(2),
      unit: 'MB',
      rating: 'neutral',
    },
    {
      label: 'Heap With 16 Players',
      value: (heapLoaded / 1048576).toFixed(2),
      unit: 'MB',
      rating: 'neutral',
    },
    {
      label: 'Delta 16 Players',
      value: (heapDelta / 1024).toFixed(1),
      unit: 'KB',
      rating: heapDelta < 500 * 1024 ? 'good' : heapDelta < 2 * 1048576 ? 'warn' : 'bad',
    },
    {
      label: 'Per-Player Heap Cost',
      value: (heapPerPlayer / 1024).toFixed(1),
      unit: 'KB',
      rating:
        heapPerPlayer < 40 * 1024 ? 'good' : heapPerPlayer < 100 * 1024 ? 'warn' : 'bad',
    },
    {
      label: 'Heap Retained Post-Clear',
      value: (heapRetained / 1024).toFixed(1),
      unit: 'KB',
      rating: heapRetained < 100 * 1024 ? 'good' : 'warn',
    },
    {
      label: 'Heap Limit',
      value: (mem.jsHeapSizeLimit / 1048576).toFixed(0),
      unit: 'MB',
      rating: 'neutral',
    },
    {
      label: 'Total Heap Allocated',
      value: (mem.totalJSHeapSize / 1048576).toFixed(2),
      unit: 'MB',
      rating: 'neutral',
    },
  ];

  finishPhase(phase, start);
  return phase;
}

async function runScrollTest(): Promise<PhaseResult> {
  const phase = addPhase('scroll', 'Scroll Performance');
  const start = performance.now();

  players.players = markRaw([...mock]);
  await waitFrames(8);

  const scrollEl = document.querySelector('.themed-scroll') as HTMLElement | null;

  if (!scrollEl) {
    phase.metrics = [
      { label: 'Scroll Container', value: 'Not found', unit: '', rating: 'warn' },
      { label: 'Note', value: 'Must be on Home view', unit: '', rating: 'neutral' },
    ];
    finishPhase(phase, start);
    return phase;
  }

  const passes = 3;
  const scrollTimes: number[] = [];

  for (let pass = 0; pass < passes; pass++) {
    scrollEl.scrollTop = 0;
    await waitFrames(4);

    const maxScroll = scrollEl.scrollHeight - scrollEl.clientHeight;
    const steps = 20;
    for (let step = 0; step <= steps; step++) {
      const stepStart = performance.now();
      scrollEl.scrollTop = (maxScroll / steps) * step;
      await waitFrames(1);
      scrollTimes.push(performance.now() - stepStart);
    }

    scrollEl.scrollTop = 0;
    await waitFrames(4);
  }

  const sorted = [...scrollTimes].sort((a, b) => a - b);
  const mean = scrollTimes.reduce((sum, value) => sum + value, 0) / scrollTimes.length;

  phase.metrics = [
    {
      label: 'Mean Frame Per Step',
      value: mean.toFixed(2),
      unit: 'ms',
      rating: rateMs(mean, 16.67, 22),
    },
    {
      label: 'p95 Scroll Frame',
      value: percentile(sorted, 95).toFixed(2),
      unit: 'ms',
      rating: rateMs(percentile(sorted, 95), 20, 33),
    },
    {
      label: 'p99 Scroll Frame',
      value: percentile(sorted, 99).toFixed(2),
      unit: 'ms',
      rating: rateMs(percentile(sorted, 99), 25, 50),
    },
    {
      label: 'Max Scroll Frame',
      value: sorted[sorted.length - 1].toFixed(2),
      unit: 'ms',
      rating: rateMs(sorted[sorted.length - 1], 33, 66),
    },
    { label: 'Scroll Passes', value: passes.toString(), unit: '', rating: 'neutral' },
    { label: 'Steps Per Pass', value: '20', unit: '', rating: 'neutral' },
  ];

  finishPhase(phase, start);
  return phase;
}

async function runLowEndDeltaTest(): Promise<PhaseResult> {
  const phase = addPhase('lowenddelta', 'Low-End Mode Delta');
  const start = performance.now();

  const wasLowEnd = config.lowEndMode;
  const sampleDuration = 400;

  async function sampleFps(): Promise<{ mean: number; p95: number; fps: number }> {
    const frames: number[] = [];
    let last = performance.now();
    await new Promise<void>((resolve) => {
      const deadline = performance.now() + sampleDuration;
      function tick() {
        const now = performance.now();
        frames.push(now - last);
        last = now;
        if (now < deadline) requestAnimationFrame(tick);
        else resolve();
      }
      requestAnimationFrame(tick);
    });
    frames.shift();
    const sorted = [...frames].sort((a, b) => a - b);
    const mean = frames.reduce((sum, value) => sum + value, 0) / frames.length;
    return { mean, p95: percentile(sorted, 95), fps: 1000 / mean };
  }

  config.lowEndMode = false;
  document.documentElement.classList.remove('low-end');
  players.players = markRaw([...mock]);
  await waitFrames(12);
  const normalResult = await sampleFps();

  config.lowEndMode = true;
  document.documentElement.classList.add('low-end');
  await waitFrames(12);
  const lowEndResult = await sampleFps();

  config.lowEndMode = wasLowEnd;
  document.documentElement.classList.toggle('low-end', wasLowEnd);
  await waitFrames(4);

  const fpsDelta = lowEndResult.fps - normalResult.fps;
  const frameDelta = normalResult.mean - lowEndResult.mean;

  phase.metrics = [
    {
      label: 'Normal Mode FPS',
      value: normalResult.fps.toFixed(1),
      unit: 'fps',
      rating: rateFps(normalResult.fps),
    },
    {
      label: 'Low-End Mode FPS',
      value: lowEndResult.fps.toFixed(1),
      unit: 'fps',
      rating: rateFps(lowEndResult.fps),
    },
    {
      label: 'FPS Gain (Low-End On)',
      value: (fpsDelta >= 0 ? '+' : '') + fpsDelta.toFixed(1),
      unit: 'fps',
      rating: fpsDelta >= 0 ? 'good' : 'bad',
    },
    {
      label: 'Normal p95 Frame',
      value: normalResult.p95.toFixed(2),
      unit: 'ms',
      rating: rateMs(normalResult.p95, 20, 33),
    },
    {
      label: 'Low-End p95 Frame',
      value: lowEndResult.p95.toFixed(2),
      unit: 'ms',
      rating: rateMs(lowEndResult.p95, 20, 33),
    },
    {
      label: 'Frame Time Saved',
      value: (frameDelta >= 0 ? '-' : '+') + Math.abs(frameDelta).toFixed(2),
      unit: 'ms avg',
      rating: frameDelta >= 0 ? 'good' : 'warn',
    },
  ];

  finishPhase(phase, start);
  return phase;
}

async function runAll(): Promise<void> {
  if (running.value) return;
  running.value = true;
  overallRating.value = null;
  phases.value = [];

  savedPlayers.value = [...players.players];

  try {
    await runFrameTimingTest();
    await sleep(25);
    await runLongTaskTest();
    await sleep(25);
    await runPlayerRenderTest();
    await sleep(25);
    await runAnimationStressTest();
    await sleep(25);
    await runReactivityTest();
    await sleep(25);
    await runAvatarCacheTest();
    await sleep(25);
    await runMemoryTest();
    await sleep(25);
    await runScrollTest();
    await sleep(25);
    await runLowEndDeltaTest();
  } finally {
    players.players = savedPlayers.value;
    running.value = false;

    const allMetrics = phases.value.flatMap((phase) => phase.metrics);
    const badCount = allMetrics.filter((metric) => metric.rating === 'bad').length;
    const warnCount = allMetrics.filter((metric) => metric.rating === 'warn').length;
    overallRating.value =
      badCount > 3 ? 'bad' : badCount > 0 || warnCount > 4 ? 'warn' : 'good';
  }
}

onUnmounted(() => {
  if (savedPlayers.value.length) {
    players.players = savedPlayers.value;
  }
});

const overallLabel = computed(() => {
  if (!overallRating.value) return '';
  return overallRating.value === 'good'
    ? 'Excellent'
    : overallRating.value === 'warn'
      ? 'Moderate'
      : 'Poor';
});

const totalDuration = computed(() =>
  phases.value.reduce((sum, phase) => sum + phase.durationMs, 0),
);
</script>

<template>
  <div
    class="bench-root themed-scroll no-drag"
    style="height: 100%; overflow-y: auto"
  >
    <div class="bench-container">
      <header class="bench-header">
        <div class="bench-header-left">
          <Activity
            :size="20"
            class="bench-icon"
          />
          <h1 class="bench-title gradient-ink">Performance Benchmark</h1>
        </div>
        <div class="bench-header-right">
          <span
            v-if="overallRating"
            class="overall-badge"
            :data-rating="overallRating"
          >
            <span class="overall-dot"></span>
            {{ overallLabel }} · {{ (totalDuration / 1000).toFixed(1) }}s
          </span>
          <Button
            variant="control"
            size="sm"
            class="gap-1.5"
            :disabled="running"
            @click="runAll"
          >
            <RefreshCw
              v-if="!running"
              :size="11"
            />
            <span
              v-if="running"
              class="btn-spinner"
            ></span>
            {{ running ? 'Running…' : phases.length ? 'Re-run' : 'Run Benchmark' }}
          </Button>
        </div>
      </header>

      <div
        v-if="!phases.length && !running"
        class="bench-empty"
      >
        <Activity
          :size="32"
          class="empty-icon"
        />
        <p class="empty-title">No results yet</p>
        <p class="empty-desc">
          Injects 16 real-shape players into the live store and measures frame timing,
          reactivity, animations, memory, scroll, cache, and low-end delta across the
          actual render pipeline with zero simulation.
        </p>
      </div>

      <div
        v-else
        class="phases-grid"
      >
        <div
          v-for="phase in phases"
          :key="phase.id"
          class="phase-card"
        >
          <div class="phase-card-header">
            <span
              class="phase-indicator"
              :data-status="phase.status"
            ></span>
            <span class="phase-name">{{ phase.label }}</span>
            <span
              v-if="phase.status === 'done'"
              class="phase-duration"
              >{{ phase.durationMs.toFixed(2) }}<small>ms</small></span
            >
            <span
              v-else-if="phase.status === 'running'"
              class="phase-duration running-text"
              >Running…</span
            >
          </div>
          <div class="phase-card-body">
            <table
              v-if="phase.metrics.length"
              class="metric-table"
            >
              <tr
                v-for="metric in phase.metrics"
                :key="metric.label"
              >
                <td class="metric-label">{{ metric.label }}</td>
                <td
                  class="metric-value"
                  :data-rating="metric.rating"
                >
                  {{ metric.value
                  }}<span
                    v-if="metric.unit"
                    class="metric-unit"
                  >
                    {{ metric.unit }}</span
                  >
                </td>
              </tr>
            </table>
            <div
              v-else-if="phase.status === 'running'"
              class="phase-loader"
            >
              <div class="loader-bar"></div>
            </div>
          </div>
        </div>
      </div>
      <div style="height: 24px; flex-shrink: 0"></div>
    </div>
  </div>
</template>

<style scoped>
.bench-root {
  font-family:
    'Inter',
    system-ui,
    -apple-system,
    sans-serif;
  padding: 16px;
  background: var(--color-surface-0, #0b0e14);
  color: var(--color-ink-1, #e5e7eb);
  min-height: 100%;
  box-sizing: border-box;
}

.bench-container {
  max-width: 680px;
  margin: 0 auto;
  padding-bottom: 24px;
}

.bench-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
}

.bench-header-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.bench-icon {
  color: var(--color-accent, #6d8ceb);
  flex-shrink: 0;
}

.bench-title {
  font-size: 1.05rem;
  font-weight: 700;
  letter-spacing: -0.01em;
  margin: 0;
  line-height: 1.2;
}

.bench-title.gradient-ink {
  display: inline-block;
}

.bench-header-right {
  display: flex;
  align-items: center;
  gap: 10px;
}

.overall-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 12px;
  border-radius: 20px;
  font-size: 0.75rem;
  font-weight: 600;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--color-border, rgba(255, 255, 255, 0.06));
  color: var(--color-ink-2, #9ca3af);
  white-space: nowrap;
}

.overall-badge[data-rating='good'] {
  border-color: rgba(52, 211, 153, 0.35);
  color: #34d399;
  background: rgba(52, 211, 153, 0.08);
  box-shadow: 0 0 14px rgba(52, 211, 153, 0.18);
}

.overall-badge[data-rating='warn'] {
  border-color: rgba(245, 158, 11, 0.35);
  color: #f59e0b;
  background: rgba(245, 158, 11, 0.08);
  box-shadow: 0 0 14px rgba(245, 158, 11, 0.18);
}

.overall-badge[data-rating='bad'] {
  border-color: rgba(248, 113, 113, 0.35);
  color: #f87171;
  background: rgba(248, 113, 113, 0.08);
  box-shadow: 0 0 14px rgba(248, 113, 113, 0.18);
}

html.low-end .overall-badge {
  box-shadow: none !important;
}

.overall-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
}

.btn-spinner {
  width: 12px;
  height: 12px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}

.bench-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 40px 16px;
  gap: 10px;
}

.empty-icon {
  color: var(--color-ink-3, #6b7280);
  opacity: 0.4;
  margin-bottom: 4px;
}

.empty-title {
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--color-ink-2, #9ca3af);
  margin: 0;
}

.empty-desc {
  font-size: 0.72rem;
  color: var(--color-ink-3, #6b7280);
  max-width: 360px;
  line-height: 1.6;
  margin: 0;
}

.phases-grid {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.phase-card {
  position: relative;
  background: linear-gradient(
    160deg,
    rgba(255, 255, 255, 0.045) 0%,
    rgba(255, 255, 255, 0.012) 100%
  );
  backdrop-filter: blur(14px) saturate(130%);
  -webkit-backdrop-filter: blur(14px) saturate(130%);
  border-radius: 10px;
  border: 1px solid var(--color-border, rgba(255, 255, 255, 0.05));
  overflow: hidden;
  transition: border-color 180ms ease;
}

.phase-card:has(.phase-indicator[data-status='running']) {
  border-color: rgba(var(--color-accent-rgb), 0.35);
  box-shadow: 0 0 20px rgba(var(--color-accent-rgb), 0.12);
}

html.low-end .phase-card {
  backdrop-filter: none !important;
  -webkit-backdrop-filter: none !important;
  background: var(--color-surface-1, #12161e) !important;
  box-shadow: none !important;
}

.phase-card-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  background: rgba(255, 255, 255, 0.02);
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

.phase-indicator {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.phase-indicator[data-status='running'] {
  background: var(--color-accent, #6d8ceb);
  box-shadow: 0 0 8px var(--color-accent, #6d8ceb);
  animation: pulse 1.2s ease-in-out infinite;
}

.phase-indicator[data-status='done'] {
  background: #34d399;
  box-shadow: 0 0 6px rgba(52, 211, 153, 0.6);
}

.phase-indicator[data-status='error'] {
  background: #f87171;
  box-shadow: 0 0 6px rgba(248, 113, 113, 0.6);
}

.phase-indicator[data-status='idle'] {
  background: var(--color-ink-3, #6b7280);
}

html.low-end .phase-indicator {
  box-shadow: none !important;
}

.phase-name {
  font-weight: 600;
  font-size: 0.78rem;
  color: var(--color-ink-1, #e5e7eb);
  flex: 1;
}

.phase-duration {
  font-size: 0.7rem;
  color: var(--color-ink-3, #6b7280);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

.phase-duration small {
  font-size: 0.65rem;
  opacity: 0.7;
  margin-left: 2px;
}

.running-text {
  color: var(--color-accent, #6d8ceb);
  font-weight: 500;
}

.phase-card-body {
  padding: 4px 0;
}

.metric-table {
  width: 100%;
  border-collapse: collapse;
}

.metric-table tr {
  border-bottom: 1px solid rgba(255, 255, 255, 0.03);
}

.metric-table tr:last-child {
  border-bottom: none;
}

.metric-label {
  padding: 5px 14px;
  font-size: 0.7rem;
  color: var(--color-ink-3, #9ca3af);
  font-weight: 450;
  vertical-align: middle;
}

.metric-value {
  padding: 5px 14px;
  text-align: right;
  font-size: 0.72rem;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  vertical-align: middle;
  white-space: nowrap;
}

.metric-value[data-rating='good'] {
  color: #34d399;
}

.metric-value[data-rating='warn'] {
  color: #f59e0b;
}

.metric-value[data-rating='bad'] {
  color: #f87171;
}

.metric-value[data-rating='neutral'] {
  color: var(--color-ink-2, #9ca3af);
}

.metric-unit {
  font-size: 0.62rem;
  font-weight: 500;
  opacity: 0.6;
  margin-left: 3px;
}

.phase-loader {
  padding: 14px 14px;
}

.loader-bar {
  height: 2px;
  border-radius: 2px;
  background: linear-gradient(90deg, var(--color-accent, #6d8ceb) 0%, transparent 100%);
  animation: scan 1.2s ease-in-out infinite;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
    transform: scale(1);
  }
  50% {
    opacity: 0.4;
    transform: scale(0.7);
  }
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

@keyframes scan {
  0% {
    transform: scaleX(0.02);
    transform-origin: left;
    opacity: 0.6;
  }
  50% {
    transform: scaleX(1);
    transform-origin: left;
    opacity: 1;
  }
  51% {
    transform: scaleX(1);
    transform-origin: right;
    opacity: 1;
  }
  100% {
    transform: scaleX(0.02);
    transform-origin: right;
    opacity: 0.6;
  }
}
</style>
