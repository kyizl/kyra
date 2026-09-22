<script setup lang="ts">
import type { LogFilePreset } from '@renderer/types';
import { Button } from '@renderer/components/ui/button';
import { Card, CardContent } from '@renderer/components/ui/card';
import { useStaggerReveal } from '@renderer/composables/useStaggerReveal';
import { useConfigStore } from '@renderer/store/config';
import { usePlayersStore } from '@renderer/store/players';
import { CheckCircle2, FolderOpen, Loader, XCircle } from 'lucide-vue-next';
import { onMounted, ref } from 'vue';

const config = useConfigStore();
const players = usePlayersStore();

const validState = ref<boolean | null>(null);
const root = ref<HTMLElement | null>(null);

useStaggerReveal(root, { selector: '.setup-reveal', stagger: 0.05, y: 10 });

const presets: { label: string; value: LogFilePreset }[] = [
  { label: 'Standard', value: 'STANDARD' },
  { label: 'Lunar', value: 'LUNAR_CLIENT' },
  { label: 'TLauncher', value: 'TLAUNCHER' },
  { label: 'Silent', value: 'SILENT_CLIENT' },
  { label: 'Feather', value: 'FEATHER_CLIENT' },
  { label: 'SK', value: 'SK_CLIENT' },
  { label: 'CM', value: 'CM_CLIENT' },
  { label: 'Salwyrr', value: 'SALWYRR' },
  { label: 'Badlion', value: 'BADLION_CLIENT' },
  { label: 'PvPLounge', value: 'PVPLOUNGE' },
  { label: 'Custom', value: 'CUSTOM' },
];

async function checkPath(): Promise<void> {
  validState.value = null;
  const isValid = await window.api.log.checkPath(config.logFilePath);
  validState.value = isValid;
  players.logPathValid = isValid;
  if (isValid) window.api.log.setPath(config.logFilePath);
  else window.api.log.setPath(null);
}

async function applyPreset(preset: LogFilePreset): Promise<void> {
  await config.setLogFilePathFromPreset(preset);
  await checkPath();
}

async function browse(): Promise<void> {
  const result = await window.api.log.openDialog();
  if (!result.canceled && result.filePaths[0]) {
    config.logFilePath = result.filePaths[0];
    config.logFilePathPreset = 'CUSTOM';
    await checkPath();
  }
}

onMounted(() => checkPath());
</script>

<template>
  <div
    ref="root"
    class="themed-scroll flex h-full flex-col gap-3.5 overflow-y-auto px-3.5 py-3.5"
  >
    <div class="setup-reveal">
      <h2
        class="gradient-ink font-semibold"
        style="font-size: 0.92rem; font-family: var(--font-display)"
      >
        Setup
      </h2>
      <p
        class="mt-0.5"
        style="font-size: 0.78rem; color: var(--color-ink-3)"
      >
        Point the overlay at your Minecraft log file to enable auto-detection.
      </p>
    </div>

    <Card class="setup-reveal">
      <CardContent>
        <div class="flex items-center justify-between">
          <div
            class="font-semibold"
            style="font-size: 0.85rem; color: var(--color-ink-1)"
          >
            Log File
          </div>
          <div
            class="flex shrink-0 items-center gap-1.5"
            style="font-size: 0.78rem"
          >
            <template v-if="validState === true">
              <CheckCircle2
                :size="13"
                style="color: var(--color-good)"
              />
              <span style="color: var(--color-good); font-weight: 500">Valid</span>
            </template>
            <template v-else-if="validState === false">
              <XCircle
                :size="13"
                style="color: var(--color-bad)"
              />
              <span style="color: var(--color-bad); font-weight: 500">Not found</span>
            </template>
            <template v-else>
              <Loader
                :size="13"
                class="animate-spin"
                style="color: var(--color-ink-3)"
              />
              <span style="color: var(--color-ink-3)">Checking…</span>
            </template>
          </div>
        </div>

        <div class="flex flex-col gap-1.5">
          <span class="kyra-eyebrow">Client</span>
          <div class="no-drag grid grid-cols-3 gap-1.5">
            <button
              v-for="preset in presets"
              :key="preset.value"
              v-press
              type="button"
              class="preset-chip rounded-md px-2 py-1.5 font-medium"
              :class="{
                'preset-chip--active': config.logFilePathPreset === preset.value,
              }"
              style="font-size: 0.76rem"
              @click="applyPreset(preset.value)"
            >
              {{ preset.label }}
            </button>
          </div>
        </div>

        <div class="flex flex-col gap-1.5">
          <span class="kyra-eyebrow">Path</span>
          <div class="no-drag flex gap-2">
            <input
              v-model="config.logFilePath"
              class="input-field flex-1 font-mono"
              style="font-size: 0.72rem"
              placeholder="/path/to/logs/latest.log"
              @blur="checkPath"
            />
            <Button
              variant="outline"
              class="h-8 shrink-0 gap-1.5"
              @click="browse"
            >
              <FolderOpen :size="12" />
              Browse
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>

    <div class="setup-reveal no-drag mt-auto flex justify-end pt-1">
      <Button
        as="router-link"
        to="/"
        variant="control"
        size="sm"
        class="gap-1.5 px-4"
      >
        <CheckCircle2 :size="11" />
        Done
      </Button>
    </div>
  </div>
</template>

<style scoped>
.preset-chip {
  background: var(--color-surface-1);
  border: 1px solid var(--color-border);
  color: var(--color-ink-3);
  transition:
    background 160ms var(--ease-out),
    border-color 160ms var(--ease-out),
    color 160ms var(--ease-out);
}

.preset-chip:hover {
  border-color: var(--color-border-hover);
  color: var(--color-ink-2);
}

.preset-chip--active {
  background: var(--color-accent-dim);
  border-color: rgba(var(--color-accent-rgb), 0.38);
  color: var(--color-accent-light);
  box-shadow: 0 0 14px rgba(var(--color-accent-rgb), 0.22);
}

html.low-end .preset-chip--active {
  box-shadow: none;
}

html.low-end .preset-chip {
  transition: none !important;
}
</style>
