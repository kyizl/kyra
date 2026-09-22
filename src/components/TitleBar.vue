<script setup lang="ts">
import type { BedwarsMode, Interval } from '@renderer/types';
import { Button } from '@renderer/components/ui/button';
import { dropEnter, dropLeave, duration as motionDuration } from '@renderer/lib/motion';
import { useConfigStore } from '@renderer/store/config';
import { usePlayersStore } from '@renderer/store/players';
import { onClickOutside } from '@vueuse/core';
import {
  Activity,
  Camera,
  ChevronDown,
  Home,
  LifeBuoy,
  Menu,
  Minus,
  Palette,
  Plus,
  Settings,
  ShieldAlert,
  Tag,
  Trash2,
  Wrench,
  X,
} from 'lucide-vue-next';
import { computed, inject, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';

const config = useConfigStore();
const players = usePlayersStore();
const route = useRoute();
const router = useRouter();

const addInput = ref('');
const menuOpen = ref(false);
const menuRef = ref<HTMLElement | null>(null);
const modeOpen = ref(false);
const modeRef = ref<HTMLElement | null>(null);
const intervalOpen = ref(false);
const intervalRef = ref<HTMLElement | null>(null);

const headerHovered = inject<ReturnType<typeof ref<boolean>>>(
  'headerHovered',
  ref(false),
);
const dropdownOpen = inject<ReturnType<typeof ref<boolean>>>('dropdownOpen', ref(false));
const setScreenshotMode = inject<(active: boolean) => void>(
  'setScreenshotMode',
  () => {},
);

watch(menuOpen, (isOpen) => {
  dropdownOpen.value = isOpen;
});
watch(modeOpen, (isOpen) => {
  if (isOpen) dropdownOpen.value = true;
});
watch(intervalOpen, (isOpen) => {
  if (isOpen) dropdownOpen.value = true;
});

onClickOutside(menuRef, () => {
  menuOpen.value = false;
});
onClickOutside(modeRef, () => {
  modeOpen.value = false;
});
onClickOutside(intervalRef, () => {
  intervalOpen.value = false;
});

const integratedHeaderStyle = computed(() => {
  if (!config.integratedMode) return {};
  const visible = headerHovered.value || menuOpen.value;
  return visible
    ? {
        opacity: '1',
        background: 'rgba(var(--color-bg-rgb), 0.97)',
        transition: 'opacity 0.15s ease, background 0.15s ease',
      }
    : { opacity: '0.05', transition: 'opacity 0.25s ease' };
});

const isHome = computed(() => route.name === 'Home');
const isSettings = computed(() => route.name === 'Settings');
const isSetup = computed(() => route.name === 'Setup');
const isQueue = computed(() => route.name === 'Queue');
const isSupport = computed(() => route.name === 'Support');

const isJartex = computed(() => config.network === 'jartexnetwork');
const networkAccent = computed(() => (isJartex.value ? '#22d3ee' : '#ffea00'));
const networkLabel = computed(() => (isJartex.value ? 'JartexNetwork' : 'PikaNetwork'));

const modes: [BedwarsMode, string][] = [
  ['ALL_MODES', 'All Modes'],
  ['SOLO', 'Solo'],
  ['DOUBLES', 'Doubles'],
  ['TRIPLES', 'Triples'],
  ['QUAD', 'Quads'],
];

const intervals: [Interval, string][] = [
  ['total', 'Overall'],
  ['weekly', 'Weekly'],
  ['monthly', 'Monthly'],
  ['yearly', 'Yearly'],
];

const endpoints = [
  { to: '/', label: 'Home', icon: Home },
  { to: '/nicks', label: 'Nicks', icon: Tag },
  { to: '/theme', label: 'Theme', icon: Palette },
  { to: '/queue', label: 'Queue', icon: ShieldAlert },
  { to: '/support', label: 'Support', icon: LifeBuoy },
  { to: '/settings', label: 'Settings', icon: Settings },
  { to: '/setup', label: 'Setup', icon: Wrench },
  { to: '/benchmark', label: 'Benchmark', icon: Activity },
];

const modeLabel = computed(
  () => modes.find(([mode]) => mode === config.mode)?.[1] ?? 'All Modes',
);
const intervalLabel = computed(
  () => intervals.find(([interval]) => interval === config.interval)?.[1] ?? 'Overall',
);

function onMinimize() {
  window.api.win.minimize();
}
function onClose() {
  window.api.win.close();
}

async function submitAdd(): Promise<void> {
  const names = addInput.value
    .trim()
    .split(/[\s,]+/)
    .filter(Boolean)
    .filter((name) => /^\w{1,16}$/.test(name));

  if (names.length === 0) {
    addInput.value = '';
    return;
  }

  await players.addNames(names, 'manual');
  addInput.value = '';
  if (route.name !== 'Home') router.replace('/');
}

async function selectMode(mode: BedwarsMode): Promise<void> {
  config.mode = mode;
  modeOpen.value = false;
  await players.refreshAllStats(config.interval, config.mode);
}

async function selectInterval(interval: Interval): Promise<void> {
  config.interval = interval;
  intervalOpen.value = false;
  await players.refreshAllStats(config.interval, config.mode);
}

function waitForNextPaint(): Promise<void> {
  return new Promise((resolve) => {
    requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
  });
}

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function doScreenshot(): Promise<void> {
  const hadOpenMenus = menuOpen.value || modeOpen.value || intervalOpen.value;
  menuOpen.value = false;
  modeOpen.value = false;
  intervalOpen.value = false;

  if (hadOpenMenus) {
    await wait(motionDuration.instant * 1000 + 40);
  }

  setScreenshotMode(true);

  try {
    await waitForNextPaint();
    await wait(60);
    await window.api.win.screenshot();
  } finally {
    setScreenshotMode(false);
  }
}
</script>

<template>
  <header
    class="flex shrink-0 items-center justify-between px-2.5"
    :class="{ 'is-integrated': config.integratedMode }"
    :style="[
      {
        height: '40px',
        background: 'var(--panel-bg)',
        backdropFilter: 'none',
        WebkitBackdropFilter: 'none',
      },
      integratedHeaderStyle,
    ]"
  >
    <div class="drag flex h-full min-w-0 flex-1 items-center gap-2 pl-0.5">
      <div
        class="brand-cluster"
        style="
          --brand-accent: var(--color-accent);
          --brand-accent-soft: var(--color-accent-light);
        "
      >
        <svg
          width="18"
          height="18"
          viewBox="0 0 20 20"
          fill="none"
          class="brand-icon shrink-0"
        >
          <defs>
            <linearGradient
              id="sg1"
              x1="0"
              y1="0"
              x2="1"
              y2="1"
            >
              <stop
                offset="0%"
                style="stop-color: var(--brand-accent-soft)"
              />
              <stop
                offset="100%"
                style="stop-color: var(--brand-accent)"
              />
            </linearGradient>
            <linearGradient
              id="sg2"
              x1="1"
              y1="0"
              x2="0"
              y2="1"
            >
              <stop
                offset="0%"
                style="stop-color: var(--brand-accent)"
              />
              <stop
                offset="100%"
                style="stop-color: var(--brand-accent)"
              />
            </linearGradient>
          </defs>
          <path
            d="M10 1 L12 8 L19 10 L12 12 L10 19 L8 12 L1 10 L8 8 Z"
            fill="url(#sg1)"
          />
          <path
            d="M10 1 L12 8 L10 10 L8 8 Z"
            fill="url(#sg2)"
            opacity="0.7"
          />
          <path
            d="M19 10 L12 12 L10 10 L12 8 Z"
            fill="url(#sg2)"
            opacity="0.5"
          />
          <path
            d="M10 19 L8 12 L10 10 L12 12 Z"
            fill="url(#sg2)"
            opacity="0.7"
          />
          <path
            d="M1 10 L8 8 L10 10 L8 12 Z"
            fill="url(#sg2)"
            opacity="0.5"
          />
          <circle
            cx="10"
            cy="10"
            r="1.5"
            fill="#fff"
            opacity="0.55"
          />
        </svg>
        <div class="brand-text-group">
          <span
            class="brand-mark brand-wordmark brand-shine gradient-ink font-normal tracking-widest"
            style="
              font-size: 1rem;
              letter-spacing: 0.12em;
              font-family: 'WsParadose', 'Outfit', sans-serif;
              font-weight: 400;
              text-transform: uppercase;
            "
            data-text="KYRA"
            >KYRA</span
          >
          <span class="brand-subtitle">overlay</span>
        </div>
      </div>
      <div
        class="network-chip"
        :style="{
          '--chip-color': networkAccent,
          '--chip-bg': isJartex ? 'rgba(14, 116, 144, 0.10)' : 'rgba(180, 120, 0, 0.10)',
          '--chip-border': isJartex
            ? 'rgba(34, 211, 238, 0.5)'
            : 'rgba(255, 214, 0, 0.5)',
        }"
      >
        {{ networkLabel }}
      </div>
    </div>

    <div class="no-drag flex items-center gap-1.5">
      <template v-if="!isSettings && !isSetup && !isQueue && !isSupport">
        <div class="relative">
          <input
            v-model.trim="addInput"
            placeholder="Add player…"
            class="input-field"
            style="height: 27px; width: 132px; padding-right: 26px; font-size: 0.78rem"
            @keydown.enter="submitAdd"
            @keydown.escape="addInput = ''"
          />
          <Button
            v-if="addInput"
            variant="subtle"
            size="icon-sm"
            :magnetic="6"
            class="absolute inset-y-0 right-1 my-auto"
            @click="submitAdd"
          >
            <Plus :size="11" />
          </Button>
        </div>
      </template>

      <template v-if="isHome">
        <div
          ref="modeRef"
          class="relative"
        >
          <button
            class="custom-select no-drag"
            :class="{ 'custom-select--open': modeOpen }"
            @click="modeOpen = !modeOpen"
          >
            <span>{{ modeLabel }}</span>
            <ChevronDown
              :size="10"
              class="custom-select-chevron"
              :class="{ 'custom-select-chevron--open': modeOpen }"
            />
          </button>
          <Transition
            :css="false"
            @enter="dropEnter"
            @leave="dropLeave"
          >
            <div
              v-if="modeOpen"
              class="custom-dropdown"
            >
              <button
                v-for="[modeValue, modeName] in modes"
                :key="modeValue"
                class="custom-dropdown-item"
                :class="{ 'custom-dropdown-item--active': config.mode === modeValue }"
                @click="selectMode(modeValue)"
              >
                <span class="custom-dropdown-item-label">{{ modeName }}</span>
              </button>
            </div>
          </Transition>
        </div>

        <div
          ref="intervalRef"
          class="relative"
        >
          <button
            class="custom-select no-drag"
            :class="{ 'custom-select--open': intervalOpen }"
            @click="intervalOpen = !intervalOpen"
          >
            <span>{{ intervalLabel }}</span>
            <ChevronDown
              :size="10"
              class="custom-select-chevron"
              :class="{ 'custom-select-chevron--open': intervalOpen }"
            />
          </button>
          <Transition
            :css="false"
            @enter="dropEnter"
            @leave="dropLeave"
          >
            <div
              v-if="intervalOpen"
              class="custom-dropdown"
            >
              <button
                v-for="[intervalValue, intervalName] in intervals"
                :key="intervalValue"
                class="custom-dropdown-item"
                :class="{
                  'custom-dropdown-item--active': config.interval === intervalValue,
                }"
                @click="selectInterval(intervalValue)"
              >
                <span class="custom-dropdown-item-label">{{ intervalName }}</span>
              </button>
            </div>
          </Transition>
        </div>

        <Button
          variant="ghost"
          size="icon"
          @click="players.clear()"
        >
          <Trash2 :size="13" />
        </Button>
      </template>

      <div
        class="mx-0.5 h-3.5 w-px"
        style="background: var(--color-border)"
      />

      <div
        ref="menuRef"
        class="relative"
      >
        <Button
          variant="ghost"
          size="icon"
          @click="menuOpen = !menuOpen"
        >
          <Menu :size="13" />
        </Button>

        <Transition
          :css="false"
          @enter="dropEnter"
          @leave="dropLeave"
        >
          <nav
            v-if="menuOpen"
            class="absolute top-full right-0 z-50 mt-1 overflow-hidden"
            style="
              width: 152px;
              display: flex;
              flex-direction: column;
              gap: 2px;
              background: rgba(var(--color-bg-rgb), 0.98);
              border: 1px solid rgba(var(--color-accent-rgb), 0.18);
              border-radius: var(--radius-md);
              box-shadow:
                0 8px 36px rgba(0, 0, 0, 0.65),
                0 0 0 1px rgba(var(--color-accent-rgb), 0.06);
              padding: 4px;
            "
            @click.stop
          >
            <router-link
              v-for="item in endpoints"
              :key="item.to"
              :to="item.to"
              class="nav-item flex items-center gap-2.5 px-2.5 py-2 transition-colors"
              :class="{ 'nav-item--active': route.path === item.to }"
              @click="menuOpen = false"
            >
              <component
                :is="item.icon"
                :size="12"
                class="shrink-0"
              />
              <span class="nav-item-label">{{ item.label }}</span>
            </router-link>

            <div
              class="mx-1.5 my-0.5 border-t"
              style="border-color: rgba(var(--color-accent-rgb), 0.12)"
            />

            <button
              class="nav-item nav-item--muted flex w-full items-center gap-2.5 px-2.5 py-2 transition-colors"
              @click="doScreenshot"
            >
              <Camera :size="12" />
              <span class="nav-item-label">Screenshot</span>
            </button>
          </nav>
        </Transition>
      </div>

      <Button
        variant="ghost"
        size="icon"
        @click="onMinimize"
      >
        <Minus :size="12" />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        class="close-btn-v2"
        @click="onClose"
      >
        <X :size="11" />
      </Button>
    </div>
  </header>
