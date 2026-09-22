import { computed, ref } from 'vue';

const api = 'https://overlay.kyizl.is-a.dev';
const changelogsSeenKey = 'announcement:changelogs:seen';
const alertsSeenKey = 'announcement:alerts:seen';

export interface ChangelogPayload {
  id: string;
  version: string;
  date: string;
  content: string;
}

export interface AlertPayload {
  id: string;
  type: 'info' | 'warning' | 'update' | 'maintenance';
  title: string;
  content: string;
}

export type AnnouncementQueueItem =
  | { mode: 'changelog'; payload: ChangelogPayload }
  | { mode: 'alert'; payload: AlertPayload };

const queue = ref<AnnouncementQueueItem[]>([]);
const activeAnnouncement = computed<AnnouncementQueueItem | null>(
  () => queue.value[0] ?? null,
);

async function fetchWithTimeout(url: string): Promise<Response> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 5_000);
  try {
    return await fetch(url, { signal: controller.signal, cache: 'no-store' });
  } finally {
    clearTimeout(timer);
  }
}

function getSeenIds(key: string): string[] {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(key) ?? '[]');
    return Array.isArray(parsed) ? parsed.filter(isString) : [];
  } catch {
    return [];
  }
}

function markSeen(key: string, id: string): void {
  try {
    const seen = getSeenIds(key);
    if (!seen.includes(id)) {
      seen.push(id);
      localStorage.setItem(key, JSON.stringify(seen));
    }
  } catch {}
}

function isString(value: unknown): value is string {
  return typeof value === 'string';
}

async function loadChangelog(): Promise<ChangelogPayload | null> {
  try {
    const response = await fetchWithTimeout(`${api}/api/changelog`);
    if (!response.ok) return null;

    const body: unknown = await response.json();
    if (typeof body !== 'object' || body === null) return null;

    const fields = body as Record<string, unknown>;
    if (!isString(fields.id) || !isString(fields.version) || !isString(fields.content))
      return null;

    const payload = fields as unknown as ChangelogPayload;
    return getSeenIds(changelogsSeenKey).includes(payload.id) ? null : payload;
  } catch {
    return null;
  }
}

async function loadAlert(): Promise<AlertPayload | null> {
  try {
    const response = await fetchWithTimeout(`${api}/api/alert`);
    if (!response.ok) return null;

    const body: unknown = await response.json();
    if (typeof body !== 'object' || body === null) return null;

    const fields = body as Record<string, unknown>;
    if (
      !isString(fields.id) ||
      !isString(fields.title) ||
      !isString(fields.content) ||
      !isString(fields.type)
    ) {
      return null;
    }

    const payload = fields as unknown as AlertPayload;
    return getSeenIds(alertsSeenKey).includes(payload.id) ? null : payload;
  } catch {
    return null;
  }
}

async function fetchAnnouncements(): Promise<void> {
  const [changelog, alert] = await Promise.all([loadChangelog(), loadAlert()]);

  const entries: AnnouncementQueueItem[] = [];
  if (changelog) entries.push({ mode: 'changelog', payload: changelog });
  if (alert) entries.push({ mode: 'alert', payload: alert });

  queue.value = entries;
}

function dismissActive(): void {
  const item = queue.value[0];
  if (item === undefined) return;

  if (item.mode === 'changelog') {
    markSeen(changelogsSeenKey, item.payload.id);
  } else {
    markSeen(alertsSeenKey, item.payload.id);
  }

  queue.value = queue.value.slice(1);
}

export function useAnnouncements() {
  return {
    activeAnnouncement,
    fetchAnnouncements,
    dismissActive,
  };
}
