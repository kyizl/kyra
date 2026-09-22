<script setup lang="ts">
import type { Player } from '@renderer/types';
import { resolveStaffSubrole } from '@renderer/lib/staff-subroles';
import { useConfigStore } from '@renderer/store/config';
import {
  Column,
  COLUMNS,
  fmt,
  getTopRankDisplay,
  getTopRankName,
  isStaff,
  levelColor,
  playerNameColor,
  statColor,
} from '@renderer/types';
import { X } from 'lucide-vue-next';
import { computed, ref, useId, watch } from 'vue';
import PlayerAvatar from './PlayerAvatar.vue';

const props = defineProps<{
  player: Player;
  activeColumns: Column[];
}>();

defineEmits<{
  remove: [name: string];
}>();

const uid = useId();
const lockGlowId = `lock-glow-${uid}`;
const ccStrokeId = `cc-stroke-${uid}`;
const ccFillId = `cc-fill-${uid}`;

const showcase = localStorage.getItem('skip-remove-btn') === '1';

const config = useConfigStore();
const staffBadgeLabel = ref('Staff');

watch(
  () => ({
    active: isStaff(props.player.profile),
    network: config.network,
    rankName: getTopRankName(props.player.profile),
    username: props.player.profile?.username ?? props.player.realName ?? null,
  }),
  (params, _previous, onCleanup) => {
    if (!params.active || !params.username) {
      staffBadgeLabel.value = 'Staff';
      return;
    }

    let cancelled = false;
    onCleanup(() => {
      cancelled = true;
    });

    resolveStaffSubrole(params.network, params.rankName, params.username)
      .then((subrole) => {
        if (!cancelled) staffBadgeLabel.value = subrole ?? 'Staff';
      })
      .catch(() => {
        if (!cancelled) staffBadgeLabel.value = 'Staff';
      });
  },
  { immediate: true, deep: true },
);

const derived = computed(() => {
  const profile = props.player.profile;
  const tag = profile?.clan?.tag ?? null;
  const statsDisabled = props.player.error === 'stats_disabled';

  let parsedClanTag: ClanSegment[] | null = null;
  if (tag) {
    const result: ClanSegment[] = [];
    for (const char of tag) {
      if (/[a-z0-9 ]/i.test(char)) {
        const last = result[result.length - 1];
        if (last && last.type === 'text') last.value += char;
        else result.push({ type: 'text', value: char });
      } else {
        result.push({ type: 'icon', value: char });
      }
    }
    parsedClanTag = result.length > 0 ? result : null;
  }

  return {
    nameColor: playerNameColor(profile),
    topRankDisplay: getTopRankDisplay(profile),
    staffPlayer: isStaff(profile),
    statsDisabled,
    parsedClanTag,
    privateLevel: statsDisabled
      ? typeof profile?.rank?.level === 'number'
        ? profile.rank.level
        : null
      : null,
  };
});

type ClanSegment = { type: 'icon'; value: string } | { type: 'text'; value: string };

const errorLabel = computed(() => {
  switch (props.player.error) {
    case 'rate_limited':
      return '429';

    case 'network':
      return 'ERR';

    case 'stats_disabled':
      return 'PRIVATE';

    default:
      return '?';
  }
});

const errorColor = computed(() => {
  switch (props.player.error) {
    case 'not_found':
      return 'var(--color-ink-3)';

    case 'rate_limited':
      return '#f97316';

    case 'stats_disabled':
      return 'var(--color-ink-3)';

    default:
      return 'var(--color-bad)';
  }
});

function cellValue(column: Column): string {
  const columnDef = COLUMNS[column];

  if (columnDef.fromProfile) {
    if (!props.player.profile) return '—';

    if (columnDef.getNum) {
      return fmt(columnDef.getNum(props.player));
    }

    return '—';
  }

  if (columnDef.getStr) {
    return columnDef.getStr(props.player) ?? '—';
  }

  if (!columnDef.getNum) return '—';

  return fmt(columnDef.getNum(props.player));
}

