<script setup lang="ts">
import type { Network, ProxyStatusAll } from '@renderer/types';
import Section from '@renderer/components/SettingsSection.vue';
import ShortcutInput from '@renderer/components/ShortcutInput.vue';
import SideTabNav from '@renderer/components/SideTabNav.vue';
import ToggleSetting from '@renderer/components/ToggleSetting.vue';
import { Button } from '@renderer/components/ui/button';
import { Switch } from '@renderer/components/ui/switch';
import { useConfigStore } from '@renderer/store/config';
import { usePlayersStore } from '@renderer/store/players';
import { Column, COLUMNS, NETWORKS } from '@renderer/types';
import {
  ArrowDown,
  ArrowUp,
  ArrowUpDown,
  Columns3,
  Download,
  Keyboard,
  Palette,
  RefreshCw,
  Sliders,
  X,
} from 'lucide-vue-next';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useRoute } from 'vue-router';

const config = useConfigStore();
const players = usePlayersStore();
const route = useRoute();
const activeTab = ref(
  typeof route.query.tab === 'string' ? route.query.tab : 'appearance',
);

type UpdateStatus =
  | 'idle'
  | 'dev'
  | 'checking'
  | 'up-to-date'
  | 'available'
  | 'downloading'
  | 'downloaded'
  | 'error';
const updateStatus = ref<UpdateStatus>('idle');
const updateVersion = ref('');
const updatePercent = ref(0);
const updateError = ref('');

const updateStatusText = computed(() => {
  switch (updateStatus.value) {
    case 'dev':
      return 'Updates disabled in dev mode';
    case 'checking':
      return 'Checking for updates…';
    case 'up-to-date':
      return 'You are on the latest version';
    case 'available':
      return `Update available: v${updateVersion.value}`;
    case 'downloading':
      return `Downloading… ${updatePercent.value}%`;
    case 'downloaded':
      return `v${updateVersion.value} ready to install`;
    case 'error':
      return `Error: ${updateError.value}`;
    default:
      return 'Check for updates';
  }
});

function checkUpdate(): void {
  window.api.updater.check();
}
function installUpdate(): void {
  window.api.updater.install();
}

let unsubscribeUpdater: (() => void) | null = null;
let proxyPollTimer: ReturnType<typeof setInterval> | null = null;

onMounted(() => {
  unsubscribeUpdater = window.api.updater.onStatus((payload) => {
    updateStatus.value = payload.status as UpdateStatus;
    if (payload.version) updateVersion.value = payload.version;
    if (payload.percent !== undefined) updatePercent.value = payload.percent;
    if (payload.error) updateError.value = payload.error;
  });
  void refreshProxyStatus();
  proxyPollTimer = setInterval(() => void refreshProxyStatus(), 3000);
});
onUnmounted(() => {
  unsubscribeUpdater?.();
  if (proxyPollTimer) clearInterval(proxyPollTimer);
});

const tabs = [
  { id: 'appearance', label: 'Appearance', icon: Palette },
  { id: 'columns', label: 'Columns', icon: Columns3 },
  { id: 'sorting', label: 'Sorting', icon: ArrowUpDown },
  { id: 'shortcuts', label: 'Shortcuts', icon: Keyboard },
  { id: 'advanced', label: 'Advanced', icon: Sliders },
];

const proxyStatus = ref<ProxyStatusAll | null>(null);
const pikaPortInput = ref<string>(String(config.pikaProxyPort));
const jartexPortInput = ref<string>(String(config.jartexProxyPort));
const proxyPortError = ref<string>('');

async function refreshProxyStatus(): Promise<void> {
  proxyStatus.value = await window.api.proxy.getStatus();
}

async function applyProxyPort(network: 'pikanetwork' | 'jartexnetwork'): Promise<void> {
  const raw = network === 'pikanetwork' ? pikaPortInput.value : jartexPortInput.value;
  const port = parseInt(raw, 10);
  if (isNaN(port) || port < 1024 || port > 65535) {
    proxyPortError.value = 'Port must be between 1024 and 65535';
    return;
  }
  proxyPortError.value = '';
  if (network === 'pikanetwork') config.pikaProxyPort = port;
  else config.jartexProxyPort = port;
  await window.api.proxy.setPort(network, port);
  await refreshProxyStatus();
}

async function applyBindHost(host: '0.0.0.0' | '127.0.0.1'): Promise<void> {
  config.proxyBindHost = host;
  config.proxyBannerDismissed = false;
  await window.api.proxy.setBindHost(host);
  await refreshProxyStatus();
}

const columns = Object.values(Column);
const sortableColumns = computed(() =>
  columns.filter((column) => COLUMNS[column]?.sortable === true),
);
const inactiveColumns = computed(() =>
  config.activeColumns
    ? columns.filter((column) => !config.activeColumns.includes(column))
    : [],
);

function toggleColumn(column: Column): void {
  const index = config.activeColumns.indexOf(column);
  if (index === -1) config.activeColumns.push(column);
  else if (config.activeColumns.length > 1) config.activeColumns.splice(index, 1);
}

