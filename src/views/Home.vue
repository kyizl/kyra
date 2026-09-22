<script setup lang="ts">
import type { Player } from '@renderer/types';
import PlayerRow from '@renderer/components/PlayerRow.vue';
import { Button } from '@renderer/components/ui/button';
import { shouldAutoFitWindow } from '@renderer/lib/window-sizing';
import { useConfigStore } from '@renderer/store/config';
import { usePlayersStore } from '@renderer/store/players';
import {
  Column,
  COLUMNS,
  fmt,
  getRankSortIndex,
  ratio,
  statColor,
  statVal,
} from '@renderer/types';
import { ChevronDown, ChevronUp, Info, Swords, Users, Wifi, X } from 'lucide-vue-next';
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue';

const players = usePlayersStore();
const config = useConfigStore();

const showcase = localStorage.getItem('skip-remove-btn') === '1';

const activeColumns = computed(() => config.activeColumns);
const activeColumnsKey = computed(() => activeColumns.value.join('|'));
const tableRef = ref<HTMLTableElement | null>(null);
const scrollAreaRef = ref<HTMLDivElement | null>(null);

let tableResizeObserver: ResizeObserver | null = null;
let scrollAreaResizeObserver: ResizeObserver | null = null;
let sortHeaderResizeObserver: ResizeObserver | null = null;
let observedSortHeaderCell: HTMLElement | null = null;
let fitWidthRaf = 0;
let lastRequestedContentWidth = 0;

interface ScrollThumbMetrics {
  top: number;
  height: number;
  visible: boolean;
}

interface ScrollThumbXMetrics {
  left: number;
  width: number;
  visible: boolean;
}

const scrollThumb = ref<ScrollThumbMetrics>({ top: 0, height: 0, visible: false });
const isDraggingThumb = ref(false);
const scrollThumbX = ref<ScrollThumbXMetrics>({ left: 0, width: 0, visible: false });
const isDraggingThumbX = ref(false);

const headerRowHeight = 34;
const inset = 6;
const gutter = 8;

const headerVisible = computed(
  () => !config.integratedMode && config.columnLabels !== 'HIDDEN',
);

function updateScrollThumb(): void {
  const el = scrollAreaRef.value;
  if (!el) {
    scrollThumb.value = { top: 0, height: 0, visible: false };
    scrollThumbX.value = { left: 0, width: 0, visible: false };
    return;
  }

  const { scrollTop, scrollHeight, clientHeight, scrollLeft, scrollWidth, clientWidth } =
    el;

  if (scrollHeight <= clientHeight + 1) {
    scrollThumb.value = { top: 0, height: 0, visible: false };
  } else {
    const topInset = headerVisible.value ? Math.max(headerRowHeight, inset) : inset;
    const bottomInset = inset;
    const trackHeight = clientHeight - topInset - bottomInset;
    const thumbHeight = Math.max(28, (trackHeight / scrollHeight) * trackHeight);
    const maxTop = trackHeight - thumbHeight;
    const scrollableMaxY = scrollHeight - clientHeight;
    const top = topInset + (scrollTop / scrollableMaxY) * maxTop;
    scrollThumb.value = { top, height: thumbHeight, visible: true };
  }

  if (scrollWidth <= clientWidth + 1) {
    scrollThumbX.value = { left: 0, width: 0, visible: false };
  } else {
    const leftInset = inset;
    const rightInset = scrollThumb.value.visible ? inset + gutter : inset;
    const trackWidth = clientWidth - leftInset - rightInset;
    const thumbWidth = Math.max(28, (trackWidth / scrollWidth) * trackWidth);
    const maxLeft = trackWidth - thumbWidth;
    const scrollableMaxX = scrollWidth - clientWidth;
    const left = leftInset + (scrollLeft / scrollableMaxX) * maxLeft;
    scrollThumbX.value = { left, width: thumbWidth, visible: true };
  }
}

