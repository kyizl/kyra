import { Buffer } from 'node:buffer';
import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import process from 'node:process';
import { app, safeStorage } from 'electron';
import { decryptLocalSecret, encryptLocalSecret, isLocalSecret } from './local-secret';

export type TrafficMode = 'middleware' | 'direct';

export interface TrafficStatus {
  mode: TrafficMode;
  hasApiKey: boolean;
}

interface TrafficConfig {
  mode: TrafficMode;
  encryptedApiKey?: string;
}

export interface StatsRoute {
  base: string;
  headers: Record<string, string>;
  cacheScope: string;
}

let config: TrafficConfig | undefined;
let revision = 0;

function loadConfig(): TrafficConfig {
  if (config) return config;
  const path = join(app.getPath('userData'), 'traffic.json');
  if (!existsSync(path)) return (config = { mode: 'middleware' });
  const saved = JSON.parse(readFileSync(path, 'utf8')) as Partial<TrafficConfig>;
  config = {
    mode: saved.mode === 'direct' ? 'direct' : 'middleware',
    encryptedApiKey:
      typeof saved.encryptedApiKey === 'string' ? saved.encryptedApiKey : undefined,
  };
  return config;
}

function persist(next: TrafficConfig): TrafficStatus {
  const directory = app.getPath('userData');
  const path = join(directory, 'traffic.json');
  mkdirSync(directory, { recursive: true, mode: 0o700 });
  writeFileSync(`${path}.tmp`, JSON.stringify(next), { mode: 0o600 });
  renameSync(`${path}.tmp`, path);
  config = next;
  revision++;
  return getTrafficStatus();
}

function canUseSystemStorage(): boolean {
  return (
    safeStorage.isEncryptionAvailable() &&
    (process.platform !== 'linux' ||
      safeStorage.getSelectedStorageBackend() !== 'basic_text')
  );
}

function readApiKey(): string {
  const { encryptedApiKey } = loadConfig();
  if (encryptedApiKey === undefined || encryptedApiKey === '')
    throw new Error('Save an API key in Settings → Traffic first.');
  try {
    if (isLocalSecret(encryptedApiKey)) {
      return decryptLocalSecret(app.getPath('userData'), encryptedApiKey);
    }
    if (!canUseSystemStorage()) {
      throw new Error('System credential storage is unavailable.');
    }
    return safeStorage.decryptString(Buffer.from(encryptedApiKey, 'base64'));
  } catch {
    throw new Error(
      'Could not decrypt the saved API key. Replace it in Settings → Traffic.',
    );
  }
}

export function getTrafficStatus(): TrafficStatus {
  const current = loadConfig();
  return {
    mode: current.mode,
    hasApiKey: current.encryptedApiKey !== undefined && current.encryptedApiKey !== '',
  };
}

export function setTrafficMode(mode: TrafficMode): TrafficStatus {
  if (mode !== 'middleware' && mode !== 'direct')
    throw new Error('Invalid traffic mode.');
  if (mode === 'direct') readApiKey();
  return persist({ ...loadConfig(), mode });
}

export function saveTrafficApiKey(value: string): TrafficStatus {
  if (typeof value !== 'string' || !/^[\x21-\x7E]+$/.test(value.trim())) {
    throw new Error('Enter a valid API key without spaces or control characters.');
  }
  let encryptedApiKey: string;
  try {
    if (!canUseSystemStorage())
      throw new Error('System credential storage is unavailable.');
    encryptedApiKey = safeStorage.encryptString(value.trim()).toString('base64');
  } catch {
    encryptedApiKey = encryptLocalSecret(app.getPath('userData'), value.trim());
  }
  return persist({ mode: 'direct', encryptedApiKey });
}

export function removeTrafficApiKey(): TrafficStatus {
  return persist({ mode: 'middleware' });
}

export function getStatsRoute(network: 'pika' | 'jartex'): StatsRoute {
  if (network === 'jartex') {
    return {
      base: 'https://stats.jartexnetwork.com/api',
      headers: {},
      cacheScope: 'jartex',
    };
  }
  const { mode } = loadConfig();
  return {
    base:
      mode === 'direct'
        ? 'https://stats.pika-network.net/api'
        : 'https://proxy.kyizl.is-a.dev/api',
    headers: mode === 'direct' ? { 'X-Api-Key': readApiKey() } : {},
    cacheScope: `pika:${revision}`,
  };
}