function moveColumn(index: number, direction: -1 | 1): void {
  const columns = [...config.activeColumns];
  const swapIndex = index + direction;
  if (swapIndex < 0 || swapIndex >= columns.length) return;
  [columns[index], columns[swapIndex]] = [columns[swapIndex], columns[index]];
  config.activeColumns = columns;
}

async function onShortcutChange(
  shortcutKey: 'shortcutMinimize' | 'shortcutClearPlayers',
  value: string,
): Promise<void> {
  config[shortcutKey] = value;
  await window.api.shortcuts.register([
    config.shortcutMinimize,
    config.shortcutClearPlayers,
  ]);
}

function onNetworkChange(network: Network): void {
  if (config.network === network) return;
  config.network = network;
  window.api.rpc.setNetwork(network);
  players.clear();
}

const networkColors: Record<Network, string> = {
  pikanetwork: '#ffea00',
  jartexnetwork: '#06b6d4',
};

const proxies = computed(() => [
  {
    label: 'PikaNetwork',
    network: 'pikanetwork' as const,
    host: 'pika.host',
    color: '#ffea00',
    status: proxyStatus.value?.pika,
    portInput: pikaPortInput,
    port: config.pikaProxyPort,
  },
  {
    label: 'JartexNetwork',
    network: 'jartexnetwork' as const,
    host: 'jartex.fun',
    color: '#06b6d4',
    status: proxyStatus.value?.jartex,
    portInput: jartexPortInput,
    port: config.jartexProxyPort,
  },
]);
</script>