function onThumbPointerDown(event: PointerEvent): void {
  const el = scrollAreaRef.value;
  if (!el) return;

  event.preventDefault();
  isDraggingThumb.value = true;

  const startY = event.clientY;
  const startScrollTop = el.scrollTop;
  const trackHeight = el.clientHeight;
  const thumbHeight = scrollThumb.value.height;
  const maxTop = trackHeight - thumbHeight;
  const scrollableMax = el.scrollHeight - el.clientHeight;

  function onMove(moveEvent: PointerEvent): void {
    if (maxTop <= 0 || scrollableMax <= 0) return;
    const deltaY = moveEvent.clientY - startY;
    scrollAreaRef.value!.scrollTop = startScrollTop + (deltaY / maxTop) * scrollableMax;
  }

  function onUp(): void {
    isDraggingThumb.value = false;
    window.removeEventListener('pointermove', onMove);
    window.removeEventListener('pointerup', onUp);
  }

  window.addEventListener('pointermove', onMove);
  window.addEventListener('pointerup', onUp);
}

function onThumbXPointerDown(event: PointerEvent): void {
  const el = scrollAreaRef.value;
  if (!el) return;

  event.preventDefault();
  isDraggingThumbX.value = true;

  const startX = event.clientX;
  const startScrollLeft = el.scrollLeft;
  const trackWidth = el.clientWidth;
  const thumbWidth = scrollThumbX.value.width;
  const maxLeft = trackWidth - thumbWidth;
  const scrollableMax = el.scrollWidth - el.clientWidth;

  function onMove(moveEvent: PointerEvent): void {
    if (maxLeft <= 0 || scrollableMax <= 0) return;
    const deltaX = moveEvent.clientX - startX;
    scrollAreaRef.value!.scrollLeft =
      startScrollLeft + (deltaX / maxLeft) * scrollableMax;
  }

  function onUp(): void {
    isDraggingThumbX.value = false;
    window.removeEventListener('pointermove', onMove);
    window.removeEventListener('pointerup', onUp);
  }

  window.addEventListener('pointermove', onMove);
  window.addEventListener('pointerup', onUp);
}

interface SortColRect {
  left: number;
  width: number;
}

const sortColRect = ref<SortColRect | null>(null);

function observeSortHeaderCell(cell: HTMLElement | null): void {
  if (cell === observedSortHeaderCell) return;
  if (observedSortHeaderCell) {
    sortHeaderResizeObserver?.unobserve(observedSortHeaderCell);
  }
  observedSortHeaderCell = cell;
  if (cell) {
    sortHeaderResizeObserver ??= new ResizeObserver(() => updateSortSpotlight());
    sortHeaderResizeObserver.observe(cell);
  }
}

function updateSortSpotlight(): void {
  const tableEl = tableRef.value;
  const containerEl = scrollAreaRef.value;
  if (!tableEl || !containerEl) {
    sortColRect.value = null;
    observeSortHeaderCell(null);
    return;
  }
  const headerCell = tableEl.querySelector<HTMLElement>('th[data-sort-col="true"]');
  observeSortHeaderCell(headerCell);
  if (!headerCell) {
    sortColRect.value = null;
    return;
  }
  const cellRect = headerCell.getBoundingClientRect();
  const containerRect = containerEl.getBoundingClientRect();
  sortColRect.value = {
    left: cellRect.left - containerRect.left + containerEl.scrollLeft,
    width: cellRect.width,
  };
}

const spotlightStyle = computed(() => {
  const rect = sortColRect.value;
  if (!rect) return {};
  return {
    backgroundImage:
      'linear-gradient(to bottom, rgba(var(--color-accent-rgb), 0.08) 0%, ' +
      'rgba(var(--color-accent-rgb), 0.032) 22%, rgba(var(--color-accent-rgb), 0.012) 50%, ' +
      'transparent 80%)',
    backgroundRepeat: 'no-repeat',
    backgroundPosition: `${rect.left}px 0`,
    backgroundSize: `${rect.width}px 100%`,
    backgroundAttachment: 'local',
  } as const;
});

function requestFitWidth(): void {
  if (fitWidthRaf) cancelAnimationFrame(fitWidthRaf);
  fitWidthRaf = requestAnimationFrame(() => {
    fitWidthRaf = 0;
    const tableEl = tableRef.value;
    if (!tableEl) return;

    updateSortSpotlight();
    updateScrollThumb();

    const desiredContentWidth = Math.ceil(tableEl.scrollWidth);
    const currentWindowWidth = window.innerWidth;
    const shouldFit = shouldAutoFitWindow({
      currentWidth: currentWindowWidth,
      desiredContentWidth,
      lastRequestedContentWidth,
      minGrowDelta: 24,
      minContentDelta: 12,
      maxGrowDelta: 480,
    });

    if (!shouldFit) return;

    lastRequestedContentWidth = desiredContentWidth;
    window.api.win.fitContentWidth(desiredContentWidth);
  });
}