function cellColor(column: Column): string {
  const columnDef = COLUMNS[column];

  if (columnDef.getColor) {
    return columnDef.getColor(props.player);
  }

  if (!columnDef.thresholds || !columnDef.getNum) {
    return 'var(--color-ink-2)';
  }

  return statColor(columnDef.getNum(props.player), columnDef.thresholds);
}
</script>

<template>
  <tr
    class="group player-row border-b"
    style="border-color: rgba(255, 255, 255, 0.035); overflow: visible !important"
    :class="{
      'opacity-30': player.error === 'not_found' && !player.nicked,
      'nicked-row': player.nicked,
    }"
  >
    <td
      v-for="column in activeColumns"
      :key="column"
      class="cell whitespace-nowrap"
      :style="{ overflow: 'visible !important' }"
      :class="column === Column.NAME ? 'name-cell text-left' : 'text-center'"
    >
      <template v-if="column === Column.NAME">
        <div class="name-wrapper">
          <div
            v-if="player.teamColor"
            class="team-indicator"
            :style="{
              background: player.teamColor,
            }"
          />

          <PlayerAvatar
            :name="player.realName || player.name"
            :size="17"
            class="avatar"
          />

          <span
            v-if="derived.topRankDisplay"
            class="rank-text"
            :style="{ color: derived.nameColor }"
          >
            {{ derived.topRankDisplay }}
          </span>

          <span
            class="name-text truncate"
            :style="{ color: derived.nameColor }"
          >
            <template v-if="player.nicked && player.name !== player.realName">
              <span style="color: var(--color-nick)">
                {{ player.name }}
              </span>

              <span class="arrow-separator"> → </span>

              <span>
                {{ player.realName }}
              </span>
            </template>

            <template v-else>
              {{ player.realName || player.name }}
            </template>
          </span>

          <span
            v-if="derived.parsedClanTag"
            class="clan-tag-wrapper"
          >
            <span class="clan-bracket">[</span>

            <template
              v-for="(segment, i) in derived.parsedClanTag"
              :key="i"
            >
              <span
                v-if="segment.type === 'icon'"
                class="clan-icon"
                >{{ segment.value }}</span
              >

              <span
                v-else
                class="clan-text-inner"
                >{{ segment.value }}</span
              >
            </template>

            <span class="clan-bracket">]</span>
          </span>

          <span
            v-if="player.nicked"
            class="nick-icon-wrapper"
          >
            <img
              src="/nick.svg"
              alt="nick"
              class="nick-badge"
            />
            <span class="tooltip nick-tooltip">The player is nicked</span>
          </span>

          <span
            v-if="derived.staffPlayer"
            class="badge badge-staff"
          >
            {{ staffBadgeLabel }}
          </span>

          <span
            v-if="derived.statsDisabled"
            class="lock-icon-wrapper"
          >
            <svg
              width="15"
              height="15"
              viewBox="0 0 15 15"
              fill="none"
            >
              <defs>
                <filter
                  :id="lockGlowId"
                  x="-50%"
                  y="-50%"
                  width="200%"
                  height="200%"
                >
                  <feGaussianBlur
                    stdDeviation="1.7"
                    result="blur"
                  />
                  <feMerge>
                    <feMergeNode in="blur" />
                    <feMergeNode in="SourceGraphic" />
                  </feMerge>
                </filter>

                <linearGradient
                  :id="ccStrokeId"
                  x1="1"
                  y1="1"
                  x2="14"
                  y2="14"
                >
                  <stop
                    offset="0%"
                    stop-color="#ff7ad9"
                  />
                  <stop
                    offset="50%"
                    stop-color="#c084fc"
                  />
                  <stop
                    offset="100%"
                    stop-color="#60dfff"
                  />
                </linearGradient>

                <linearGradient
                  :id="ccFillId"
                  x1="1"
                  y1="6"
                  x2="14"
                  y2="14"
                >
                  <stop
                    offset="0%"
                    stop-color="#ff7ad9"
                    stop-opacity="0.24"
                  />
                  <stop
                    offset="100%"
                    stop-color="#60dfff"
                    stop-opacity="0.24"
                  />
                </linearGradient>
              </defs>

              <g :filter="`url(#${lockGlowId})`">
                <path
                  d="M4.5 6.5V4.5a3 3 0 0 1 6 0v2"
                  :stroke="`url(#${ccStrokeId})`"
                  stroke-width="1.5"
                  stroke-linecap="round"
                />

                <rect
                  x="2.5"
                  y="6.5"
                  width="10"
                  height="7"
                  rx="2"
                  :fill="`url(#${ccFillId})`"
                  :stroke="`url(#${ccStrokeId})`"
                  stroke-width="1.5"
                />

                <circle
                  cx="7.5"
                  cy="10"
                  r="1"
                  fill="#ffffff"
                  opacity="0.98"
                />
              </g>
            </svg>
            <span class="tooltip lock-tooltip"
              >Public stats are disabled for the player</span
            >
          </span>
        </div>
      </template>

      <template v-else-if="player.loading">
        <span
          class="animate-shimmer inline-block rounded"
          style="width: 40px; height: 9px; display: inline-block"
        />
      </template>

      <template v-else-if="player.error">
        <span
          v-if="
            player.error !== 'not_found' &&
            player.error !== 'stats_disabled' &&
            (column === Column.FKDR || activeColumns.indexOf(column) === 1)
          "
          class="stat-text"
          :style="{ color: errorColor }"
        >
          {{ errorLabel }}
        </span>

        <span
          v-else-if="
            player.error === 'stats_disabled' &&
            column === Column.LEVEL &&
            derived.privateLevel !== null
          "
          class="stat-text"
          :style="{ color: levelColor(derived.privateLevel!) }"
        >
          {{ derived.privateLevel }}
        </span>

        <span
          v-else
          class="stat-empty"
        >
          —
        </span>
      </template>

      <template v-else-if="!player.stats && !COLUMNS[column].fromProfile">
        <span class="stat-empty"> — </span>
      </template>

      <template v-else>
        <span
          class="stat-text"
          :style="{ color: cellColor(column) }"
        >
          {{ cellValue(column) }}
        </span>
      </template>
    </td>

    <td
      v-if="!showcase"
      class="remove-cell"
      style="overflow: visible !important"
    >
      <button
        class="btn remove-btn"
        @click="$emit('remove', player.name)"
      >
        <X :size="10" />
      </button>
    </td>
  </tr>