</template>

<style scoped>
.network-chip {
  display: inline-flex;
  flex-shrink: 0;
  align-items: center;
  height: 20px;
  padding: 0 8px;
  border-radius: 999px;
  background: var(--chip-bg);
  border: 1px solid var(--chip-border);
  color: var(--chip-color);
  font-family: Inter, system-ui, sans-serif;
  font-size: 10.5px;
  font-weight: 500;
  letter-spacing: 0.02em;
  line-height: 1;
  white-space: nowrap;
  -webkit-font-smoothing: antialiased;
  transition:
    background 0.2s ease,
    border-color 0.2s ease;
}

.input-field {
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(var(--color-accent-rgb), 0.18);
  color: var(--color-ink-2);
}

.input-field:hover {
  border-color: rgba(var(--color-accent-rgb), 0.38);
  background: rgba(var(--color-accent-rgb), 0.08);
  color: var(--color-ink-1);
}

.input-field:focus {
  border-color: rgba(var(--color-accent-rgb), 0.55);
  background: rgba(var(--color-accent-rgb), 0.08);
  color: var(--color-ink-1);
}

.input-field::placeholder {
  color: var(--color-ink-2);
  font-weight: 500;
  opacity: 1;
}

.custom-select {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 27px;
  padding: 0 9px;
  border-radius: var(--radius-sm);
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(var(--color-accent-rgb), 0.18);
  color: var(--color-ink-2);
  font-family: var(--font-sans);
  font-size: 0.78rem;
  font-weight: 500;
  cursor: pointer;
  white-space: nowrap;
  line-height: 1;
  transition:
    border-color 150ms ease,
    background 150ms ease,
    color 150ms ease;
  -webkit-app-region: no-drag;
}