watch(
  tableRef,
  (newTable, oldTable) => {
    if (oldTable) {
      tableResizeObserver?.disconnect();
      observeSortHeaderCell(null);
    }
    if (newTable) {
      tableResizeObserver ??= new ResizeObserver(() => requestFitWidth());
      tableResizeObserver.observe(newTable);
      requestFitWidth();
    }
  },
  { flush: 'post' },
);

watch(
  scrollAreaRef,
  (newArea, oldArea) => {
    if (oldArea) {
      scrollAreaResizeObserver?.disconnect();
    }
    if (newArea) {
      scrollAreaResizeObserver ??= new ResizeObserver(() => updateScrollThumb());
      scrollAreaResizeObserver.observe(newArea);
      updateScrollThumb();
    } else {
      scrollThumb.value = { top: 0, height: 0, visible: false };
    }
  },
  { flush: 'post' },
);

watch(
  () => [config.sortBy, config.sortAscending, activeColumnsKey.value] as const,
  () => {
    void nextTick(updateSortSpotlight);
  },
);

const rosterKey = computed(() =>
  players.players.map((player) => player.realName).join('\u0000'),
);

watch(rosterKey, () => {
  void nextTick(updateSortSpotlight);
});

watch(headerVisible, () => {
  void nextTick(updateScrollThumb);
});

onBeforeUnmount(() => {
  tableResizeObserver?.disconnect();
  scrollAreaResizeObserver?.disconnect();
  sortHeaderResizeObserver?.disconnect();
  if (fitWidthRaf) cancelAnimationFrame(fitWidthRaf);
});

function colLabel(col: Column): string {
  if (config.columnLabels === 'HIDDEN') return '';
  if (config.columnLabels === 'SHORT') return COLUMNS[col].shortLabel;
  return COLUMNS[col].label;
}

function toggleSort(col: Column): void {
  if (config.sortBy === col) {
    config.sortAscending = !config.sortAscending;
  } else {
    config.sortBy = col;
    config.sortAscending = false;
  }
}

function getSortKey(player: Player): string | number {
  const columnDef = COLUMNS[config.sortBy];
  if (config.sortBy === Column.NAME && columnDef.getNum) return columnDef.getNum(player);
  if (columnDef.getNum && !columnDef.getStr) return columnDef.getNum(player);
  if (columnDef.getStr) return columnDef.getStr(player) ?? '';
  return 0;
}

function getDataTier(player: Player): number {
  if (player.loading || player.error || !player.stats) return 2;
  const stats = player.stats;
  const hasAny =
    statVal(stats['Final kills']) > 0 ||
    statVal(stats['Final deaths']) > 0 ||
    statVal(stats.Wins) > 0 ||
    statVal(stats.Losses) > 0 ||
    statVal(stats.Kills) > 0 ||
    statVal(stats.Deaths) > 0;
  return hasAny ? 0 : 1;
}

function sortPlayers(list: Player[]): Player[] {
  const count = list.length;
  if (count <= 1) return [...list];
  const keys = new Array<string | number>(count);
  const tiers = new Array<number>(count);
  const sortByName = config.sortBy === Column.NAME;
  const rankIndices = sortByName ? new Array<number>(count) : null;
  for (let i = 0; i < count; i++) {
    const player = list[i];
    keys[i] = getSortKey(player);
    tiers[i] = getDataTier(player);
    if (rankIndices) rankIndices[i] = getRankSortIndex(player.profile);
  }
  const indices = Array.from({ length: count }, (_, i) => i);
  const sortDirection = config.sortAscending ? 1 : -1;
  indices.sort((indexA, indexB) => {
    const playerA = list[indexA];
    const playerB = list[indexB];
    if (playerA.loading && !playerB.loading) return 1;
    if (!playerA.loading && playerB.loading) return -1;
    if (sortByName) {
      const rankA = rankIndices![indexA];
      const rankB = rankIndices![indexB];
      if (rankA !== rankB) return rankA - rankB;
      const tierA = tiers[indexA];
      const tierB = tiers[indexB];
      if (tierA !== tierB) return tierA - tierB;
    }
    const keyA = keys[indexA];
    const keyB = keys[indexB];
    if (typeof keyA === 'number' && typeof keyB === 'number') {
      return sortByName ? (keyA - keyB) * sortDirection : (keyB - keyA) * sortDirection;
    }
    return String(keyA).localeCompare(String(keyB)) * sortDirection;
  });
  return indices.map((index) => list[index]);
}