</template>

<style scoped>
.player-row {
  position: relative;
  overflow: visible !important;
  contain: layout style;

  background: linear-gradient(
    90deg,
    rgba(255, 0, 140, 0.01) 0%,
    rgba(168, 85, 247, 0.008) 50%,
    rgba(0, 229, 255, 0.006) 100%
  );

  transition:
    background 180ms ease,
    border-color 180ms ease,
    box-shadow 180ms ease,
    transform 180ms ease;
}

.player-row:hover {
  background: linear-gradient(
    90deg,
    rgba(255, 0, 140, 0.018) 0%,
    rgba(168, 85, 247, 0.014) 50%,
    rgba(0, 229, 255, 0.012) 100%
  );
  box-shadow: inset 0 0 0 1px rgba(var(--color-accent-rgb), 0.06);
}

html.low-end .player-row:hover {
  box-shadow: none;
}

.nicked-row {
  position: relative;
  overflow: hidden !important;
  contain: paint;
  isolation: isolate;
  background-image:
    linear-gradient(
      90deg,
      rgba(255, 0, 98, 0.16) 0%,
      rgba(255, 102, 0, 0.15) 18%,
      rgba(255, 217, 0, 0.12) 38%,
      rgba(174, 0, 255, 0.12) 62%,
      rgba(0, 225, 255, 0.1) 100%
    ),
    linear-gradient(
      115deg,
      transparent 20%,
      rgba(255, 255, 255, 0.13) 50%,
      transparent 80%
    );
  background-size:
    100% 100%,
    220% 100%;
  background-position:
    0 0,
    -20% 0;
  animation: nickShine 3.2s linear infinite;
  animation-play-state: var(--anim-play-state, running);
  box-shadow:
    inset 0 0 0 1px rgba(255, 255, 255, 0.05),
    inset 0 0 18px rgba(255, 0, 140, 0.08),
    0 0 18px rgba(255, 0, 170, 0.08);
}