.custom-select span {
  display: inline-flex;
  align-items: center;
  line-height: 1;
  position: relative;
  top: 1px;
}

.custom-select-chevron {
  display: inline-flex;
  align-items: center;
}

.custom-select {
  min-width: 110px;
}

.custom-select:hover,
.custom-select--open {
  border-color: rgba(var(--color-accent-rgb), 0.38);
  background: rgba(var(--color-accent-rgb), 0.08);
  color: var(--color-ink-1);
}

.custom-select-chevron {
  color: var(--color-ink-3);
  flex-shrink: 0;
  transition: transform 160ms cubic-bezier(0.34, 1.56, 0.64, 1);
}

.custom-select-chevron--open {
  transform: rotate(180deg);
}

.custom-dropdown {
  position: absolute;
  top: calc(100% + 5px);
  left: 0;
  z-index: 50;
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 100%;
  background: rgba(var(--color-bg-rgb), 0.9);
  backdrop-filter: blur(16px) saturate(150%);
  -webkit-backdrop-filter: blur(16px) saturate(150%);
  border: 1px solid rgba(var(--color-accent-rgb), 0.18);
  border-radius: var(--radius-md);
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.6),
    0 0 0 1px rgba(var(--color-accent-rgb), 0.06);
  padding: 5px;
  overflow: hidden;
}