const sortedPlayers = computed((): Player[] => sortPlayers(players.players));

const hasTeamData = computed(() =>
  players.players.some((player) => player.team !== null),
);

interface TeamGroup {
  name: string;
  color: string;
  players: Player[];
}

const teamGroups = computed((): TeamGroup[] => {
  if (!hasTeamData.value) return [];
  const map = new Map<string, { name: string; color: string; players: Player[] }>();
  for (const player of players.players) {
    if (!player.team) continue;
    const key = player.team;
    if (!map.has(key)) {
      map.set(key, {
        name: player.team,
        color: player.teamColor ?? '#AAAAAA',
        players: [],
      });
    }
    map.get(key)!.players.push(player);
  }
  return [...map.values()]
    .sort((groupA, groupB) => groupA.name.localeCompare(groupB.name))
    .map((group) => {
      const sorted = sortPlayers(group.players);
      return {
        name: group.name,
        color: group.color,
        players: sorted,
      };
    });
});

const ungroupedPlayers = computed((): Player[] =>
  hasTeamData.value ? sortPlayers(players.players.filter((player) => !player.team)) : [],
);

function removePlayer(name: string): void {
  players.removeByName(name);
}

const avgFkdrStats = computed(() => {
  let count = 0;
  let sum = 0;
  for (const player of players.players) {
    if (!player.stats || player.loading) continue;
    sum += ratio(
      statVal(player.stats['Final kills']),
      statVal(player.stats['Final deaths']),
    );
    count++;
  }
  if (!count) return null;
  return sum / count;
});

const avgFkdr = computed(() =>
  avgFkdrStats.value === null ? '—' : fmt(avgFkdrStats.value),
);

const avgFkdrColor = computed(() =>
  avgFkdrStats.value === null
    ? 'var(--color-ink-3)'
    : statColor(avgFkdrStats.value, [0, 1, 2, 4, 7, 12, 20, 35]),
);

const proxyConnected = computed(() => players.proxyConnectedNetwork !== null);
const showProxyBanner = computed(
  () => !proxyConnected.value && !config.proxyBannerDismissed,
);

watch(
  () => players.proxyConnectedNetwork,
  (newVal, oldVal) => {
    if (newVal === null && oldVal !== null) {
      config.proxyBannerDismissed = false;
    }
  },
);

function dismissProxyBanner(): void {
  config.proxyBannerDismissed = true;
}

const activeNetworkLabel = computed(() =>
  config.network === 'jartexnetwork' ? 'JartexNetwork' : 'PikaNetwork',
);
const activeNetworkPort = computed(() =>
  config.network === 'jartexnetwork' ? config.jartexProxyPort : config.pikaProxyPort,
);
</script>

