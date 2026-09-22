import { reactive } from 'vue';

export type PerfCategory =
  'frame' | 'longtask' | 'store' | 'ipc' | 'network' | 'memory' | 'lifecycle' | 'render';

export interface PerfEvent {
  t: number;
  cat: PerfCategory;
  label: string;
  dur: number;
  meta?: Record<string, unknown>;
}

export interface PerfSummary {
  fps: number;
  frameMs: number;
  jankPct: number;
  longTaskCount: number;
  longTaskTotalMs: number;
  heapUsedMB: number;
  heapTotalMB: number;
  heapLimitMB: number;
  ipcCount: number;
  ipcAvgMs: number;
  ipcMaxMs: number;
  storeCount: number;
  storeAvgMs: number;
  storeMaxMs: number;
  networkCount: number;
  networkAvgMs: number;
  networkMaxMs: number;
  renderCount: number;
  renderAvgMs: number;
  uptimeMs: number;
}

export interface PerfSnapshot {
  generatedAt: string;
  uptimeMs: number;
  summary: PerfSummary;
  events: Record<PerfCategory, PerfEvent[]>;
}

const capacityPerCategory = 4000;
const frame = 240;
const rollingWindow = 120;
const memSampleInterval = 2000;
const longTaskThreshold = 50;
const jankFrameThreshold = 33.34;
const maxTracked = 60000;

class RingBuffer<T> {
  private items: T[] = [];

  constructor(private readonly capacity: number) {}

  push(item: T): void {
    this.items.push(item);
    if (this.items.length > this.capacity) this.items.shift();
  }

  toArray(): T[] {
    return this.items.slice();
  }

  clear(): void {
    this.items = [];
  }

  get length(): number {
    return this.items.length;
  }
}

const categories: PerfCategory[] = [
  'frame',
  'longtask',
  'store',
  'ipc',
  'network',
  'memory',
  'lifecycle',
  'render',
];

const buffers: Record<PerfCategory, RingBuffer<PerfEvent>> = {
  frame: new RingBuffer(capacityPerCategory),
  longtask: new RingBuffer(capacityPerCategory),
  store: new RingBuffer(capacityPerCategory),
  ipc: new RingBuffer(capacityPerCategory),
  network: new RingBuffer(capacityPerCategory),
  memory: new RingBuffer(capacityPerCategory),
  lifecycle: new RingBuffer(capacityPerCategory),
  render: new RingBuffer(capacityPerCategory),
};

const startedAt = performance.now();

export const perfState = reactive<PerfSummary>({
  fps: 0,
  frameMs: 0,
  jankPct: 0,
  longTaskCount: 0,
  longTaskTotalMs: 0,
  heapUsedMB: 0,
  heapTotalMB: 0,
  heapLimitMB: 0,
  ipcCount: 0,
  ipcAvgMs: 0,
  ipcMaxMs: 0,
  storeCount: 0,
  storeAvgMs: 0,
  storeMaxMs: 0,
  networkCount: 0,
  networkAvgMs: 0,
  networkMaxMs: 0,
  renderCount: 0,
  renderAvgMs: 0,
  uptimeMs: 0,
});

export const perfEnabled = reactive({ value: false });

let frameHandle = 0;
let lastFrameTime = 0;
const recentFrameDurations: number[] = [];

let longTaskObserver: PerformanceObserver | null = null;
let memoryTimer: ReturnType<typeof setInterval> | null = null;

function pushRolling(arr: number[], value: number, max: number): void {
  arr.push(value);
  if (arr.length > max) arr.shift();
}

function average(arr: number[]): number {
  if (arr.length === 0) return 0;
  let sum = 0;
  for (const v of arr) sum += v;
  return sum / arr.length;
}

function maxOf(arr: number[]): number {
  let m = 0;
  for (const v of arr) if (v > m) m = v;
  return m;
}

export function record(
  cat: PerfCategory,
  label: string,
  dur = 0,
  meta?: Record<string, unknown>,
): void {
  buffers[cat].push({ t: performance.now(), cat, label, dur, meta });
}

export function mark(label: string, meta?: Record<string, unknown>): void {
  record('lifecycle', label, performance.now() - startedAt, meta);
}

const storeDurations: number[] = [];
export function recordStore(
  name: string,
  dur: number,
  meta?: Record<string, unknown>,
): void {
  record('store', name, dur, meta);
  pushRolling(storeDurations, dur, rollingWindow);
  perfState.storeCount += 1;
  perfState.storeAvgMs = average(storeDurations);
  perfState.storeMaxMs = maxOf(storeDurations);
}