<template>
  <div
    class="settings-outer flex h-full overflow-hidden"
    style="pointer-events: all"
  >
    <aside
      class="settings-sidebar kyra-sidebar themed-scroll flex shrink-0 flex-col overflow-y-auto border-r py-2"
    >
      <div class="px-3">
        <SideTabNav
          :tabs="tabs"
          :model-value="activeTab"
          @update:model-value="activeTab = $event"
        />
      </div>
    </aside>

    <div
      class="settings-content themed-scroll flex flex-1 flex-col gap-4 overflow-y-auto p-3"
    >
      <div
        v-if="activeTab === 'appearance'"
        class="animate-fade-in flex flex-col gap-4"
      >
        <Section title="Appearance">
          <ToggleSetting
            label="Rounded corners"
            :value="config.roundedCorners"
            @update="config.roundedCorners = $event"
          />
          <ToggleSetting
            label="Always on top"
            :value="config.alwaysOnTop"
            @update="config.alwaysOnTop = $event"
          />
          <ToggleSetting
            label="Text shadow"
            :value="config.textShadow"
            @update="config.textShadow = $event"
          />
          <ToggleSetting
            label="Integrated mode"
            :value="config.integratedMode"
            @update="config.integratedMode = $event"
          />
          <div class="settings-row flex items-center justify-between">
            <span style="font-size: 0.82rem; font-weight: 500; color: var(--color-ink-2)"
              >Column labels</span
            >
            <div class="no-drag flex gap-1">
              <Button
                v-for="opt in ['FULL', 'SHORT', 'HIDDEN'] as const"
                :key="opt"
                variant="control"
                size="sm"
                class="min-w-[56px]"
                :class="{ 'btn-control-v2--active': config.columnLabels === opt }"
                @click="config.columnLabels = opt"
              >
                {{ opt.charAt(0) + opt.slice(1).toLowerCase() }}
              </Button>
            </div>
          </div>
        </Section>
      </div>

      <div
        v-if="activeTab === 'columns'"
        class="animate-fade-in flex flex-col gap-4"
      >
        <Section title="Columns">
          <div
            v-if="config.activeColumns?.length"
            class="flex flex-col gap-4 py-3"
          >
            <div v-if="config.activeColumns.length">
              <p class="group-label mb-1.5">Active</p>
              <div class="flex flex-col gap-1.5">
                <div
                  v-for="(column, index) in config.activeColumns"
                  :key="column"
                  class="active-col-row flex items-center justify-between"
                >
                  <span
                    style="
                      font-size: 0.8rem;
                      font-weight: 500;
                      color: var(--color-accent-light);
                    "
                    >{{ COLUMNS[column].label }}</span
                  >
                  <div class="col-reorder-group no-drag">
                    <button
                      type="button"
                      class="col-reorder-btn"
                      :disabled="index === 0"
                      @click="moveColumn(index, -1)"
                    >
                      <ArrowUp :size="10" />
                    </button>
                    <button
                      type="button"
                      class="col-reorder-btn"
                      :disabled="index === config.activeColumns.length - 1"
                      @click="moveColumn(index, 1)"
                    >
                      <ArrowDown :size="10" />
                    </button>
                    <button
                      type="button"
                      class="col-remove-btn"
                      :disabled="config.activeColumns.length <= 1"
                      @click="toggleColumn(column)"
                    >
                      <X :size="10" />
                    </button>
                  </div>
                </div>
              </div>
            </div>

            <div v-if="inactiveColumns.length">
              <p class="group-label mb-1.5">Available</p>
              <div class="flex flex-col gap-1.5">
                <div
                  v-for="column in inactiveColumns"
                  :key="column"
                  class="inactive-col-row flex cursor-pointer items-center justify-between"
                  @click="toggleColumn(column)"
                >
                  <span
                    style="font-size: 0.8rem; font-weight: 500; color: var(--color-ink-3)"
                    >{{ COLUMNS[column].label }}</span
                  >
                  <span class="add-label">+ Add</span>
                </div>
              </div>
            </div>
          </div>
          <div
            v-else
            class="py-4 text-center"
            style="font-size: 0.78rem; color: var(--color-ink-3)"
          >
            No columns configured.
          </div>
        </Section>
      </div>

      <div
        v-if="activeTab === 'sorting'"
        class="animate-fade-in flex flex-col gap-4"
      >
        <Section title="Sort By">
          <div
            v-if="sortableColumns.length"
            class="no-drag flex flex-col gap-1.5 py-3"
          >
            <button
              v-for="column in sortableColumns"
              :key="column"
              type="button"
              class="sort-list-row"
              :class="{ 'sort-list-row--active': config.sortBy === column }"
              @click="config.sortBy = column"
            >
              <span class="sort-radio">
                <span
                  v-if="config.sortBy === column"
                  class="sort-radio-dot"
                />
              </span>
              {{ COLUMNS[column].label }}
            </button>
          </div>
          <ToggleSetting
            label="Sort ascending"
            :value="config.sortAscending"
            @update="config.sortAscending = $event"
          />
        </Section>
      </div>

      <div
        v-if="activeTab === 'shortcuts'"
        class="animate-fade-in flex flex-col gap-4"
      >
        <Section
          title="Keyboard Shortcuts"
          description="Click a field, then press your desired key combination."
        >
          <ShortcutInput
            label="Minimize / Restore"
            :value="config.shortcutMinimize"
            @update="onShortcutChange('shortcutMinimize', $event)"
          />
          <ShortcutInput
            label="Clear Players"
            :value="config.shortcutClearPlayers"
            @update="onShortcutChange('shortcutClearPlayers', $event)"
          />
        </Section>
      </div>

      <div
        v-if="activeTab === 'advanced'"
        class="animate-fade-in flex flex-col gap-4"
      >
        <Section title="Network">
          <div class="flex flex-col gap-3 py-3">
            <div class="setting-panel flex items-center justify-between gap-3">
              <div>
                <div class="panel-label">Auto-detect via proxy</div>
                <div class="panel-desc mt-0.5">
                  Automatically switches the active network when your client connects
                  through the proxy
                </div>
              </div>
              <Switch
                class="no-drag shrink-0"
                :model-value="config.autoDetectNetwork"
                @update:model-value="config.autoDetectNetwork = $event"
              />
            </div>

            <div
              :style="{
                opacity: config.autoDetectNetwork ? 0.4 : 1,
                pointerEvents: config.autoDetectNetwork ? 'none' : 'auto',
                transition: 'opacity 0.2s',
              }"
            >
              <p class="panel-desc mb-2 text-center">Manual Selection</p>
              <div class="flex justify-center">
                <div class="network-select no-drag">
                  <button
                    v-for="network in NETWORKS"
                    :key="network.value"
                    type="button"
                    class="network-seg"
                    :class="{
                      'network-seg--active': config.network === network.value,
                    }"
                    :style="
                      config.network === network.value
                        ? {
                            background: `${networkColors[network.value]}18`,
                            color: networkColors[network.value],
                            boxShadow: `0 0 16px ${networkColors[network.value]}33`,
                          }
                        : {}
                    "
                    @click="onNetworkChange(network.value)"
                  >
                    {{ network.label }}
                  </button>
                </div>
              </div>
            </div>
          </div>
        </Section>

        <Section title="Proxy">
          <div class="flex flex-col gap-2.5 py-3">
            <p
              class="panel-desc"
              style="line-height: 1.6"
            >
              Route your Minecraft client through the proxy to enable team detection,
              player tracking, and automatic network switching.
            </p>

            <div class="setting-panel flex items-center justify-between gap-3">
              <div>
                <div class="panel-label">Allow LAN access</div>
                <div class="panel-desc mt-0.5">
                  Binds to
                  <span
                    class="font-mono"
                    style="color: var(--color-ink-2); font-size: 0.69rem"
                    >0.0.0.0</span
                  >
                  - other devices on your local network can connect
                </div>
              </div>
              <Switch
                class="no-drag shrink-0"
                :model-value="config.proxyBindHost === '0.0.0.0'"
                @update:model-value="applyBindHost($event ? '0.0.0.0' : '127.0.0.1')"
              />
            </div>

            <div
              v-if="config.proxyBindHost === '0.0.0.0'"
              class="rounded-xl px-3 py-2"
              style="
                background: rgba(251, 191, 36, 0.05);
                border: 1px solid rgba(251, 191, 36, 0.18);
              "
            >
              <span style="font-size: 0.71rem; color: #fbbf24; line-height: 1.5">
                LAN mode active. Any device on your local network can route traffic
                through this proxy.
              </span>
            </div>

            <div class="flex flex-col gap-2">
              <div
                v-for="entry in proxies"
                :key="entry.network"
                class="proxy-card p-3"
              >
                <div class="mb-2.5 flex items-center justify-between">
                  <div class="flex items-center gap-2">
                    <span
                      class="proxy-status-dot"
                      :class="{ 'animate-status-blink': entry.status?.running }"
                      :style="{
                        background: entry.status?.running
                          ? '#34d399'
                          : entry.status?.error
                            ? '#f87171'
                            : 'rgba(255,255,255,0.15)',
                        boxShadow: entry.status?.running
                          ? '0 0 6px rgba(52,211,153,0.6)'
                          : 'none',
                      }"
                    />
                    <span
                      class="font-semibold"
                      :style="{ color: entry.color, fontSize: '0.78rem' }"
                      >{{ entry.label }}</span
                    >
                    <span
                      v-if="entry.status?.running"
                      class="status-pill status-pill--active"
                      >ACTIVE</span
                    >
                    <span
                      v-else-if="entry.status?.error"
                      class="status-pill status-pill--error"
                      >ERROR</span
                    >
                  </div>
                  <span style="font-size: 0.7rem; color: var(--color-ink-3)"
                    >{{ entry.host }}:25565</span
                  >
                </div>

                <div
                  class="proxy-dest-row flex items-center justify-between gap-3 rounded-lg px-3 py-2.5"
                >
                  <div class="min-w-0">
                    <span class="field-label">Point your client to</span>
                    <div class="mt-1 flex items-center gap-2">
                      <span
                        class="font-mono font-semibold"
                        style="font-size: 0.82rem"
                        :style="{ color: entry.color }"
                      >
                        {{
                          config.proxyBindHost === '0.0.0.0'
                            ? '&lt;your-ip&gt;'
                            : 'localhost'
                        }}:{{ entry.port }}
                      </span>
                      <span
                        v-if="entry.status?.clientCount"
                        class="proxy-client-badge"
                      >
                        {{ entry.status.clientCount }} connected
                      </span>
                    </div>
                  </div>

                  <div class="flex shrink-0 flex-col items-end gap-1.5">
                    <span class="field-label">Local port</span>
                    <div class="no-drag flex items-center gap-1.5">
                      <input
                        class="proxy-port-input font-mono"
                        type="number"
                        min="1024"
                        max="65535"
                        :value="
                          entry.network === 'pikanetwork'
                            ? pikaPortInput
                            : jartexPortInput
                        "
                        @input="
                          entry.network === 'pikanetwork'
                            ? (pikaPortInput = ($event.target as HTMLInputElement).value)
                            : (jartexPortInput = ($event.target as HTMLInputElement)
                                .value)
                        "
                        @keydown.enter="applyProxyPort(entry.network)"
                      />
                      <Button
                        variant="control"
                        size="sm"
                        @click="applyProxyPort(entry.network)"
                      >
                        Apply
                      </Button>
                    </div>
                  </div>
                </div>

                <div
                  v-if="entry.status?.error"
                  class="mt-2 rounded-lg px-2.5 py-1.5"
                  style="
                    background: rgba(248, 113, 113, 0.06);
                    border: 1px solid rgba(248, 113, 113, 0.15);
                  "
                >
                  <span style="font-size: 0.68rem; color: #f87171">{{
                    entry.status.error
                  }}</span>
                </div>
              </div>
            </div>

            <div
              v-if="proxyPortError"
              class="rounded-lg px-2.5 py-1.5"
              style="
                background: rgba(248, 113, 113, 0.06);
                border: 1px solid rgba(248, 113, 113, 0.15);
              "
            >
              <span style="font-size: 0.68rem; color: #f87171">{{ proxyPortError }}</span>
            </div>
          </div>
        </Section>

        <Section title="Auto-detection">
          <ToggleSetting
            label="Auto-add on join"
            :value="config.autoAddPlayers"
            @update="config.autoAddPlayers = $event"
          />
          <ToggleSetting
            label="Clear list on /who"
            :value="config.autoRemoveAllOnWho"
            @update="config.autoRemoveAllOnWho = $event"
          />
          <ToggleSetting
            label="Remove on final kill"
            :value="config.autoRemoveFinalDeath"
            @update="config.autoRemoveFinalDeath = $event"
          />
          <ToggleSetting
            label="Remove on quit"
            :value="config.autoRemoveOnQuit"
            @update="config.autoRemoveOnQuit = $event"
          />
          <ToggleSetting
            label="Missing players warning"
            :value="config.missingPlayersWarning"
            @update="config.missingPlayersWarning = $event"
          />
        </Section>

        <Section title="Performance">
          <ToggleSetting
            label="Low-end device mode"
            :value="config.lowEndMode"
            @update="config.lowEndMode = $event"
          />
          <ToggleSetting
            label="Diagnostics logging"
            :value="config.perfLoggingEnabled"
            @update="config.perfLoggingEnabled = $event"
          />
        </Section>

        <Section title="Updates">
          <ToggleSetting
            label="Auto-update on launch"
            :value="config.autoUpdateEnabled"
            @update="config.autoUpdateEnabled = $event"
          />
          <div class="settings-row flex items-center justify-between">
            <span style="font-size: 0.8rem; color: var(--color-ink-2)">{{
              updateStatusText
            }}</span>
            <Button
              variant="control"
              size="sm"
              class="gap-1.5"
              :disabled="updateStatus === 'checking' || updateStatus === 'downloading'"
              @click="checkUpdate"
            >
              <RefreshCw :size="10" />
              Check
            </Button>
          </div>
          <div
            v-if="updateStatus === 'downloaded'"
            class="settings-row flex items-center justify-between"
          >
            <span style="font-size: 0.8rem; color: var(--color-good)"
              >Ready to install</span
            >
            <Button
              variant="control"
              size="sm"
              class="gap-1.5"
              @click="installUpdate"
            >
              <Download :size="10" />
              Restart &amp; install
            </Button>
          </div>
        </Section>

        <Section title="Danger Zone">
          <div class="settings-row flex items-center justify-between">
            <span style="font-size: 0.8rem; color: var(--color-ink-2)"
              >Reset all settings to defaults</span
            >
            <Button
              variant="control"
              size="sm"
              @click="config.$reset()"
            >
              Reset
            </Button>
          </div>
        </Section>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-outer {
  border-radius: 0;
  contain: paint;
}