<template>
  <div class="app-shell flex h-full flex-col overflow-hidden">
    <div
      v-if="showProxyBanner"
      class="no-drag proxy-banner flex shrink-0 items-center justify-between gap-3 px-3 py-2"
    >
      <div class="flex min-w-0 items-center gap-2">
        <div
          class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full"
          style="
            background: rgba(var(--color-accent-rgb), 0.12);
            border: 1px solid rgba(var(--color-accent-rgb), 0.28);
          "
        >
          <Wifi
            :size="16"
            style="color: var(--color-accent-light)"
          />
        </div>
        <div class="min-w-0">
          <div
            style="
              font-size: 0.85rem;
              font-weight: 600;
              color: var(--color-accent-light);
              line-height: 1.2;
            "
          >
            Proxy not connected
          </div>
          <div
            class="truncate"
            style="
              font-size: 0.8rem;
              color: var(--color-ink-3);
              line-height: 1.35;
              margin-top: 1px;
            "
          >
            Connect to {{ activeNetworkLabel }} using
            <span
              class="font-mono"
              style="color: var(--color-ink-2)"
            >
              {{
                config.proxyBindHost === '0.0.0.0' ? '&lt;your-ip&gt;' : 'localhost'
              }}:{{ activeNetworkPort }}
            </span>
            to enable team detection and automation features.
          </div>
        </div>
      </div>
      <div class="flex shrink-0 items-center gap-1.5">
        <Button
          as="router-link"
          to="/settings?tab=advanced"
          variant="control"
        >
          Configure
        </Button>
        <Button
          variant="control-icon"
          size="icon-sm"
          style="
            width: 22px;
            height: 22px;
            border-radius: var(--radius-sm);
            background: rgba(255, 255, 255, 0.05);
            border: 1px solid rgba(255, 255, 255, 0.08);
            color: var(--color-ink-3);
          "
          @click="dismissProxyBanner"
        >
          <X :size="9" />
        </Button>
      </div>
    </div>

    <div
      v-if="config.missingPlayersWarning && players.missingCount > 0"
      class="flex shrink-0 items-center gap-2 px-3.5 py-1.5"
      style="
        background: rgba(248, 113, 113, 0.05);
        border-bottom: 1px solid rgba(248, 113, 113, 0.12);
        color: var(--color-bad);
      "
    >
      <Info :size="11" />
      <span style="font-size: 0.79rem">
        {{ players.missingCount }} player{{ players.missingCount !== 1 ? 's' : '' }}
        missing — type
        <code
          class="rounded px-1.5 py-0.5 font-mono"
          style="background: rgba(255, 255, 255, 0.07); font-size: 0.75rem"
          >/who</code
        >
        in-game
      </span>
    </div>

    <div
      class="relative flex flex-1 flex-col overflow-hidden"
      :class="config.integratedMode ? 'stats-box-integrated' : 'stats-box'"
    >
      <div
        v-if="players.players.length === 0"
        class="flex flex-1 flex-col items-center justify-center gap-3"
      >
        <div class="relative">
          <div
            class="animate-pulse-dot absolute inset-0 rounded-full"
            style="
              background: radial-gradient(
                circle,
                rgba(var(--color-accent-rgb), 0.12) 0%,
                transparent 70%
              );
              transform: scale(2.2);
            "
          />
          <div
            class="relative flex h-14 w-14 items-center justify-center rounded-full"
            style="
              background: rgba(var(--color-accent-rgb), 0.08);
              border: 1px solid rgba(var(--color-accent-rgb), 0.2);
            "
          >
            <Users
              :size="22"
              style="color: rgba(var(--color-accent-rgb), 0.5)"
            />
          </div>
        </div>
        <div class="text-center">
          <div
            class="mb-0.5 font-semibold"
            style="font-size: 0.85rem; color: var(--color-ink-2)"
          >
            No players yet
          </div>
          <div style="font-size: 0.76rem; color: var(--color-ink-3)">
            Join a game or add players manually.
          </div>
        </div>
      </div>

      <div
        v-else
        ref="scrollAreaRef"
        class="themed-scroll sort-spotlight-container flex-1 overflow-x-auto overflow-y-auto"
        :style="spotlightStyle"
        @scroll="updateScrollThumb"
      >
        <template v-if="hasTeamData">
          <table
            ref="tableRef"
            class="w-full border-separate"
            style="border-spacing: 0"
          >
            <thead
              v-if="!config.integratedMode && config.columnLabels !== 'HIDDEN'"
              class="table-header-bar sticky top-0 z-20"
            >
              <tr>
                <th
                  v-for="col in activeColumns"
                  :key="col"
                  class="table-header-cell px-2.5 py-2 text-center font-bold select-none"
                  :class="[
                    COLUMNS[col].sortable
                      ? 'table-header-cell--sortable cursor-pointer'
                      : '',
                    config.sortBy === col ? 'table-header-cell--active' : '',
                  ]"
                  :data-sort-col="config.sortBy === col"
                  :role="COLUMNS[col].sortable ? 'button' : undefined"
                  :tabindex="COLUMNS[col].sortable ? 0 : undefined"
                  @click="COLUMNS[col].sortable && toggleSort(col)"
                  @keydown.enter="COLUMNS[col].sortable && toggleSort(col)"
                  @keydown.space.prevent="COLUMNS[col].sortable && toggleSort(col)"
                >
                  <div class="table-header-label table-header-label--center">
                    <span class="table-header-chevron table-header-chevron--slot" />
                    <span>{{ colLabel(col) }}</span>
                    <component
                      :is="config.sortAscending ? ChevronUp : ChevronDown"
                      :size="9"
                      class="table-header-chevron table-header-chevron--slot"
                      :class="{ 'table-header-chevron--visible': config.sortBy === col }"
                      aria-hidden="true"
                    />
                  </div>
                </th>
                <th
                  v-if="!showcase"
                  class="w-6"
                />
              </tr>
            </thead>
            <tbody>
              <template
                v-for="group in teamGroups"
                :key="group.name"
              >
                <tr class="team-header-row-tr">
                  <td
                    :colspan="activeColumns.length + 1"
                    style="padding: 0"
                  >
                    <div
                      class="team-bar"
                      :style="{ '--team-color': group.color }"
                    >
                      <div class="team-bar-flag">
                        <span class="team-bar-name">{{ group.name }}</span>
                      </div>
                      <span class="team-bar-count">
                        {{ group.players.length
                        }}{{ group.players.length === 1 ? ' PLAYER' : ' PLAYERS' }}
                      </span>
                    </div>
                  </td>
                </tr>
                <PlayerRow
                  v-for="player in group.players"
                  :key="player.realName"
                  :player="player"
                  :active-columns="activeColumns"
                  @remove="removePlayer"
                />
              </template>
              <template v-if="ungroupedPlayers.length > 0">
                <tr>
                  <td
                    :colspan="activeColumns.length + 1"
                    style="padding: 0"
                  >
                    <div
                      class="team-bar"
                      style="--team-color: #8a8a8a"
                    >
                      <div class="team-bar-flag team-bar-flag--muted">
                        <span class="team-bar-name">UNASSIGNED</span>
                      </div>
                      <span class="team-bar-count">
                        {{ ungroupedPlayers.length
                        }}{{ ungroupedPlayers.length === 1 ? ' PLAYER' : ' PLAYERS' }}
                      </span>
                    </div>
                  </td>
                </tr>
                <PlayerRow
                  v-for="player in ungroupedPlayers"
                  :key="player.realName"
                  v-memo="[player.realName, activeColumnsKey]"
                  :player="player"
                  :active-columns="activeColumns"
                  @remove="removePlayer"
                />
              </template>
            </tbody>
          </table>
        </template>

        <template v-else>
          <table
            ref="tableRef"
            class="w-full border-separate"
            style="border-spacing: 0"
          >
            <thead
              v-if="!config.integratedMode && config.columnLabels !== 'HIDDEN'"
              class="table-header-bar sticky top-0 z-20"
            >
              <tr>
                <th
                  v-for="col in activeColumns"
                  :key="col"
                  class="table-header-cell px-2.5 py-2 text-center font-bold select-none"
                  :class="[
                    COLUMNS[col].sortable
                      ? 'table-header-cell--sortable cursor-pointer'
                      : '',
                    config.sortBy === col ? 'table-header-cell--active' : '',
                  ]"
                  :data-sort-col="config.sortBy === col"
                  :role="COLUMNS[col].sortable ? 'button' : undefined"
                  :tabindex="COLUMNS[col].sortable ? 0 : undefined"
                  @click="COLUMNS[col].sortable && toggleSort(col)"
                  @keydown.enter="COLUMNS[col].sortable && toggleSort(col)"
                  @keydown.space.prevent="COLUMNS[col].sortable && toggleSort(col)"
                >
                  <div class="table-header-label table-header-label--center">
                    <span class="table-header-chevron table-header-chevron--slot" />
                    <span>{{ colLabel(col) }}</span>
                    <component
                      :is="config.sortAscending ? ChevronUp : ChevronDown"
                      :size="9"
                      class="table-header-chevron table-header-chevron--slot"
                      :class="{ 'table-header-chevron--visible': config.sortBy === col }"
                      aria-hidden="true"
                    />
                  </div>
                </th>
                <th
                  v-if="!showcase"
                  class="w-6"
                />
              </tr>
            </thead>
            <tbody>
              <PlayerRow
                v-for="player in sortedPlayers"
                :key="player.realName"
                v-memo="[player.realName, activeColumnsKey]"
                :player="player"
                :active-columns="activeColumns"
                @remove="removePlayer"
              />
            </tbody>
          </table>
        </template>
      </div>

      <div
        v-if="scrollThumb.visible"
        class="no-drag custom-scrollbar-thumb"
        :class="{ 'custom-scrollbar-thumb--active': isDraggingThumb }"
        :style="{ top: `${scrollThumb.top}px`, height: `${scrollThumb.height}px` }"
        @pointerdown="onThumbPointerDown"
      />
      <div
        v-if="scrollThumbX.visible"
        class="no-drag custom-scrollbar-thumb-x"
        :class="{ 'custom-scrollbar-thumb-x--active': isDraggingThumbX }"
        :style="{ left: `${scrollThumbX.left}px`, width: `${scrollThumbX.width}px` }"
        @pointerdown="onThumbXPointerDown"
      />
    </div>
  </div>

  <div
    v-if="players.players.length > 0 && !config.integratedMode"
    class="no-drag flex shrink-0 items-center justify-between px-3.5 py-1.5"
    style="transform: translateY(-5px)"
  >
    <div class="flex items-center gap-2">
      <span style="font-size: 0.74rem; color: var(--color-ink-2); font-weight: 500">
        {{ players.players.length }} player{{ players.players.length !== 1 ? 's' : '' }}
      </span>
      <span
        v-if="hasTeamData"
        class="flex items-center gap-1 rounded px-1.5 py-0.5"
        style="
          font-size: 0.6rem;
          font-weight: 600;
          background: rgba(var(--color-accent-rgb), 0.12);
          border: 1px solid rgba(var(--color-accent-rgb), 0.25);
          color: var(--color-accent-light);
        "
      >
        <Swords :size="8" />
        {{ teamGroups.length }} teams
      </span>
    </div>
    <span
      class="font-semibold"
      style="font-size: 0.74rem"
      :style="{ color: avgFkdrColor }"
      >avg {{ avgFkdr }} FKDR</span
    >
  </div>
