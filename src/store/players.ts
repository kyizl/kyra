import type {
  BedwarsMode,
  Interval,
  PikaBedwarsStats,
  PikaProfile,
  Player,
} from '@renderer/types';
import { recordNetwork } from '@renderer/lib/perf-bus';
import { normalizeProfile } from '@renderer/types';
import { defineStore } from 'pinia';
import { useConfigStore } from './config';
import { useNicksStore } from './nicks';

export interface PlayersState {
  players: Player[];
  playersCount: number | null;
  logPathValid: boolean | null;
  setupVisible: boolean;
  proxyConnectedNetwork: string | null;
}

interface FetchResult {
  profile: PikaProfile | null;
  stats: PikaBedwarsStats | null;
  notFound?: boolean;
  rateLimit?: boolean;
  statsDisabled?: boolean;
}

function createBlankPlayer(name: string, source: Player['source']): Player {
  return {
    name,
    realName: name,
    uuid: null,
    loading: true,
    error: null,
    nicked: false,
    profile: null,
    stats: null,
    source,
    team: null,
    teamColor: null,
  };
}

function getNetworkApi() {
  const config = useConfigStore();
  return config.network === 'jartexnetwork' ? window.api.jartex : window.api.pika;
}

const concurrency = 12;
const retries = 2;

let availableSlots = concurrency;
const slotWaiters: Array<(acquired: boolean) => void> = [];
let fetchEpoch = 0;

async function acquireFetchSlot(): Promise<boolean> {
  if (availableSlots > 0) {
    availableSlots--;
    return Promise.resolve(true);
  }
  return new Promise<boolean>((resolve) => slotWaiters.push(resolve));
}

function releaseFetchSlot(): void {
  const nextWaiter = slotWaiters.shift();
  if (nextWaiter) {
    nextWaiter(true);
  } else {
    availableSlots++;
  }
}

function resetFetchQueue(): void {
  fetchEpoch++;
  let waiter: ((acquired: boolean) => void) | undefined;
  while (slotWaiters.length > 0) {
    waiter = slotWaiters.shift();
    if (waiter) waiter(false);
  }
  availableSlots = concurrency;
}

async function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
const retryJitter = (): number => Math.random() * 600;

const pendingAdds = new Set<string>();

