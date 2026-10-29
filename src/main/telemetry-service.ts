import { Buffer } from 'node:buffer';
import { promises as fs } from 'node:fs';
import { dirname, join } from 'node:path';
import { app, safeStorage, shell } from 'electron';
import { dbg } from './logger';
import { startUsageReporting, suspendUsageReporting } from './usage-reporter';
import { getMainWindow } from './window';

const api = 'https://overlay.kyizl.is-a.dev';

interface TelemetryState {
  apiKey: string | null;
  countClaimed: boolean;
  usageDisclosureAcknowledged: boolean;
}

export type TelemetryEvent =
  { type: 'linking' } | { type: 'linked' } | { type: 'error'; message: string };

type OAuthSessionResult =
  | { status: 'pending' }
  | { status: 'success'; apiKey: string; username: string; avatarUrl: string }
  | { status: 'error'; message: string };

let statePath: string | null = null;
let legacyStatePath: string | null = null;
let cachedState: TelemetryState | null = null;
let activePollToken = 0;

function getStatePath(): string {
  if (statePath === null) {
    statePath = join(app.getPath('userData'), 'telemetry.enc');
  }
  return statePath;
}

function getLegacyStatePath(): string {
  if (legacyStatePath === null) {
    legacyStatePath = join(app.getPath('userData'), 'telemetry.json');
  }
  return legacyStatePath;
}

function normalizeState(parsed: Partial<TelemetryState>): TelemetryState {
  return {
    apiKey: typeof parsed.apiKey === 'string' ? parsed.apiKey : null,
    countClaimed: parsed.countClaimed === true,
    usageDisclosureAcknowledged: parsed.usageDisclosureAcknowledged === true,
  };
}

async function loadLegacyPlaintextState(): Promise<TelemetryState | null> {
  try {
    const raw = await fs.readFile(getLegacyStatePath(), 'utf8');
    const state = normalizeState(JSON.parse(raw) as Partial<TelemetryState>);
    await fs.unlink(getLegacyStatePath()).catch(() => undefined);
    return state;
  } catch {
    return null;
  }
}

async function loadState(): Promise<TelemetryState> {
  if (cachedState) return cachedState;

  try {
    const raw = await fs.readFile(getStatePath());

    if (safeStorage.isEncryptionAvailable()) {
      cachedState = normalizeState(
        JSON.parse(safeStorage.decryptString(raw)) as Partial<TelemetryState>,
      );
      return cachedState;
    }

    cachedState = normalizeState(
      JSON.parse(raw.toString('utf8')) as Partial<TelemetryState>,
    );
    return cachedState;
  } catch {
    const migrated = await loadLegacyPlaintextState();

    if (migrated) {
      cachedState = migrated;
      await saveState(migrated);
      return cachedState;
    }

    cachedState = normalizeState({});
    return cachedState;
  }
}

async function saveState(state: TelemetryState): Promise<void> {
  cachedState = state;

  const target = getStatePath();
  const tmpPath = `${target}.tmp`;
  const json = JSON.stringify(state);
  const data = safeStorage.isEncryptionAvailable()
    ? safeStorage.encryptString(json)
    : Buffer.from(json, 'utf8');

  await fs.mkdir(dirname(target), { recursive: true, mode: 0o700 });
  await fs.writeFile(tmpPath, data, { mode: 0o600 });
  await fs.rename(tmpPath, target);
}

function emit(event: TelemetryEvent): void {
  getMainWindow()?.webContents.send('telemetry:event', event);
}

function syncUsageReporting(state: TelemetryState): void {
  if (state.apiKey !== null && state.usageDisclosureAcknowledged) {
    startUsageReporting(state.apiKey);
    return;
  }

  suspendUsageReporting();
}

const validity = 10 * 60 * 1000;

let verifiedApiKey: string | null = null;
let verifiedUntil = 0;

type VerifyResult = 'valid' | 'invalid' | 'unknown';

async function verifyApiKey(apiKey: string): Promise<VerifyResult> {
  try {
    const response = await fetch(
      `${api}/api/telemetry/verify?key=${encodeURIComponent(apiKey)}`,
    );
    if (!response.ok) return 'unknown';
    const body = (await response.json()) as { valid: boolean };
    return body.valid ? 'valid' : 'invalid';
  } catch {
    return 'unknown';
  }
}