html.low-end .custom-dropdown {
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
  background: rgba(var(--color-bg-rgb), 0.98);
}

.custom-dropdown-item {
  display: flex;
  align-items: center;
  gap: 7px;
  width: 100%;
  padding: 7px 10px;
  border-radius: 6px;
  font-family: var(--font-sans);
  font-size: 0.78rem;
  font-weight: 500;
  line-height: 1;
  color: var(--color-ink-2);
  cursor: pointer;
  transition:
    background 140ms cubic-bezier(0.25, 0.46, 0.45, 0.94),
    color 140ms ease;
  -webkit-app-region: no-drag;
  white-space: nowrap;
}

.custom-dropdown-item-label {
  position: relative;
  top: 1px;
}

.custom-dropdown-item:hover {
  background: rgba(255, 255, 255, 0.06);
  color: var(--color-ink-1);
}

.custom-dropdown-item--active {
  color: var(--color-accent-light);
  background: rgba(var(--color-accent-rgb), 0.12);
}

.custom-dropdown-item--active:hover {
  background: rgba(var(--color-accent-rgb), 0.18);
}

.nav-item {
  font-size: 0.81rem;
  font-weight: 500;
  line-height: 1;
  border-radius: 6px;
  color: var(--color-ink-2);
}

.nav-item-label {
  position: relative;
  top: 1px;
}

