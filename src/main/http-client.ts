import http from 'node:http';
import https from 'node:https';
import axios from 'axios';
import { dbg } from './logger';

const concurrent = 12;
const ttl = 600_000;
const cacheMaxEntries = 2_000;
const cacheSweepIntervalMs = 120_000;

export const httpAgent = new http.Agent({ keepAlive: true, maxSockets: concurrent });
export const httpsAgent = new https.Agent({ keepAlive: true, maxSockets: concurrent });

interface CacheEntry<T> {
  data: T;
  expires: number;
}

class SimpleCache {
  private store = new Map<string, CacheEntry<unknown>>();
  private sweepTimer: NodeJS.Timeout;

  constructor() {
    this.sweepTimer = setInterval(() => this.sweep(), cacheSweepIntervalMs);
    this.sweepTimer.unref();
  }

  set<T>(key: string, data: T, ttlMs: number = ttl): void {
    this.store.delete(key);
    this.store.set(key, { data, expires: Date.now() + ttlMs });
    if (this.store.size > cacheMaxEntries) {
      const oldestKey = this.store.keys().next().value;
      if (oldestKey !== undefined) this.store.delete(oldestKey);
    }
    dbg.cache(`SET  "${key}"  (TTL ${ttlMs / 1000}s, store size: ${this.store.size})`);
  }

  get<T>(key: string): T | null {
    const entry = this.store.get(key);
    if (!entry) {
      dbg.cache(`MISS "${key}"`);
      return null;
    }
    if (Date.now() > entry.expires) {
      this.store.delete(key);
      dbg.cache(`EXPIRED "${key}"`);
      return null;
    }
    const secondsLeft = Math.round((entry.expires - Date.now()) / 1000);
    dbg.cache(`HIT  "${key}"  (${secondsLeft}s left)`);
    return entry.data as T;
  }

  private sweep(): void {
    const now = Date.now();
    let purged = 0;
    for (const [key, entry] of this.store) {
      if (now > entry.expires) {
        this.store.delete(key);
        purged++;
      }
    }
    if (purged > 0) {
      dbg.cache(
        `SWEEP (${purged} expired entries purged, store size: ${this.store.size})`,
      );
    }
  }

  clear(): void {
    dbg.cache(`CLEAR (${this.store.size} entries flushed)`);
    this.store.clear();
  }

  dispose(): void {
    clearInterval(this.sweepTimer);
    this.clear();
  }
}

export const cache = new SimpleCache();

const inFlightRequests = new Map<string, Promise<unknown>>();

export async function dedupe<T>(key: string, fn: () => Promise<T>): Promise<T> {
  const existing = inFlightRequests.get(key);
  if (existing) {
    dbg.dedup(`REUSE in-flight "${key}" (${inFlightRequests.size} total in-flight)`);
    return existing as Promise<T>;
  }
  dbg.dedup(`NEW   "${key}"`);
  const promise = fn().finally(() => {
    inFlightRequests.delete(key);
    dbg.dedup(`DONE  "${key}" (${inFlightRequests.size} remaining in-flight)`);
  });
  inFlightRequests.set(key, promise);
  return promise;
}

class Semaphore {
  private capacity: number;
  private inUse = 0;
  private waitQueue: Array<() => void> = [];

  constructor(limit: number) {
    this.capacity = limit;
  }

  getLimit(): number {
    return this.capacity;
  }

  setLimit(limit: number): void {
    this.capacity = Math.max(1, Math.min(limit, concurrent));
    dbg.sem(`ADAPT (target ${this.capacity}, in use: ${this.inUse})`);
    this.drainQueue();
  }

  private drainQueue(): void {
    while (this.waitQueue.length > 0 && this.inUse < this.capacity) {
      this.inUse++;
      const resolve = this.waitQueue.shift();
      if (resolve) resolve();
    }
  }

  async acquire(): Promise<void> {
    if (this.inUse < this.capacity) {
      this.inUse++;
      dbg.sem(
        `ACQUIRE (in use: ${this.inUse}/${this.capacity}, queued: ${this.waitQueue.length})`,
      );
      return Promise.resolve();
    }
    dbg.sem(
      `QUEUE   (in use: ${this.inUse}/${this.capacity}, queued: ${this.waitQueue.length + 1})`,
    );
    return new Promise<void>((resolve) => this.waitQueue.push(resolve));
  }

  release(): void {
    this.inUse = Math.max(0, this.inUse - 1);
    if (this.waitQueue.length > 0 && this.inUse < this.capacity) {
      this.inUse++;
      const resolve = this.waitQueue.shift();
      dbg.sem(
        `RELEASE → waking queued request (in use: ${this.inUse}/${this.capacity}, queued remaining: ${this.waitQueue.length})`,
      );
      if (resolve) resolve();
    } else {
      dbg.sem(
        `RELEASE (in use: ${this.inUse}/${this.capacity}, queued: ${this.waitQueue.length})`,
      );
    }
  }
}

const requestSemaphore = new Semaphore(concurrent);

async function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function withRetry<T>(
  fn: () => Promise<T>,
  retries = 3,
  delayMs = 750,
): Promise<T> {
  let lastError: unknown;
  for (let attempt = 0; attempt <= retries; attempt++) {
    try {
      return await fn();
    } catch (error) {
      lastError = error;
      if (attempt < retries) {
        const isRateLimit = (error as Error).message.includes('429');
        const baseDelay = isRateLimit ? 3_000 : delayMs;
        const maxBackoff = isRateLimit ? 15_000 : 8_000;
        const wait = Math.min(baseDelay * 2 ** attempt, maxBackoff) + Math.random() * 600;
        dbg.retry(
          `attempt ${attempt + 1}/${retries} failed - retrying in ${Math.round(wait)}ms  (${(error as Error).message})`,
        );
        await sleep(wait);
      }
    }
  }
  throw lastError;
}

export async function apiGet<T = unknown>(
  url: string,
  headers: Record<string, string> = {},
) {
  await requestSemaphore.acquire();
  const startedAt = Date.now();
  dbg.http(`GET ${url}`);
  try {
    const response = await withRetry(async () => {
      const result = await axios.get<T>(url, {
        timeout: 5_000,
        headers: { Accept: 'application/json', ...headers },
        maxRedirects: 0,
        validateStatus: () => true,
        httpAgent,
        httpsAgent,
      });
      if (result.status === 429) throw new Error('Rate limited (429)');
      if (result.status >= 500) throw new Error(`Server error ${result.status}`);
      return result;
    });
    dbg.http(`${response.status} ${url}  (${Date.now() - startedAt}ms)`);
    return response;
  } catch (error) {
    const message = (error as Error).message;
    if (message.includes('429')) {
      const nextLimit = Math.max(4, Math.floor(requestSemaphore.getLimit() / 2));
      requestSemaphore.setLimit(nextLimit);
    }
    dbg.error(`FAIL ${url}  (${Date.now() - startedAt}ms) - ${message}`);
    throw error;
  } finally {
    requestSemaphore.release();
  }
}

export function disposeHttpClient(): void {
  cache.dispose();
  httpAgent.destroy();
  httpsAgent.destroy();
}