.nicked-row:hover {
  transform: translateY(-1px);
  background: linear-gradient(
    90deg,
    rgba(255, 0, 98, 0.22) 0%,
    rgba(255, 102, 0, 0.2) 18%,
    rgba(255, 217, 0, 0.16) 38%,
    rgba(174, 0, 255, 0.16) 62%,
    rgba(0, 225, 255, 0.14) 100%
  );
}

@keyframes nickShine {
  0% {
    background-position:
      0 0,
      -20% 0;
  }
  100% {
    background-position:
      0 0,
      140% 0;
  }
}

.cell {
  padding: 6px 10px;
  vertical-align: middle;
  position: relative;
  z-index: 1;
  overflow: visible !important;
}

.name-cell {
  padding-left: 8px !important;
  width: 1%;
  text-align: left !important;
  z-index: 10;
}

.name-wrapper {
  display: flex;
  width: 100%;
  align-items: center;
  height: 17px;
  gap: 7px;
  min-width: 120px;
  overflow: visible !important;
  line-height: 0;
}

.avatar {
  width: 17px;
  height: 17px;
  border-radius: 4px;
  image-rendering: crisp-edges;
  flex-shrink: 0;
  display: block;
  align-self: center;
  line-height: 0;
  margin: 0;
}

.team-indicator {
  width: 3px;
  height: 18px;
  border-radius: 999px;
  flex-shrink: 0;
  align-self: center;
  box-shadow: 0 0 6px currentColor;
}