@media (min-width: 640px) {
  .settings-outer {
    border-radius: 12px 12px 0 0;
    overflow: hidden;
  }
}

.settings-sidebar {
  width: 190px;
  font-size: 0.96rem;
}

.settings-content {
  font-size: 0.98rem;
  scrollbar-gutter: stable;
  contain: paint;
}

.settings-content :deep(.settings-section) {
  position: relative;
  overflow: hidden;
  background: rgba(255, 255, 255, 0.018) !important;
  border: 1px solid rgba(255, 255, 255, 0.06) !important;
  border-radius: 16px !important;
  box-shadow:
    0 0 0 1px rgba(0, 0, 0, 0.4),
    0 22px 70px 4px rgba(0, 0, 0, 0.56),
    inset 0 1px rgba(255, 255, 255, 0.035) !important;
  backdrop-filter: blur(14px) saturate(120%) !important;
  -webkit-backdrop-filter: blur(14px) saturate(120%) !important;
}

.settings-content :deep(.settings-section)::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: linear-gradient(
    90deg,
    transparent,
    rgba(var(--color-accent-rgb), 0.32),
    transparent
  );
  opacity: 0.7;
  pointer-events: none;
}

.settings-content :deep(.settings-section > .section-card) {
  position: relative;
  display: flex;
  flex-direction: column;
  background: rgba(255, 255, 255, 0.018) !important;
  border: 1px solid rgba(255, 255, 255, 0.055) !important;
  border-radius: 0 0 16px 16px !important;
  box-shadow:
    0 0 0 1px rgba(0, 0, 0, 0.3),
    0 -6px 18px rgba(0, 0, 0, 0.35),
    0 10px 26px rgba(0, 0, 0, 0.5),
    inset 0 1px rgba(255, 255, 255, 0.025) !important;
  backdrop-filter: blur(9px) saturate(108%) !important;
  -webkit-backdrop-filter: blur(9px) saturate(108%) !important;
}