export async function getLinkedApiKey(): Promise<string | null> {
  const state = await loadState();
  return state.apiKey;
}

export async function isLinked(): Promise<boolean> {
  const state = await loadState();
  if (state.apiKey === null) return false;

  const now = Date.now();
  if (verifiedApiKey === state.apiKey && now < verifiedUntil) {
    syncUsageReporting(state);
    return true;
  }

  const result = await verifyApiKey(state.apiKey);

  if (result === 'valid') {
    verifiedApiKey = state.apiKey;
    verifiedUntil = now + validity;
    syncUsageReporting(state);
    return true;
  }

  if (result === 'invalid') {
    suspendUsageReporting();
    await saveState(normalizeState({}));
    verifiedApiKey = null;
    verifiedUntil = 0;
    return false;
  }

  syncUsageReporting(state);
  return true;
}

export async function isUsageDisclosureAcknowledged(): Promise<boolean> {
  const state = await loadState();
  return state.usageDisclosureAcknowledged;
}

export async function acknowledgeUsageDisclosure(): Promise<void> {
  const state = await loadState();
  const next = { ...state, usageDisclosureAcknowledged: true };
  await saveState(next);
  syncUsageReporting(next);
}

export function startDiscordLink(): void {
  const token = ++activePollToken;

  dbg.ipc('telemetry:start-link - creating Discord OAuth session');
  emit({ type: 'linking' });

  void (async () => {
    let session: { state: string; authorizeUrl: string };
    try {
      const response = await fetch(`${api}/oauth/discord/start`);
      if (!response.ok) {
        throw new Error(`start failed with status ${response.status}`);
      }
      session = (await response.json()) as { state: string; authorizeUrl: string };
    } catch {
      if (token === activePollToken) {
        emit({ type: 'error', message: 'Could not reach the server. Please try again.' });
      }
      return;
    }

    dbg.ipc('telemetry:start-link - opening system browser for Discord OAuth');
    void shell.openExternal(session.authorizeUrl);

    void pollDiscordSession(session.state, token);
  })();
}

async function pollDiscordSession(state: string, token: number): Promise<void> {
  const deadline = Date.now() + 10 * 60 * 1000;

  while (Date.now() < deadline) {
    if (token !== activePollToken) return;

    await sleep(1500);

    if (token !== activePollToken) return;

    let result: OAuthSessionResult;
    try {
      const response = await fetch(
        `${api}/oauth/discord/poll?state=${encodeURIComponent(state)}`,
      );
      result = (await response.json()) as OAuthSessionResult;
    } catch {
      continue;
    }

    if (result.status === 'pending') {
      continue;
    }

    if (result.status === 'success') {
      await completeDiscordLink(result.apiKey);
      return;
    }

    emit({ type: 'error', message: result.message });
    return;
  }

  if (token === activePollToken) {
    emit({ type: 'error', message: 'Sign-in timed out. Please try again.' });
  }
}

async function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function claimCountIncrement(apiKey: string): Promise<boolean> {
  const response = await fetch(`${api}/api/telemetry/count-increment`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ key: apiKey }),
  });

  if (!response.ok) {
    return false;
  }

  return true;
}

export async function completeDiscordLink(apiKey: string): Promise<void> {
  dbg.ipc('telemetry - completing Discord link with permanent API key');

  const state = await loadState();

  try {
    if (!state.countClaimed) {
      const claimed = await claimCountIncrement(apiKey);
      if (!claimed) {
        emit({
          type: 'error',
          message: 'Could not confirm your link. Please try again.',
        });
        return;
      }
    }

    const linkedState: TelemetryState = {
      apiKey,
      countClaimed: true,
      usageDisclosureAcknowledged: true,
    };
    await saveState(linkedState);
    verifiedApiKey = apiKey;
    verifiedUntil = Date.now() + validity;
    syncUsageReporting(linkedState);
    emit({ type: 'linked' });
  } catch {
    emit({ type: 'error', message: 'Could not reach the server. Please try again.' });
  }
}
