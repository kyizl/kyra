<script setup lang="ts">
import { Button } from '@renderer/components/ui/button';
import {
  clearPerfData,
  perfState,
  snapshotPerfData,
  topSlowest,
} from '@renderer/lib/perf-bus';
import { Download, FolderOpen, Radio, Trash2, X } from 'lucide-vue-next';
import { computed, onMounted, onUnmounted, ref } from 'vue';

const collapsed = ref(false);
const dumping = ref(false);
const tracing = ref(false);
const lastDumpPath = ref<string | null>(null);
const refreshTick = ref(0);

let refreshTimer: ReturnType<typeof setInterval> | null = null;

onMounted(() => {
  refreshTimer = setInterval(() => {
    refreshTick.value += 1;
  }, 1000);
});

onUnmounted(() => {
  if (refreshTimer !== null) clearInterval(refreshTimer);
});

const fpsTone = computed(() => tone(perfState.fps, 50, 30, false));
const jankTone = computed(() => tone(perfState.jankPct, 5, 15, true));
const longTaskTone = computed(() => tone(perfState.longTaskCount, 0, 5, true));
const heapTone = computed(() =>
  tone(
    perfState.heapUsedMB,
    perfState.heapLimitMB * 0.5,
    perfState.heapLimitMB * 0.8,
    true,
  ),
);

function tone(value: number, good: number, bad: number, higherIsWorse: boolean): string {
  const isBad = higherIsWorse ? value >= bad : value <= bad;
  const isGood = higherIsWorse ? value <= good : value >= good;
  if (isBad) return 'var(--color-bad)';
  if (isGood) return 'var(--color-good)';
  return 'var(--color-warn)';
}

const topStoreActions = computed(() => {
  void refreshTick.value;
  return topSlowest('store', 5);
});

const topIpcChannels = computed(() => {
  void refreshTick.value;
  return topSlowest('ipc', 5);
});

const topNetworkCalls = computed(() => {
  void refreshTick.value;
  return topSlowest('network', 5);
});

function fmt(value: number, digits = 1): string {
  return value.toFixed(digits);
}

async function handleDump(): Promise<void> {
  dumping.value = true;
  try {
    const snapshot = snapshotPerfData();
    lastDumpPath.value = await window.api.perf.dump(snapshot);
  } finally {
    dumping.value = false;
  }
}

async function handleClear(): Promise<void> {
  clearPerfData();
  lastDumpPath.value = null;
}

async function handleTraceToggle(): Promise<void> {
  if (tracing.value) {
    await window.api.perf.stopTrace();
    tracing.value = false;
  } else {
    tracing.value = await window.api.perf.startTrace();
  }
}

async function handleOpenFolder(): Promise<void> {
  await window.api.perf.openLogDir();
}
</script>