.settings-content :deep(.settings-section .section-heading) {
  display: flex;
  align-items: center;
  min-height: 42px;
  padding: 0 0.9rem;
  background: transparent;
  border-radius: 0 !important;
}

.settings-content :deep(.settings-section .section-title) {
  font-size: 0.72rem;
  font-weight: 700;
  color: var(--color-accent);
  letter-spacing: 0.12em;
  text-transform: uppercase;
  line-height: 1.3;
}

.settings-content :deep(.settings-section .section-desc) {
  margin: 0;
  padding: 0 0.9rem 0.75rem;
  font-size: 0.78rem;
  line-height: 1.55;
  color: var(--color-ink-3);
  background: transparent;
  border-radius: 0 !important;
}

.settings-row {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 48px;
  margin: 0 -0.75rem;
  padding: 0.72rem 0.75rem;
  background: transparent;
  border: none;
  border-radius: 0 !important;
  transition: background 160ms ease;
}

.settings-row:hover {
  background: rgba(var(--color-accent-rgb), 0.055) !important;
}

.settings-content :deep(.settings-section > .section-card > .settings-row:last-child) {
  border-top: 1px solid rgba(255, 255, 255, 0.055) !important;
}

.settings-content
  :deep(.settings-section > .section-card > .settings-row:last-child:hover) {
  background: rgba(var(--color-accent-rgb), 0.055) !important;
}

.label-btn {
  font-size: 0.71rem;
  font-weight: 650;
  padding: 0.34rem 0.8rem;
  background: rgba(255, 255, 255, 0.015);
  color: var(--color-ink-3);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 8px;
  cursor: pointer;
  transition:
    background 150ms ease,
    color 150ms ease,
    border-color 150ms ease,
    box-shadow 150ms ease,
    transform 150ms ease;
}

.label-btn:hover {
  color: var(--color-ink-2);
  background: rgba(var(--color-accent-rgb), 0.055);
  border-color: rgba(var(--color-accent-rgb), 0.16);
}