const networkDurations: number[] = [];
export function recordNetwork(
  label: string,
  dur: number,
  meta?: Record<string, unknown>,
): void {
  record('network', label, dur, meta);
  pushRolling(networkDurations, dur, rollingWindow);
  perfState.networkCount += 1;
  perfState.networkAvgMs = average(networkDurations);
  perfState.networkMaxMs = maxOf(networkDurations);
}

function frameStep(now: number): void {
  if (lastFrameTime !== 0) {
    const delta = now - lastFrameTime;
    if (delta < maxTracked) {
      pushRolling(recentFrameDurations, delta, frame);
      record('frame', 'frame', delta);

      const meanFrame = average(recentFrameDurations);
      perfState.frameMs = meanFrame;
      perfState.fps = meanFrame > 0 ? 1000 / meanFrame : 0;

      const jankFrames = recentFrameDurations.filter(
        (d) => d > jankFrameThreshold,
      ).length;
      perfState.jankPct = (jankFrames / recentFrameDurations.length) * 100;
    }
  }
  lastFrameTime = now;
  perfState.uptimeMs = now - startedAt;
  frameHandle = requestAnimationFrame(frameStep);
}

function startFrameLoop(): void {
  if (frameHandle !== 0) return;
  lastFrameTime = 0;
  frameHandle = requestAnimationFrame(frameStep);
}

function stopFrameLoop(): void {
  if (frameHandle === 0) return;
  cancelAnimationFrame(frameHandle);
  frameHandle = 0;
}

function startLongTaskObserver(): void {
  if (longTaskObserver !== null) return;
  if (typeof PerformanceObserver === 'undefined') return;

  try {
    longTaskObserver = new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        if (entry.duration < longTaskThreshold) continue;
        record(
          'longtask',
          entry.name.length > 0 ? entry.name : 'long-task',
          entry.duration,
          {
            startTime: entry.startTime,
          },
        );
        perfState.longTaskCount += 1;
        perfState.longTaskTotalMs += entry.duration;
      }
    });
    longTaskObserver.observe({ entryTypes: ['longtask'] });
  } catch {
    longTaskObserver = null;
  }
}

function stopLongTaskObserver(): void {
  longTaskObserver?.disconnect();
  longTaskObserver = null;
}

interface HeapMemoryInfo {
  usedJSHeapSize: number;
  totalJSHeapSize: number;
  jsHeapSizeLimit: number;
}

function sampleMemory(): void {
  const perfWithMemory = performance as Performance & { memory?: HeapMemoryInfo };
  const mem = perfWithMemory.memory;
  if (!mem) return;

  const usedMB = mem.usedJSHeapSize / 1048576;
  const totalMB = mem.totalJSHeapSize / 1048576;
  const limitMB = mem.jsHeapSizeLimit / 1048576;

  perfState.heapUsedMB = usedMB;
  perfState.heapTotalMB = totalMB;
  perfState.heapLimitMB = limitMB;

  record('memory', 'heap', usedMB, { totalMB, limitMB });
}

function startMemorySampling(): void {
  if (memoryTimer !== null) return;
  sampleMemory();
  memoryTimer = setInterval(sampleMemory, memSampleInterval);
}

function stopMemorySampling(): void {
  if (memoryTimer === null) return;
  clearInterval(memoryTimer);
  memoryTimer = null;
}

export function startPerfLogging(): void {
  if (perfEnabled.value) return;
  perfEnabled.value = true;
  startFrameLoop();
  startLongTaskObserver();
  startMemorySampling();
  mark('perf-logging-started');
}

export function stopPerfLogging(): void {
  if (!perfEnabled.value) return;
  mark('perf-logging-stopped');
  perfEnabled.value = false;
  stopFrameLoop();
  stopLongTaskObserver();
  stopMemorySampling();
}

export function clearPerfData(): void {
  for (const cat of categories) buffers[cat].clear();
  recentFrameDurations.length = 0;
  storeDurations.length = 0;
  networkDurations.length = 0;
  perfState.longTaskCount = 0;
  perfState.longTaskTotalMs = 0;
  perfState.ipcCount = 0;
  perfState.storeCount = 0;
  perfState.networkCount = 0;
  perfState.renderCount = 0;
}

export function snapshotPerfData(): PerfSnapshot {
  const events = {} as Record<PerfCategory, PerfEvent[]>;
  for (const cat of categories) events[cat] = buffers[cat].toArray();

  return {
    generatedAt: new Date().toISOString(),
    uptimeMs: perfState.uptimeMs,
    summary: { ...perfState },
    events,
  };
}

export function topSlowest(cat: PerfCategory, count = 10): PerfEvent[] {
  return buffers[cat]
    .toArray()
    .sort((a, b) => b.dur - a.dur)
    .slice(0, count);
}
