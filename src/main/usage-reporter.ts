import { randomUUID } from 'node:crypto';
import { app } from 'electron';
import { dbg } from './logger';

const endpoint = 'https://overlay.kyizl.is-a.dev/api/telemetry/ping';
const build = 'electron';
const heartbeatIntervalMs = 120_000;
const heartbeatJitterMs = 10_000;
const requestTimeoutMs = 5_000;
const shutdownTimeoutMs = 2_000;

type PingEvent = 'start' | 'heartbeat' | 'end';
type PingOutcome = 'accepted' | 'rejected' | 'failed';

interface UsageSession {
  apiKey: string;
  sessionId: string;
  timer: NodeJS.Timeout | null;
}

let session: UsageSession | null = null;
let rejectedApiKey: string | null = null;

async function sendPing(
  target: UsageSession,
  event: PingEvent,
  timeoutMs: number,
): Promise<PingOutcome> {
  try {
    const response = await fetch(endpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        key: target.apiKey,
        sessionId: target.sessionId,
        build,
        version: app.getVersion(),
        event,
      }),
      signal: AbortSignal.timeout(timeoutMs),
    });

    if (response.status === 401) return 'rejected';
    return response.ok ? 'accepted' : 'failed';
  } catch {
    return 'failed';
  }
}

function nextDelayMs(): number {
  return heartbeatIntervalMs + (Math.random() * 2 - 1) * heartbeatJitterMs;
}

function clearTimer(target: UsageSession): void {
  if (target.timer === null) return;
  clearTimeout(target.timer);
  target.timer = null;
}

function dropRejectedSession(target: UsageSession): void {
  dbg.ipc('usage - API key rejected, reporting suspended');
  clearTimer(target);
  rejectedApiKey = target.apiKey;
  if (session === target) session = null;
}

function scheduleHeartbeat(target: UsageSession): void {
  clearTimer(target);
  target.timer = setTimeout(() => void heartbeat(target), nextDelayMs());
  target.timer.unref();
}

async function heartbeat(target: UsageSession): Promise<void> {
  target.timer = null;
  if (session !== target) return;

  const outcome = await sendPing(target, 'heartbeat', requestTimeoutMs);
  if (session !== target) return;

  if (outcome === 'rejected') {
    dropRejectedSession(target);
    return;
  }

  scheduleHeartbeat(target);
}

async function begin(target: UsageSession): Promise<void> {
  const outcome = await sendPing(target, 'start', requestTimeoutMs);
  if (session !== target) return;

  if (outcome === 'rejected') {
    dropRejectedSession(target);
    return;
  }

  scheduleHeartbeat(target);
}

export function startUsageReporting(apiKey: string): void {
  if (session?.apiKey === apiKey || rejectedApiKey === apiKey) return;

  if (session !== null) {
    clearTimer(session);
    session = null;
  }

  rejectedApiKey = null;

  const target: UsageSession = { apiKey, sessionId: randomUUID(), timer: null };
  session = target;
  dbg.ipc('usage - session started');
  void begin(target);
}

export function suspendUsageReporting(): void {
  if (session === null) return;

  clearTimer(session);
  session = null;
}

export function isUsageReportingActive(): boolean {
  return session !== null;
}

export async function stopUsageReporting(): Promise<void> {
  const target = session;
  if (target === null) return;

  clearTimer(target);
  session = null;
  await sendPing(target, 'end', shutdownTimeoutMs);
  dbg.ipc('usage - session ended');
}