.rank-text,
.name-text,
.stat-text,
.stat-empty {
  display: inline-flex;
  align-items: center;
  align-self: center;
  height: 17px;
  font-family: Inter, system-ui, sans-serif;
  line-height: 17px;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

.rank-text {
  font-size: 12px;
  font-weight: 900;
  letter-spacing: 0;
  text-shadow: 0 0 10px rgba(255, 255, 255, 0.04);
}

.name-text {
  font-size: 12px;
  font-weight: 900;
  letter-spacing: 0;
  text-shadow: 0 0 10px rgba(255, 255, 255, 0.03);
}

.clan-tag-wrapper {
  display: inline-flex;
  align-items: center;
  align-self: center;
  height: 17px;
  font-family: Inter, system-ui, sans-serif;
  font-size: 12px;
  font-weight: 900;
  letter-spacing: 0;
  line-height: 1;
  flex-shrink: 0;
  -webkit-font-smoothing: antialiased;
}

.clan-bracket {
  color: rgba(255, 255, 255, 0.28);
  text-shadow: 0 0 6px rgba(255, 255, 255, 0.06);
}

.clan-text-inner {
  color: #ff73dc;
  text-shadow: 0 0 10px rgba(255, 100, 210, 0.22);
}

.clan-icon {
  background: linear-gradient(135deg, #ff7ad9 0%, #c084fc 35%, #60dfff 70%, #4ade80 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
  filter: drop-shadow(0 0 5px rgba(192, 132, 252, 0.55))
    drop-shadow(0 0 10px rgba(96, 223, 255, 0.3));
}

.stat-text {
  justify-content: center;
  font-size: 12px;
  font-weight: 900;
  letter-spacing: 0;
  font-variant-numeric: tabular-nums;
}

.stat-empty {
  justify-content: center;
  color: rgba(255, 255, 255, 0.22);
  font-size: 12px;
  font-weight: 800;
}

.arrow-separator {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  height: 17px;
  color: rgba(255, 255, 255, 0.28);
  margin: 0 3px;
  font-size: 9px;
  font-weight: 800;
}

.badge {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  align-self: center;
  min-width: max-content;
  height: 18px;
  padding: 0 8px;
  border-radius: 999px;
  font-family: Inter, system-ui, sans-serif;
  font-size: 10px;
  font-weight: 900;
  line-height: 1;
  letter-spacing: 0.03em;
  text-transform: uppercase;
  white-space: nowrap;
  -webkit-font-smoothing: antialiased;
  transform: translateZ(0);
}

.badge-staff {
  color: #3b0764;
  background: linear-gradient(120deg, #d9c8ff 0%, #b48cff 40%, #4fe3a5 100%);
  box-shadow:
    0 1px 3px rgba(59, 7, 100, 0.2),
    inset 0 1px 0 rgba(255, 255, 255, 0.6),
    inset 0 -6px 8px -6px rgba(59, 7, 100, 0.22);
}

.nick-icon-wrapper {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  align-self: center;
  width: 26px;
  height: 26px;
  flex-shrink: 0;
  position: relative;
  cursor: default;
}

.nick-badge {
  width: 22px;
  height: 22px;
  object-fit: contain;
  display: block;
  filter: contrast(1.4) saturate(2.2);
}

.lock-icon-wrapper {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  align-self: center;
  width: 20px;
  height: 20px;
  flex-shrink: 0;
  position: relative;
  cursor: default;
}

.lock-icon-wrapper svg {
  display: block;
}

.tooltip {
  position: absolute;
  left: calc(100% + 10px);
  top: 50%;
  transform: translateY(-50%) translateX(-4px);
  padding: 6px 11px;
  border-radius: 8px;
  font-family: 'Inter', system-ui, sans-serif;
  font-size: 11px;
  font-weight: 600;
  line-height: 1.4;
  color: rgba(255, 255, 255, 0.92);
  white-space: nowrap;
  pointer-events: none;
  z-index: 99999;
  opacity: 0;
  visibility: hidden;
  transition:
    opacity 0.16s ease,
    visibility 0.16s ease,
    transform 0.16s ease;
  transition-delay: 0.06s;
  will-change: transform, opacity;
}

.tooltip::after {
  content: '';
  position: absolute;
  top: 50%;
  right: 100%;
  transform: translateY(-50%);
  width: 0;
  height: 0;
  border-top: 5px solid transparent;
  border-bottom: 5px solid transparent;
  border-right: 6px solid;
}

.nick-icon-wrapper:hover .nick-tooltip,
.lock-icon-wrapper:hover .lock-tooltip {
  opacity: 1;
  visibility: visible;
  transform: translateY(-50%) translateX(0);
  transition-delay: 0.05s;
}

.nick-tooltip {
  background: linear-gradient(
    135deg,
    rgba(220, 30, 60, 0.97),
    rgba(255, 80, 30, 0.95),
    rgba(180, 10, 40, 0.97)
  );

  box-shadow: 0 6px 20px rgba(220, 40, 60, 0.5);
}

.nick-tooltip::after {
  border-right-color: rgba(220, 30, 60, 0.92);
}

.lock-tooltip {
  background: linear-gradient(
    135deg,
    rgba(160, 60, 210, 0.99),
    rgba(110, 70, 235, 0.99),
    rgba(40, 160, 245, 0.99)
  );

  box-shadow: 0 6px 20px rgba(130, 60, 220, 0.55);
}

.lock-tooltip::after {
  border-right-color: rgba(200, 80, 220, 0.92);
}

.remove-cell {
  width: 24px;
  padding: 0 6px;
  vertical-align: middle;
  position: relative;
  z-index: 1;
  overflow: visible !important;
}

.remove-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: 6px;
  opacity: 0;
  transition: opacity 120ms ease;
  color: var(--color-ink-3);
}

.group:hover .remove-btn {
  opacity: 1;
}
</style>