<template>
  <div
    class="perf-overlay"
    :class="{ 'perf-overlay--collapsed': collapsed }"
  >
    <div class="perf-overlay__header">
      <div class="perf-overlay__title">
        <Radio
          :size="12"
          :style="{ color: fpsTone }"
        />
        <span>{{ fmt(perfState.fps, 0) }} FPS</span>
        <span class="perf-overlay__sub">{{ fmt(perfState.frameMs) }}ms</span>
      </div>
      <button
        class="perf-overlay__collapse"
        type="button"
        @click="collapsed = !collapsed"
      >
        {{ collapsed ? '+' : '\u2212' }}
      </button>
    </div>

    <div
      v-if="!collapsed"
      class="perf-overlay__body"
    >
      <div class="perf-overlay__grid">
        <div class="perf-metric">
          <span class="perf-metric__label">Jank</span>
          <span
            class="perf-metric__value"
            :style="{ color: jankTone }"
            >{{ fmt(perfState.jankPct) }}%</span
          >
        </div>
        <div class="perf-metric">
          <span class="perf-metric__label">Long tasks</span>
          <span
            class="perf-metric__value"
            :style="{ color: longTaskTone }"
            >{{ perfState.longTaskCount }}</span
          >
        </div>
        <div class="perf-metric">
          <span class="perf-metric__label">Heap</span>
          <span
            class="perf-metric__value"
            :style="{ color: heapTone }"
            >{{ fmt(perfState.heapUsedMB, 0) }}/{{
              fmt(perfState.heapLimitMB, 0)
            }}MB</span
          >
        </div>
        <div class="perf-metric">
          <span class="perf-metric__label">Uptime</span>
          <span class="perf-metric__value">{{ fmt(perfState.uptimeMs / 1000, 0) }}s</span>
        </div>
        <div class="perf-metric">
          <span class="perf-metric__label">IPC avg/max</span>
          <span class="perf-metric__value">
            {{ fmt(perfState.ipcAvgMs) }}/{{ fmt(perfState.ipcMaxMs) }}ms
          </span>
        </div>
        <div class="perf-metric">
          <span class="perf-metric__label">Store avg/max</span>
          <span class="perf-metric__value">
            {{ fmt(perfState.storeAvgMs) }}/{{ fmt(perfState.storeMaxMs) }}ms
          </span>
        </div>
        <div class="perf-metric">
          <span class="perf-metric__label">Network avg/max</span>
          <span class="perf-metric__value">
            {{ fmt(perfState.networkAvgMs) }}/{{ fmt(perfState.networkMaxMs) }}ms
          </span>
        </div>
        <div class="perf-metric">
          <span class="perf-metric__label">Render avg</span>
          <span class="perf-metric__value">{{ fmt(perfState.renderAvgMs) }}ms</span>
        </div>
      </div>

      <div
        v-if="topStoreActions.length > 0"
        class="perf-overlay__section"
      >
        <span class="perf-overlay__section-title">Slowest store actions</span>
        <div
          v-for="event in topStoreActions"
          :key="`${event.label}-${event.t}`"
          class="perf-overlay__row"
        >
          <span class="perf-overlay__row-label">{{ event.label }}</span>
          <span class="perf-overlay__row-value">{{ fmt(event.dur) }}ms</span>
        </div>
      </div>

      <div
        v-if="topIpcChannels.length > 0"
        class="perf-overlay__section"
      >
        <span class="perf-overlay__section-title">Slowest IPC channels</span>
        <div
          v-for="event in topIpcChannels"
          :key="`${event.label}-${event.t}`"
          class="perf-overlay__row"
        >
          <span class="perf-overlay__row-label">{{ event.label }}</span>
          <span class="perf-overlay__row-value">{{ fmt(event.dur) }}ms</span>
        </div>
      </div>

      <div
        v-if="topNetworkCalls.length > 0"
        class="perf-overlay__section"
      >
        <span class="perf-overlay__section-title">Slowest network calls</span>
        <div
          v-for="event in topNetworkCalls"
          :key="`${event.label}-${event.t}`"
          class="perf-overlay__row"
        >
          <span class="perf-overlay__row-label">{{ event.label }}</span>
          <span class="perf-overlay__row-value">{{ fmt(event.dur) }}ms</span>
        </div>
      </div>

      <div class="perf-overlay__actions">
        <Button
          variant="control"
          size="sm"
          class="gap-1"
          :disabled="dumping"
          @click="handleDump"
        >
          <Download :size="12" />
          Dump
        </Button>
        <Button
          variant="control"
          size="sm"
          class="gap-1"
          @click="handleTraceToggle"
        >
          <X
            v-if="tracing"
            :size="12"
          />
          {{ tracing ? 'Stop trace' : 'Deep trace' }}
        </Button>
        <Button
          variant="control"
          size="sm"
          class="gap-1"
          @click="handleOpenFolder"
        >
          <FolderOpen :size="12" />
        </Button>
        <Button
          variant="control"
          size="sm"
          class="gap-1"
          @click="handleClear"
        >
          <Trash2 :size="12" />
        </Button>
      </div>

      <div
        v-if="lastDumpPath"
        class="perf-overlay__dump-path"
      >
        Saved to {{ lastDumpPath }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.perf-overlay {
  position: fixed;
  right: 10px;
  bottom: 10px;
  z-index: 12000;
  width: 240px;
  max-height: 70vh;
  overflow-y: auto;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border);
  background: rgba(10, 8, 6, 0.88);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  color: var(--color-ink-1);
  font-size: 11px;
  line-height: 1.4;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
}

.perf-overlay--collapsed {
  width: auto;
}

.perf-overlay__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 6px 8px;
  border-bottom: 1px solid var(--color-border);
}

.perf-overlay--collapsed .perf-overlay__header {
  border-bottom: none;
}

.perf-overlay__title {
  display: flex;
  align-items: center;
  gap: 5px;
  font-weight: 600;
  white-space: nowrap;
}

.perf-overlay__sub {
  color: var(--color-ink-2);
  font-weight: 400;
}

.perf-overlay__collapse {
  border: none;
  background: transparent;
  color: var(--color-ink-2);
  cursor: pointer;
  font-size: 13px;
  line-height: 1;
  padding: 2px 4px;
}

.perf-overlay__collapse:hover {
  color: var(--color-ink-1);
}

.perf-overlay__body {
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.perf-overlay__grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 6px;
}

.perf-metric {
  display: flex;
  flex-direction: column;
  gap: 1px;
  padding: 4px 6px;
  border-radius: var(--radius-xs);
  background: var(--color-surface-1);
}

.perf-metric__label {
  color: var(--color-ink-2);
  font-size: 9.5px;
  text-transform: uppercase;
  letter-spacing: 0.02em;
}

.perf-metric__value {
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.perf-overlay__section {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.perf-overlay__section-title {
  color: var(--color-ink-2);
  font-size: 9.5px;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  margin-bottom: 2px;
}

.perf-overlay__row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
}

.perf-overlay__row-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.perf-overlay__row-value {
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
  color: var(--color-accent-light);
}

.perf-overlay__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.perf-overlay__dump-path {
  color: var(--color-ink-2);
  font-size: 9.5px;
  word-break: break-all;
}
</style>
