<script setup lang="ts">
import type { TrafficMode, TrafficStatus } from '@renderer/types';
import Section from '@renderer/components/SettingsSection.vue';
import { Button } from '@renderer/components/ui/button';
import { onMounted, ref } from 'vue';

const status = ref<TrafficStatus>({ mode: 'middleware', hasApiKey: false });
const selectedMode = ref<TrafficMode>('middleware');
const apiKey = ref('');
const busy = ref(true);
const loaded = ref(false);
const error = ref('');
const message = ref('');

async function update(
  action: () => Promise<TrafficStatus>,
  success = '',
  keepInputOnError = false,
): Promise<void> {
  busy.value = true;
  error.value = '';
  message.value = '';
  try {
    status.value = await action();
    selectedMode.value = status.value.mode;
    apiKey.value = '';
    loaded.value = true;
    message.value = success;
  } catch (cause) {
    if (!keepInputOnError) selectedMode.value = status.value.mode;
    const detail = cause instanceof Error ? cause.message : String(cause);
    error.value = detail.replace(
      /^Error invoking remote method '[^']+': (?:Error: )?/,
      '',
    );
  } finally {
    busy.value = false;
  }
}

function selectMode(mode: TrafficMode): void {
  selectedMode.value = mode;
  error.value = '';
  message.value = '';
  apiKey.value = '';
  if (mode === 'direct' && !status.value.hasApiKey) return;
  void update(() => window.api.traffic.setMode(mode), 'Traffic preference saved.');
}

function saveApiKey(): void {
  if (!apiKey.value.trim() || busy.value) return;
  void update(
    () => window.api.traffic.saveApiKey(apiKey.value.trim()),
    'API key saved securely. PikaNetwork requests now use your key.',
    true,
  );
}

function removeApiKey(): void {
  void update(
    () => window.api.traffic.removeApiKey(),
    'API key removed. Traffic now uses the middleware proxy.',
  );
}

function loadSettings(): void {
  void update(() => window.api.traffic.get());
}

onMounted(loadSettings);
</script>

<template>
  <Section
    title="Traffic"
    description="Choose how Kyra connects to PikaNetwork to retrieve player profiles, BedWars stats, and clan details."
  >
    <div class="traffic-settings no-drag flex flex-col gap-3 py-3">
      <fieldset
        class="flex min-w-0 flex-col gap-2"
        :disabled="busy || !loaded"
      >
        <legend class="sr-only">PikaNetwork API routing</legend>
        <label
          class="traffic-option flex cursor-pointer items-start gap-3 rounded-lg p-3"
        >
          <input
            type="radio"
            name="traffic-mode"
            value="middleware"
            :checked="selectedMode === 'middleware'"
            @change="selectMode('middleware')"
          />
          <span class="flex min-w-0 flex-col gap-1">
            <span class="traffic-label">Route through middleware proxy</span>
            <span class="traffic-description"
              >Use Kyra's shared service to fetch PikaNetwork data. This is the default
              option and works without an API key.</span
            >
          </span>
        </label>
        <label
          class="traffic-option flex cursor-pointer items-start gap-3 rounded-lg p-3"
        >
          <input
            type="radio"
            name="traffic-mode"
            value="direct"
            :checked="selectedMode === 'direct'"
            @change="selectMode('direct')"
          />
          <span class="flex min-w-0 flex-col gap-1">
            <span class="traffic-label">Use my own API key</span>
            <span class="traffic-description"
              >Send requests directly to PikaNetwork using your own API key. Add a valid
              key below to enable direct access.</span
            >
          </span>
        </label>
      </fieldset>

      <form
        v-if="selectedMode === 'direct' || status.hasApiKey"
        class="flex flex-col gap-2"
        @submit.prevent="saveApiKey"
      >
        <label
          for="traffic-api-key"
          class="traffic-label"
          >{{ status.hasApiKey ? 'Replace API key' : 'API key' }}</label
        >
        <input
          id="traffic-api-key"
          v-model="apiKey"
          type="password"
          class="traffic-key w-full rounded-lg px-3 py-2"
          :placeholder="
            status.hasApiKey ? 'Enter a new API key' : 'Enter your PikaNetwork API key'
          "
          autocomplete="off"
          autocapitalize="off"
          :spellcheck="false"
          :disabled="busy"
        />
        <p class="traffic-description">
          {{
            status.hasApiKey
              ? 'An API key is saved securely. Saving a replacement enables direct requests.'
              : 'Save a key to enable direct requests. Until then, traffic uses the middleware proxy.'
          }}
          Your key is encrypted before it is saved on this device.
        </p>
        <div class="flex flex-wrap gap-2">
          <Button
            type="submit"
            variant="control"
            size="sm"
            :disabled="busy || !apiKey.trim()"
            >{{ status.hasApiKey ? 'Replace API key' : 'Save API key' }}</Button
          >
          <Button
            v-if="status.hasApiKey"
            type="button"
            variant="control"
            size="sm"
            :disabled="busy"
            @click="removeApiKey"
            >Remove API key</Button
          >
        </div>
      </form>

      <p
        v-if="error"
        role="alert"
        class="text-sm text-red-400"
      >
        {{ error }}
      </p>
      <Button
        v-if="!loaded && !busy"
        variant="control"
        size="sm"
        @click="loadSettings"
        >Retry</Button
      >
      <p
        v-if="message"
        role="status"
        class="traffic-description"
      >
        {{ message }}
      </p>
    </div>
  </Section>
</template>

<style scoped>
.traffic-option,
.traffic-key {
  border: 1px solid var(--color-border);
  background: rgba(255, 255, 255, 0.025);
}

.traffic-option:has(input:checked) {
  border-color: var(--color-accent);
}

.traffic-option input {
  margin-top: 3px;
  accent-color: var(--color-accent);
}

.traffic-label,
.traffic-key {
  color: var(--color-ink-2);
  font-size: 0.82rem;
}

.traffic-description {
  color: var(--color-ink-3);
  font-size: 0.74rem;
  line-height: 1.6;
}
</style>