</template>

<style scoped>
.app-shell {
  background: var(--panel-bg);
  box-shadow: none !important;
  filter: none;
}

.stats-box {
  position: relative;
  margin: 0 10px 10px;
  border-radius: 16px;
  overflow: hidden;

  background: rgba(255, 255, 255, 0.022);
  border: 1px solid rgba(var(--color-accent-rgb), 0.45);

  backdrop-filter: blur(24px) saturate(115%);
  -webkit-backdrop-filter: blur(24px) saturate(115%);

  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.055),
    inset 0 0 0 1px rgba(var(--color-accent-rgb), 0.18),
    inset 0 0 18px rgba(var(--color-accent-rgb), 0.16);
}

html.low-end .stats-box,
html.window-unfocused .stats-box {
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}

.stats-box-integrated {
  position: relative;
  margin: 0;
  border-radius: 0;
  overflow: hidden;

  background: transparent;
  border: none;
  box-shadow: none;
  backdrop-filter: none;
  -webkit-backdrop-filter: none;

  filter: saturate(1.45) contrast(1.18);
  text-shadow:
    0 1px 3px rgba(0, 0, 0, 0.9),
    0 0 6px rgba(0, 0, 0, 0.6);
}

.sort-spotlight-container {
  position: relative;
}

.table-header-bar {
  background:
    linear-gradient(var(--color-surface-2), var(--color-surface-2)), var(--color-bg);
  border-bottom: 1px solid var(--color-border);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.04);
  isolation: isolate;
  transform: translateZ(0);
}