export const usePlayersStore = defineStore('players', {
  state: (): PlayersState => ({
    players: [],
    playersCount: null,
    logPathValid: null,
    setupVisible: true,
    proxyConnectedNetwork: null,
  }),

  getters: {
    missingCount(state): number {
      if (state.playersCount === null || state.playersCount === 0) return 0;
      const autoCount = state.players.filter((player) => player.source === 'auto').length;
      return Math.max(0, state.playersCount - autoCount);
    },
  },

  actions: {
    async addByName(rawName: string, source: Player['source'] = 'manual'): Promise<void> {
      const config = useConfigStore();
      const nicks = useNicksStore();
      const api = getNetworkApi();

      const stripped = rawName.split(' ').pop()!;
      const realName = nicks.resolve(stripped);
      const key = realName.toLowerCase();

      if (pendingAdds.has(key)) return;
      if (this.players.some((player) => player.realName.toLowerCase() === key)) return;

      pendingAdds.add(key);

      const player = createBlankPlayer(stripped, source);
      player.realName = realName;
      player.nicked = key !== stripped.toLowerCase();
      this.players.push(player);

      const capturedEpoch = fetchEpoch;
      const acquired = await acquireFetchSlot();

      if (!acquired || capturedEpoch !== fetchEpoch) {
        pendingAdds.delete(key);
        const index = this.players.findIndex(
          (player) => player.realName.toLowerCase() === key,
        );
        if (index !== -1) this.players[index].loading = false;
        return;
      }

      try {
        let result: FetchResult | null = null;

        for (let attempt = 0; attempt <= retries; attempt++) {
          if (capturedEpoch !== fetchEpoch) break;

          try {
            const fetchStart = performance.now();
            result = await api.fetch(realName, config.interval, config.mode);
            recordNetwork(`${config.network}.fetch`, performance.now() - fetchStart);
          } catch {
            if (attempt < retries) {
              await sleep(350 * (attempt + 1) + retryJitter());
              continue;
            }
            result = null;
            break;
          }

          if (result.rateLimit && attempt < retries) {
            await sleep(Math.min(1200 * (attempt + 1), 4000) + retryJitter());
            result = null;
            continue;
          }

          break;
        }

        if (capturedEpoch !== fetchEpoch) return;

        const index = this.players.findIndex(
          (player) => player.realName.toLowerCase() === key,
        );
        if (index === -1) return;

        const player = this.players[index];
        if (!result) {
          player.error = 'network';
        } else if (result.notFound) {
          player.nicked = true;
          if (!player.profile && !player.stats) player.error = 'not_found';
        } else if (result.rateLimit) {
          player.error = 'rate_limited';
        } else if (result.statsDisabled) {
          player.profile = normalizeProfile(result.profile);
          player.stats = null;
          player.error = 'stats_disabled';
          const apiName = result.profile?.username;
          if (typeof apiName === 'string' && apiName.length > 0) {
            player.name = apiName;
            player.realName = apiName;
          }
        } else {
          player.profile = normalizeProfile(result.profile);
          player.stats = result.stats;
          const apiName = result.profile?.username;
          if (typeof apiName === 'string' && apiName.length > 0) {
            player.name = apiName;
            player.realName = apiName;
          }
        }
      } finally {
        releaseFetchSlot();
        pendingAdds.delete(key);
        const index = this.players.findIndex(
          (player) => player.realName.toLowerCase() === key,
        );
        if (index !== -1) this.players[index].loading = false;
      }
    },

    async addNames(
      rawNames: string[],
      source: Player['source'] = 'manual',
    ): Promise<void> {
      const uniqueNames = [
        ...new Set(rawNames.map((name) => name.trim()).filter(Boolean)),
      ];
      if (uniqueNames.length === 0) return;

      await Promise.allSettled(
        uniqueNames.map(async (rawName) => this.addByName(rawName, source)),
      );
    },

    async refreshAllStats(interval: Interval, mode: BedwarsMode): Promise<void> {
      const api = getNetworkApi();
      const loaded = this.players.filter(
        (player) => !player.loading && player.error !== 'not_found',
      );
      for (const player of loaded) {
        const index = this.players.findIndex(
          (candidate) => candidate.realName === player.realName,
        );
        if (index !== -1) this.players[index].loading = true;
      }

      const capturedEpoch = fetchEpoch;

      await Promise.allSettled(
        loaded.map(async (player) => {
          const acquired = await acquireFetchSlot();
          if (!acquired || capturedEpoch !== fetchEpoch) {
            const index = this.players.findIndex(
              (candidate) => candidate.realName === player.realName,
            );
            if (index !== -1) this.players[index].loading = false;
            return;
          }
          try {
            const fetchName = player.profile?.username ?? player.realName;
            let stats: PikaBedwarsStats | null = null;

            for (let attempt = 0; attempt <= retries; attempt++) {
              if (capturedEpoch !== fetchEpoch) break;
              try {
                stats = await api.stats(fetchName, interval, mode);
                if (stats !== null) break;
                if (attempt < retries) {
                  await sleep(1000 * (attempt + 1) + retryJitter());
                }
              } catch {
                if (attempt < retries) {
                  await sleep(1000 * 2 ** attempt + retryJitter());
                }
              }
            }

            if (capturedEpoch !== fetchEpoch) return;
            const index = this.players.findIndex(
              (candidate) => candidate.realName === player.realName,
            );
            if (index !== -1) {
              if (stats) this.players[index].stats = stats;
              this.players[index].loading = false;
            }
          } finally {
            releaseFetchSlot();
            const index = this.players.findIndex(
              (candidate) => candidate.realName === player.realName,
            );
            if (index !== -1) this.players[index].loading = false;
          }
        }),
      );
    },

    removeByName(name: string): void {
      const lowerName = name.toLowerCase();
      const index = this.players.findIndex(
        (player) =>
          player.name.toLowerCase() === lowerName ||
          player.realName.toLowerCase() === lowerName,
      );
      if (index !== -1) this.players.splice(index, 1);
    },

    clear(): void {
      resetFetchQueue();
      pendingAdds.clear();
      this.players = [];
      this.playersCount = null;
    },

    setCount(count: number | null): void {
      this.playersCount = count;
    },

    decrementCount(): void {
      if (this.playersCount !== null && this.playersCount > 0) this.playersCount--;
    },

    incrementCount(): void {
      if (this.playersCount !== null) this.playersCount++;
    },

    applyTeams(
      teams: Array<{
        name: string;
        displayName: string;
        color: string;
        players: string[];
      }>,
    ): void {
      for (const player of this.players) {
        player.team = null;
        player.teamColor = null;
      }
      for (const team of teams) {
        for (const memberName of team.players) {
          const normalizedMemberName = memberName.replace(/§./g, '').trim();
          const lowerName = normalizedMemberName.toLowerCase();
          if (!lowerName) continue;
          const index = this.players.findIndex(
            (player) =>
              player.name.toLowerCase() === lowerName ||
              player.realName.toLowerCase() === lowerName,
          );
          if (index !== -1) {
            this.players[index].team = team.name;
            this.players[index].teamColor = team.color;
          }
        }
      }
    },

    clearTeams(): void {
      for (const player of this.players) {
        player.team = null;
        player.teamColor = null;
      }
    },

    setProxyConnectedNetwork(network: string | null): void {
      this.proxyConnectedNetwork = network;
    },
  },
});