.nav-item:hover {
  background: rgba(255, 255, 255, 0.05);
  color: var(--color-ink-1);
}

.nav-item--active {
  color: var(--color-accent-light);
  background: rgba(var(--color-accent-rgb), 0.14);
}

.nav-item--active:hover {
  background: rgba(var(--color-accent-rgb), 0.18);
}

.nav-item--muted {
  color: var(--color-ink-3);
}

.nav-item--muted:hover {
  color: var(--color-ink-2);
  background: rgba(255, 255, 255, 0.04);
}

.brand-cluster {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.55rem;
  height: 100%;
  line-height: 1;
}

.brand-icon {
  display: block;
  width: 18px;
  height: 18px;
  flex-shrink: 0;
  margin: 0;
  align-self: center;
}

.brand-text-group {
  display: inline-flex;
  align-items: flex-end;
  justify-content: center;
  gap: 0.35rem;
  line-height: 1;
}

.brand-mark {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
  letter-spacing: inherit;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-rendering: geometricPrecision;
  backface-visibility: hidden;
  transform: translateZ(0);
}

.brand-wordmark {
  transform: none;
}

.brand-shine {
  position: relative;
  isolation: isolate;
}

.brand-shine::after {
  content: attr(data-text);
  position: absolute;
  inset: 0;
  background: linear-gradient(
    115deg,
    transparent 35%,
    rgba(255, 255, 255, 0.95) 48%,
    rgba(255, 255, 255, 0.95) 52%,
    transparent 65%
  );
  background-size: 260% 100%;
  background-position: 150% 0;
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
  mix-blend-mode: overlay;
  animation: brand-shine-sweep 5.5s ease-in-out infinite;
  animation-delay: 1.2s;
  animation-play-state: var(--anim-play-state, running);
  pointer-events: none;
}

@keyframes brand-shine-sweep {
  0% {
    background-position: 150% 0;
  }
  35%,
  100% {
    background-position: -60% 0;
  }
}

html.low-end .brand-shine::after {
  animation: none !important;
  content: none;
}

@media (prefers-reduced-motion: reduce) {
  .brand-shine::after {
    animation: none !important;
    content: none;
  }
}

.brand-subtitle {
  position: relative;
  bottom: 1px;
  font-size: 0.7rem;
  color: var(--color-ink-3);
  font-weight: 400;
  letter-spacing: 0.02em;
  line-height: 1;
}

.close-btn-v2:hover {
  background: rgba(239, 68, 68, 0.65) !important;
  color: #fff !important;
}
</style>