.table-header-bar th {
  background:
    linear-gradient(var(--color-surface-2), var(--color-surface-2)), var(--color-bg);
}

.table-header-cell {
  vertical-align: middle;
  font-size: 0.68rem;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--color-accent-light);
  transition:
    color 150ms ease,
    background 150ms ease;
}

.table-header-cell--sortable:hover {
  color: var(--color-ink-1);
  background:
    linear-gradient(var(--color-surface-3), var(--color-surface-3)), var(--color-bg);
}

.table-header-cell--active {
  box-shadow: inset 0 -2px 0 var(--color-accent);
}

.table-header-cell--active.table-header-cell--sortable:hover {
  background:
    linear-gradient(
      rgba(var(--color-accent-rgb), 0.1),
      rgba(var(--color-accent-rgb), 0.1)
    ),
    var(--color-bg);
}

.table-header-chevron {
  color: var(--color-accent-light);
  flex-shrink: 0;
}

.table-header-label {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  line-height: 1;
  position: relative;
  top: 3.3px;
}

.table-header-label--center {
  justify-content: center;
}

.table-header-chevron--slot {
  width: 9px;
  height: 9px;
  visibility: hidden;
}

.table-header-chevron--slot.table-header-chevron--visible {
  visibility: visible;
}

.themed-scroll {
  scrollbar-width: none;
}