.label-btn:active {
  transform: translateY(1px);
}

.label-btn--active {
  background: linear-gradient(
    180deg,
    rgba(var(--color-accent-rgb), 0.17),
    rgba(var(--color-accent-rgb), 0.075)
  );
  color: var(--color-accent-light);
  border-color: rgba(var(--color-accent-rgb), 0.34);
  box-shadow:
    0 0 18px rgba(var(--color-accent-rgb), 0.09),
    inset 0 1px rgba(255, 255, 255, 0.045);
}

html.low-end .label-btn--active {
  box-shadow: none;
}

.group-label {
  font-size: 0.66rem;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--color-ink-3);
}

.active-col-row,
.inactive-col-row,
.sort-list-row {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: 0.6rem 0.8rem;
  border-radius: 11px;
  border: 1px solid rgba(255, 255, 255, 0.07);
  background: linear-gradient(
    160deg,
    rgba(255, 255, 255, 0.04) 0%,
    rgba(255, 255, 255, 0.012) 100%
  );
  box-shadow:
    0 6px 16px rgba(0, 0, 0, 0.28),
    inset 0 1px rgba(255, 255, 255, 0.045);
  transition:
    background 150ms ease,
    border-color 150ms ease,
    box-shadow 150ms ease,
    transform 150ms ease,
    color 150ms ease;
}

.active-col-row {
  border-color: rgba(var(--color-accent-rgb), 0.3);
  background: linear-gradient(
    160deg,
    rgba(var(--color-accent-rgb), 0.1) 0%,
    rgba(var(--color-accent-rgb), 0.03) 100%
  );
  box-shadow:
    0 3px 10px rgba(0, 0, 0, 0.24),
    0 0 16px rgba(var(--color-accent-rgb), 0.14),
    inset 0 1px rgba(255, 255, 255, 0.06);
}

.active-col-row:hover {
  border-color: rgba(var(--color-accent-rgb), 0.48);
  box-shadow:
    0 4px 12px rgba(0, 0, 0, 0.28),
    0 0 22px rgba(var(--color-accent-rgb), 0.24),
    inset 0 1px rgba(255, 255, 255, 0.07);
  transform: translateY(-1px);
}

.inactive-col-row {
  cursor: pointer;
}

.inactive-col-row:hover {
  border-color: rgba(var(--color-accent-rgb), 0.3);
  background: linear-gradient(
    160deg,
    rgba(var(--color-accent-rgb), 0.07) 0%,
    rgba(255, 255, 255, 0.012) 100%
  );
  box-shadow:
    0 4px 12px rgba(0, 0, 0, 0.26),
    0 0 16px rgba(var(--color-accent-rgb), 0.12),
    inset 0 1px rgba(255, 255, 255, 0.05);
  transform: translateY(-1px);
}

html.low-end .active-col-row,
html.low-end .inactive-col-row,
html.low-end .sort-list-row {
  box-shadow: none !important;
  transform: none !important;
}

.add-label {
  font-size: 0.66rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  color: var(--color-accent-light);
  opacity: 0.7;
  transition: opacity 150ms ease;
}

.inactive-col-row:hover .add-label {
  opacity: 1;
}

.col-reorder-group {
  display: flex;
  align-items: center;
  gap: 6px;
}

.col-reorder-btn,
.col-remove-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  flex-shrink: 0;
  border-radius: 7px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: linear-gradient(
    160deg,
    rgba(255, 255, 255, 0.05) 0%,
    rgba(255, 255, 255, 0.015) 100%
  );
  box-shadow:
    0 0 0 1px rgba(0, 0, 0, 0.3),
    0 2px 6px rgba(0, 0, 0, 0.3),
    inset 0 1px rgba(255, 255, 255, 0.06);
  color: var(--color-ink-3);
  cursor: pointer;
  transition:
    background 140ms ease,
    color 140ms ease,
    border-color 140ms ease,
    box-shadow 140ms ease,
    transform 120ms ease;
}

.col-reorder-btn:hover:not(:disabled) {
  border-color: rgba(var(--color-accent-rgb), 0.4);
  background: linear-gradient(
    160deg,
    rgba(var(--color-accent-rgb), 0.22) 0%,
    rgba(var(--color-accent-rgb), 0.07) 100%
  );
  color: var(--color-accent-light);
  box-shadow:
    0 0 0 1px rgba(0, 0, 0, 0.3),
    0 0 12px rgba(var(--color-accent-rgb), 0.28),
    inset 0 1px rgba(255, 255, 255, 0.08);
}

.col-reorder-btn:active:not(:disabled) {
  transform: translateY(1px);
}

.col-reorder-btn:disabled {
  opacity: 0.28;
  cursor: not-allowed;
  box-shadow: none;
}

.col-remove-btn:hover:not(:disabled) {
  border-color: rgba(248, 113, 113, 0.4);
  background: linear-gradient(
    160deg,
    rgba(248, 113, 113, 0.22) 0%,
    rgba(248, 113, 113, 0.07) 100%
  );
  color: #f87171;
  box-shadow:
    0 0 0 1px rgba(0, 0, 0, 0.3),
    0 0 12px rgba(248, 113, 113, 0.28),
    inset 0 1px rgba(255, 255, 255, 0.08);
}

