import type { Network } from '@renderer/types';

interface StaffApiMember {
  username: string;
  role: string;
}

interface StaffApiGroup {
  id: string;
  members: StaffApiMember[];
}

interface StaffApiResponse {
  groups: StaffApiGroup[];
}

type SubroleGroupMap = Map<string, Map<string, string>>;

interface CacheEntry {
  data: SubroleGroupMap | null;
  expiresAt: number;
  pending: Promise<SubroleGroupMap | null> | null;
}

const api = 'https://craftigames.kyizl.is-a.dev/api/staff';
const ttl = 5 * 60 * 1000;

const map: Record<Network, string> = {
  pikanetwork: 'pika',
  jartexnetwork: 'jartex',
};

const eligible: Record<Network, Set<string>> = {
  pikanetwork: new Set(['admin', 'developer']),
  jartexnetwork: new Set(['developer']),
};

const cache = new Map<Network, CacheEntry>();

function buildGroupMap(response: StaffApiResponse): SubroleGroupMap {
  const groupMap: SubroleGroupMap = new Map();
  for (const group of response.groups ?? []) {
    if (!group?.id) continue;
    const memberMap = new Map<string, string>();
    for (const member of group.members ?? []) {
      if (!member?.username || !member?.role) continue;
      memberMap.set(member.username.toLowerCase(), member.role);
    }
    groupMap.set(group.id, memberMap);
  }
  return groupMap;
}

async function fetchGroupMap(network: Network): Promise<SubroleGroupMap | null> {
  const slug = map[network];
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), 8000);

  try {
    const response = await fetch(`${api}/${slug}`, {
      signal: controller.signal,
    });
    if (!response.ok) return null;

    const payload = (await response.json()) as StaffApiResponse;
    if (!Array.isArray(payload?.groups)) return null;

    return buildGroupMap(payload);
  } catch {
    return null;
  } finally {
    clearTimeout(timeoutId);
  }
}

async function getGroupMap(network: Network): Promise<SubroleGroupMap | null> {
  const now = Date.now();
  const existing = cache.get(network);

  if (existing?.pending) return existing.pending;
  if (existing && existing.expiresAt > now) return Promise.resolve(existing.data);

  const pending = fetchGroupMap(network).then((data) => {
    cache.set(network, {
      data,
      expiresAt: Date.now() + ttl,
      pending: null,
    });
    return data;
  });

  cache.set(network, {
    data: existing?.data ?? null,
    expiresAt: existing?.expiresAt ?? 0,
    pending,
  });

  return pending;
}

export function isStaffSubroleEligible(
  network: Network,
  rankName: string | null,
): boolean {
  /* eslint-disable-next-line ts/strict-boolean-expressions */
  if (!rankName) return false;
  return eligible[network]?.has(rankName) ?? false;
}

export async function resolveStaffSubrole(
  network: Network,
  rankName: string | null,
  username: string | null,
): Promise<string | null> {
  /* eslint-disable-next-line ts/strict-boolean-expressions */
  if (!username || !isStaffSubroleEligible(network, rankName)) return null;

  const groupMap = await getGroupMap(network);
  if (!groupMap) return null;

  const memberMap = groupMap.get(rankName as string);
  if (!memberMap) return null;

  return memberMap.get(username.toLowerCase()) ?? null;
}