.themed-scroll::-webkit-scrollbar {
  display: none;
  width: 0;
  height: 0;
}

.custom-scrollbar-thumb {
  position: absolute;
  top: 6px;
  right: 0;
  width: 5px;
  border-radius: 999px;
  background: rgba(var(--color-accent-rgb), 0.32);
  cursor: pointer;
  -webkit-app-region: no-drag;
  transition:
    background 140ms ease,
    width 140ms ease;
  touch-action: none;
}

.custom-scrollbar-thumb:hover,
.custom-scrollbar-thumb--active {
  background: rgba(var(--color-accent-rgb), 0.55);
  width: 6px;
}

.custom-scrollbar-thumb-x {
  position: absolute;
  left: 0;
  bottom: 0;
  height: 5px;
  border-radius: 999px;
  background: rgba(var(--color-accent-rgb), 0.32);
  cursor: pointer;
  -webkit-app-region: no-drag;
  transition:
    background 140ms ease,
    height 140ms ease;
  touch-action: none;
}

.custom-scrollbar-thumb-x:hover,
.custom-scrollbar-thumb-x--active {
  background: rgba(var(--color-accent-rgb), 0.55);
  height: 6px;
}

.proxy-banner {
  position: relative;
  overflow: hidden;
  margin-bottom: 10px;
  background: var(--panel-bg);
  backdrop-filter: var(--panel-blur);
  -webkit-backdrop-filter: var(--panel-blur);
  border-bottom: 1px solid rgba(var(--color-accent-rgb), 0.18);
}
.proxy-banner::before {
  content: '';
  position: absolute;
  inset: 0;
  width: 300%;
  left: -100%;
  background: linear-gradient(
    105deg,
    rgba(var(--color-accent-rgb), 0.07) 0%,
    rgba(var(--color-accent-rgb), 0.03) 50%,
    rgba(var(--color-accent-rgb), 0.05) 100%
  );
  will-change: transform;
  animation: banner-shimmer 6s ease-in-out infinite;
  animation-play-state: var(--anim-play-state, running);
  pointer-events: none;
}
.team-bar {
  position: relative;
  display: flex;
  align-items: stretch;
  height: 28px;
  margin: 5px 0 2px;
  overflow: hidden;
  background: rgba(255, 255, 255, 0.025);
  border-top: 1px solid rgba(255, 255, 255, 0.035);
  border-bottom: 1px solid rgba(255, 255, 255, 0.025);
}
.team-bar-flag {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  flex-shrink: 0;
  min-width: 142px;
  padding: 0 25px 0 13px;
  background: var(--team-color);
  clip-path: polygon(0 0, 100% 0, calc(100% - 15px) 100%, 0 100%);
}
.team-bar-flag--muted {
  background: var(--team-color);
}
.team-bar-flag--muted .team-bar-name {
  color: rgba(20, 20, 20, 0.84);
}
.team-bar-name {
  position: relative;
  z-index: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  font-size: 0.65rem;
  font-weight: 800;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: #111;
  text-shadow: 0 1px 0 rgba(255, 255, 255, 0.25);
  white-space: nowrap;
}
.team-bar-count {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  margin-left: 12px;
  font-size: 0.56rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  color: color-mix(in srgb, var(--color-accent) 62%, white 8%);
  white-space: nowrap;
}
</style>