.col-remove-btn:active:not(:disabled) {
  transform: translateY(1px);
}

.col-remove-btn:disabled {
  opacity: 0.28;
  cursor: not-allowed;
  box-shadow: none;
}

html.low-end .col-reorder-btn,
html.low-end .col-remove-btn {
  box-shadow: none !important;
  transform: none !important;
}

.sort-list-row {
  gap: 0.6rem;
  color: var(--color-ink-2);
  font-family: inherit;
  font-size: 0.8rem;
  font-weight: 500;
  text-align: left;
  cursor: pointer;
  justify-content: flex-start;
}

.sort-list-row:hover {
  border-color: rgba(var(--color-accent-rgb), 0.3);
  background: linear-gradient(
    160deg,
    rgba(var(--color-accent-rgb), 0.07) 0%,
    rgba(255, 255, 255, 0.012) 100%
  );
  box-shadow:
    0 0 0 1px rgba(0, 0, 0, 0.3),
    0 8px 18px rgba(0, 0, 0, 0.3),
    0 0 16px rgba(var(--color-accent-rgb), 0.12),
    inset 0 1px rgba(255, 255, 255, 0.05);
  color: var(--color-ink-1);
  transform: translateY(-1px);
}

.sort-list-row--active {
  border-color: rgba(var(--color-accent-rgb), 0.48);
  background: linear-gradient(
    160deg,
    rgba(var(--color-accent-rgb), 0.14) 0%,
    rgba(var(--color-accent-rgb), 0.04) 100%
  );
  color: var(--color-accent-light);
  box-shadow:
    0 0 0 1px rgba(0, 0, 0, 0.3),
    0 6px 16px rgba(0, 0, 0, 0.28),
    0 0 20px rgba(var(--color-accent-rgb), 0.24),
    inset 0 1px rgba(255, 255, 255, 0.07);
}

.sort-list-row--active:hover {
  border-color: rgba(var(--color-accent-rgb), 0.6);
  transform: translateY(-1px);
  box-shadow:
    0 0 0 1px rgba(0, 0, 0, 0.3),
    0 8px 20px rgba(0, 0, 0, 0.32),
    0 0 26px rgba(var(--color-accent-rgb), 0.32),
    inset 0 1px rgba(255, 255, 255, 0.08);
}

html.low-end .sort-list-row--active {
  box-shadow: none !important;
  transform: none !important;
}

.sort-radio {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  flex-shrink: 0;
  border-radius: 50%;
  border: 1.5px solid rgba(255, 255, 255, 0.2);
  transition: border-color 150ms ease;
}

.sort-list-row--active .sort-radio {
  border-color: var(--color-accent);
}

.sort-radio-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--color-accent);
  box-shadow: 0 0 6px rgba(var(--color-accent-rgb), 0.7);
}

.setting-panel {
  padding: 0.75rem 0.9rem;
  border-radius: 13px;
  border: 1px solid rgba(255, 255, 255, 0.07);
  background: linear-gradient(
    160deg,
    rgba(255, 255, 255, 0.035) 0%,
    rgba(255, 255, 255, 0.01) 100%
  );
  box-shadow:
    0 0 0 1px rgba(0, 0, 0, 0.28),
    0 8px 20px rgba(0, 0, 0, 0.3),
    inset 0 1px rgba(255, 255, 255, 0.045);
  transition:
    border-color 150ms ease,
    box-shadow 150ms ease;
}

.setting-panel:hover {
  border-color: rgba(var(--color-accent-rgb), 0.18);
}

html.low-end .setting-panel {
  box-shadow: none !important;
}

.panel-label {
  font-size: 0.82rem;
  font-weight: 600;
  color: var(--color-ink-1);
}

.panel-desc {
  font-size: 0.71rem;
  line-height: 1.5;
  color: var(--color-ink-3);
}

.network-select {
  display: inline-flex;
  gap: 4px;
  padding: 4px;
  border-radius: 13px;
  background: rgba(0, 0, 0, 0.16);
  border: 1px solid rgba(255, 255, 255, 0.07);
  box-shadow:
    0 0 0 1px rgba(0, 0, 0, 0.28),
    inset 0 2px 6px rgba(0, 0, 0, 0.35);
}

.network-seg {
  padding: 0.45rem 1.1rem;
  border-radius: 9px;
  border: 1px solid transparent;
  background: transparent;
  color: var(--color-ink-3);
  font-family: inherit;
  font-size: 0.78rem;
  font-weight: 600;
  cursor: pointer;
  transition:
    background 150ms ease,
    color 150ms ease,
    box-shadow 150ms ease,
    border-color 150ms ease,
    transform 120ms ease;
}

.network-seg:hover {
  color: var(--color-ink-1);
  background: rgba(255, 255, 255, 0.06);
}

.network-seg--active {
  border-color: currentColor;
}

.network-seg:active {
  transform: translateY(1px);
}

html.low-end .network-seg--active {
  box-shadow: none !important;
}

.proxy-card {
  position: relative;
  border-radius: 14px;
  contain: paint;
  background: linear-gradient(
    160deg,
    rgba(255, 255, 255, 0.035) 0%,
    rgba(255, 255, 255, 0.012) 100%
  );
  border: 1px solid rgba(255, 255, 255, 0.07);
  box-shadow:
    0 0 0 1px rgba(0, 0, 0, 0.25),
    0 10px 26px rgba(0, 0, 0, 0.35),
    inset 0 1px rgba(255, 255, 255, 0.035);
  backdrop-filter: blur(10px) saturate(120%);
  -webkit-backdrop-filter: blur(10px) saturate(120%);
  transition:
    border-color 150ms ease,
    box-shadow 150ms ease;
}

.proxy-card:hover {
  border-color: rgba(var(--color-accent-rgb), 0.22);
  box-shadow:
    0 0 0 1px rgba(0, 0, 0, 0.25),
    0 0 20px rgba(var(--color-accent-rgb), 0.1),
    0 10px 26px rgba(0, 0, 0, 0.35),
    inset 0 1px rgba(255, 255, 255, 0.035);
}

html.low-end .proxy-card {
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}

.proxy-status-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
  transition:
    background 200ms ease,
    box-shadow 200ms ease;
}

.status-pill {
  display: inline-flex;
  align-items: center;
  padding: 0.12rem 0.45rem;
  border-radius: 5px;
  font-size: 0.6rem;
  font-weight: 700;
  letter-spacing: 0.06em;
  line-height: 1.4;
}

.status-pill--active {
  background: rgba(52, 211, 153, 0.14);
  border: 1px solid rgba(52, 211, 153, 0.32);
  color: #34d399;
  box-shadow: 0 0 10px rgba(52, 211, 153, 0.22);
}

.status-pill--error {
  background: rgba(248, 113, 113, 0.14);
  border: 1px solid rgba(248, 113, 113, 0.32);
  color: #f87171;
  box-shadow: 0 0 10px rgba(248, 113, 113, 0.22);
}

html.low-end .status-pill--active,
html.low-end .status-pill--error {
  box-shadow: none !important;
}

.proxy-dest-row {
  background: rgba(0, 0, 0, 0.16);
  border: 1px solid rgba(255, 255, 255, 0.06);
  box-shadow: inset 0 2px 6px rgba(0, 0, 0, 0.3);
}

.field-label {
  font-size: 0.6rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--color-ink-3);
}

.proxy-client-badge {
  display: inline-flex;
  align-items: center;
  padding: 0.1rem 0.4rem;
  border-radius: 5px;
  font-size: 0.62rem;
  font-weight: 600;
  color: var(--color-accent-light);
  background: rgba(var(--color-accent-rgb), 0.12);
  border: 1px solid rgba(var(--color-accent-rgb), 0.24);
}

.proxy-port-input {
  -webkit-app-region: no-drag;
  width: 72px;
  height: 26px;
  padding: 0 0.5rem;
  border-radius: 7px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--color-border);
  color: var(--color-ink-1);
  font-size: 0.76rem;
  text-align: center;
  outline: none;
  transition:
    border-color 140ms ease,
    background 140ms ease,
    box-shadow 140ms ease;
}

.proxy-port-input:focus {
  border-color: rgba(var(--color-accent-rgb), 0.55);
  background: rgba(var(--color-accent-rgb), 0.07);
  box-shadow: 0 0 0 2.5px rgba(var(--color-accent-rgb), 0.16);
}

.proxy-port-input::-webkit-inner-spin-button,
.proxy-port-input::-webkit-outer-spin-button {
  opacity: 0.4;
}

.settings-content :deep(button[data-variant='control']),
.settings-content :deep(.btn-control-v2) {
  transition:
    background 140ms ease,
    border-color 140ms ease,
    color 140ms ease,
    box-shadow 140ms ease,
    transform 120ms ease;
}

.settings-content :deep(button[data-variant='control']:hover),
.settings-content :deep(.btn-control-v2:hover) {
  box-shadow: 0 0 14px rgba(var(--color-accent-rgb), 0.06);
}

.settings-content :deep(button[data-variant='control']:active),
.settings-content :deep(.btn-control-v2:active) {
  transform: translateY(1px);
}

.settings-content button:focus-visible,
.settings-content input:focus-visible {
  outline: 1px solid rgba(var(--color-accent-rgb), 0.55);
  outline-offset: 2px;
  box-shadow: 0 0 0 3px rgba(var(--color-accent-rgb), 0.08);
}

.animate-fade-in {
  animation: settings-fade-in 180ms ease-out both;
}

@keyframes settings-fade-in {
  from {
    opacity: 0;
    transform: translateY(3px);
  }

  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@media (prefers-reduced-motion: reduce) {
  .animate-fade-in {
    animation: none;
  }

  .settings-row,
  .setting-panel,
  .active-col-row,
  .inactive-col-row,
  .sort-list-row,
  .sort-radio,
  .proxy-card,
  .proxy-status-dot,
  .proxy-port-input,
  .network-seg,
  .label-btn,
  .col-reorder-btn,
  .col-remove-btn {
    transition: none;
  }
}
</style>
