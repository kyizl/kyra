<script setup lang="ts">
import type { ThemeColors } from '@renderer/store/config';
import type { ProxyEventPayload } from '@renderer/types';
import LoadingScreen from '@renderer/components/LoadingScreen.vue';
import AnnouncementModal from '@renderer/components/modals/AnnouncementModal.vue';
import DiscordLinkModal from '@renderer/components/modals/DiscordLinkModal.vue';
import PremiumSignInModal from '@renderer/components/modals/PremiumSignInModal.vue';
import PerfOverlay from '@renderer/components/PerfOverlay.vue';
import QueueSafetyBanner from '@renderer/components/queue/QueueSafetyBanner.vue';
import TitleBar from '@renderer/components/TitleBar.vue';
import { useAnnouncements } from '@renderer/composables/useAnnouncements';
import { parseLine } from '@renderer/composables/useLogParser';
import { usePremiumAuth } from '@renderer/composables/usePremiumAuth';
import { useQueueSafety } from '@renderer/composables/useQueueSafety';
import { useTelemetryAuth } from '@renderer/composables/useTelemetryAuth';
import { mark, startPerfLogging, stopPerfLogging } from '@renderer/lib/perf-bus';
import { useConfigStore } from '@renderer/store/config';
import { useNicksStore } from '@renderer/store/nicks';
import { usePlayersStore } from '@renderer/store/players';
import { computed, onMounted, onUnmounted, provide, ref, watch } from 'vue';
import { useRouter } from 'vue-router';

const config = useConfigStore();
const players = usePlayersStore();
const nicks = useNicksStore();
const router = useRouter();

const screenshotMode = ref(false);

function updateLowEndClass(): void {
  document.documentElement.classList.toggle(
    'low-end',
    config.lowEndMode || screenshotMode.value,
  );
}

updateLowEndClass();
watch(
  () => config.lowEndMode,
  () => {
    updateLowEndClass();
    applyThemeVars();
  },
);

function setScreenshotMode(active: boolean): void {
  if (screenshotMode.value === active) return;
  screenshotMode.value = active;
  updateLowEndClass();
  if (active) {
    document.documentElement.classList.toggle('window-unfocused', false);
    applyThemeVars();
    applyPanelBlur();
  } else {
    applyThemeVars();
    updateAnimPlayState();
  }
}

provide('setScreenshotMode', setScreenshotMode);

watch(() => config.integratedMode, applyThemeVars);

watch(
  () => config.alwaysOnTop,
  (enabled) => window.api.win.setAlwaysOnTop(enabled),
  { immediate: true },
);

watch(
  () => config.perfLoggingEnabled,
  (enabled) => {
    if (enabled) startPerfLogging();
    else stopPerfLogging();
  },
  { immediate: true },
);

const windowFocused = ref(document.hasFocus());

function updateAnimPlayState(): void {
  const shouldPause = document.hidden || !windowFocused.value;
  document.documentElement.style.setProperty(
    '--anim-play-state',
    shouldPause ? 'paused' : 'running',
  );
  document.documentElement.classList.toggle('window-unfocused', !windowFocused.value);
  applyPanelBlur();
}

function handleVisibilityChange(): void {
  updateAnimPlayState();
}

function handleWindowFocus(): void {
  windowFocused.value = true;
  updateAnimPlayState();
}

function handleWindowBlur(): void {
  windowFocused.value = false;
  updateAnimPlayState();
  clearPendingWake();
  setIgnore(true);
}

const { activeAnnouncement, fetchAnnouncements, dismissActive } = useAnnouncements();

const {
  active: activePremiumAuth,
  handleProxyAuthEvent,
  dismiss: dismissPremiumAuth,
} = usePremiumAuth();

const {
  verdict: queueSafetyVerdict,
  visible: queueSafetyVisibleRaw,
  dismiss: dismissQueueSafety,
} = useQueueSafety();

const queueSafetyVisible = computed(
  () => queueSafetyVisibleRaw.value && router.currentRoute.value.name !== 'Queue',
);

const {
  checked: discordChecked,
  linked: discordLinked,
  checking: discordChecking,
  needsLink: needsDiscordLink,
  linking: discordLinking,
  errorMessage: discordLinkError,
  refreshStatus: refreshDiscordLinkStatus,
  startLink: startDiscordLink,
  bypassLink: bypassDiscordLink,
  handleTelemetryEvent,
} = useTelemetryAuth();

const skipLoading = localStorage.getItem('skip-loading') === '1';
const skipAnnouncements = localStorage.getItem('skip-announcements') === '1';
const skipDiscordLink = localStorage.getItem('skip-discord-link') === '1';

const loadingDone = ref(skipLoading);
const appVisible = ref(skipLoading);
const postLoadingReady = ref(skipLoading);

function onLoadingDone(): void {
  loadingDone.value = true;
  mark('app-visible');

  requestAnimationFrame(() => {
    appVisible.value = true;
  });

  if (!skipAnnouncements) {
    void fetchAnnouncements();
  }
}

function onLoadingScreenGone(): void {
  postLoadingReady.value = true;
}

const showDiscordLinkModal = computed(
  () => postLoadingReady.value && !discordChecking.value && needsDiscordLink.value,
);

const showPremiumSignInModal = computed(
  () =>
    postLoadingReady.value &&
    !showDiscordLinkModal.value &&
    activePremiumAuth.value !== null,
);

const showAnnouncementModal = computed(
  () =>
    postLoadingReady.value &&
    !showDiscordLinkModal.value &&
    !showPremiumSignInModal.value &&
    activeAnnouncement.value !== null,
);

let desiredIgnoring = false;
let committedIgnoring = false;
let ignoreCommitTimer: ReturnType<typeof setTimeout> | null = null;

const overlayActive = ref(true);

const headerHovered = ref(false);
const dropdownOpen = ref(false);
const dropdownResetters = new Set<() => void>();

function registerDropdownResetter(fn: () => void): () => void {
  dropdownResetters.add(fn);
  return () => dropdownResetters.delete(fn);
}

function resetStuckDropdowns(): void {
  if (!dropdownOpen.value) return;
  dropdownOpen.value = false;
  for (const reset of dropdownResetters) reset();
}

provide('headerHovered', headerHovered);
provide('dropdownOpen', dropdownOpen);
provide('registerDropdownResetter', registerDropdownResetter);

const headerHeight = 42;
const resizeEdge = 6;

function flushStaleHover(): void {
  const root = document.documentElement;
  root.style.pointerEvents = 'none';
  void root.offsetHeight;
  root.style.pointerEvents = '';
}

function commitIgnore(next: boolean): void {
  if (ignoreCommitTimer !== null) {
    clearTimeout(ignoreCommitTimer);
    ignoreCommitTimer = null;
  }
  if (committedIgnoring === next) return;
  committedIgnoring = next;
  overlayActive.value = !next;
  window.api.win.setIgnoreMouse(next);
  if (next) {
    flushStaleHover();
    resetStuckDropdowns();
  }
}

function setIgnore(ignore: boolean): void {
  if (desiredIgnoring === ignore) return;

  desiredIgnoring = ignore;

  if (ignore) {
    commitIgnore(true);
    return;
  }

  if (ignoreCommitTimer !== null) clearTimeout(ignoreCommitTimer);
  ignoreCommitTimer = setTimeout(() => {
    ignoreCommitTimer = null;
    if (!desiredIgnoring) commitIgnore(false);
  }, 45);
}

let pendingWakeKey: string | null = null;
let pendingWakeSince = 0;

function clearPendingWake(): void {
  pendingWakeKey = null;
}

function requestWake(key: string, fromForwarded: boolean): void {
  if (!fromForwarded) {
    pendingWakeKey = null;
    setIgnore(false);
    return;
  }

  const now = performance.now();

  if (pendingWakeKey !== key) {
    pendingWakeKey = key;
    pendingWakeSince = now;
    return;
  }

  if (now - pendingWakeSince >= 90) {
    setIgnore(false);
  }
}

function describeTarget(target: HTMLElement | null): string {
  if (!target) return 'null';
  const id = target.id ? `#${target.id}` : '';
  const classes =
    typeof target.className === 'string' && target.className
      ? `.${target.className.trim().split(/\s+/).slice(0, 4).join('.')}`
      : '';
  return `${target.tagName.toLowerCase()}${id}${classes}`;
}

const resizeEdgeExit = resizeEdge + 6;
let stickyOnResizeEdge = false;

function isOnResizeEdge(pointerX: number, pointerY: number): boolean {
  const windowWidth = window.innerWidth;
  const windowHeight = window.innerHeight;
  const margin = stickyOnResizeEdge ? resizeEdgeExit : resizeEdge;

  const onEdge =
    pointerX <= margin ||
    pointerX >= windowWidth - margin ||
    pointerY <= margin ||
    pointerY >= windowHeight - margin;

  stickyOnResizeEdge = onEdge;
  return onEdge;
}

const selectors =
  'button, input, select, textarea, a, label, [role="button"], [tabindex="0"]';

function isScrollable(element: HTMLElement): boolean {
  let node: HTMLElement | null = element;

  while (node && node !== document.documentElement) {
    const style = window.getComputedStyle(node);

    const overflowY = style.overflowY;
    const overflowX = style.overflowX;

    if (
      ((overflowY === 'auto' || overflowY === 'scroll') &&
        node.scrollHeight > node.clientHeight) ||
      ((overflowX === 'auto' || overflowX === 'scroll') &&
        node.scrollWidth > node.clientWidth)
    ) {
      return true;
    }

    node = node.parentElement;
  }

  return false;
}

function updateResizeCursor(pointerX: number, pointerY: number): void {
  const windowWidth = window.innerWidth;
  const windowHeight = window.innerHeight;

  const onLeftEdge = pointerX <= resizeEdge;
  const onRightEdge = pointerX >= windowWidth - resizeEdge;
  const onTopEdge = pointerY <= resizeEdge;
  const onBottomEdge = pointerY >= windowHeight - resizeEdge;

  let cursor = '';

  if (onTopEdge && onLeftEdge) cursor = 'nw-resize';
  else if (onTopEdge && onRightEdge) cursor = 'ne-resize';
  else if (onBottomEdge && onLeftEdge) cursor = 'sw-resize';
  else if (onBottomEdge && onRightEdge) cursor = 'se-resize';
  else if (onTopEdge) cursor = 'n-resize';
  else if (onBottomEdge) cursor = 's-resize';
  else if (onLeftEdge) cursor = 'w-resize';
  else if (onRightEdge) cursor = 'e-resize';

  document.documentElement.style.cursor = cursor;
}

function evaluatePointer(
  pointerX: number,
  pointerY: number,
  fromForwarded = false,
): void {
  if (isOnResizeEdge(pointerX, pointerY)) {
    updateResizeCursor(pointerX, pointerY);
    requestWake('resize-edge', fromForwarded);
    return;
  }

  clearPendingWake();
  document.documentElement.style.cursor = '';

  headerHovered.value = pointerY <= headerHeight || dropdownOpen.value;

  if (pointerY <= headerHeight) {
    requestWake('header-band', fromForwarded);
    return;
  }

  const target = document.elementFromPoint(pointerX, pointerY) as HTMLElement | null;

  if (!target || target === document.documentElement || target === document.body) {
    if (dropdownOpen.value) {
      setIgnore(false);
      return;
    }

    setIgnore(true);
    return;
  }

  if (dropdownOpen.value) {
    setIgnore(false);
    return;
  }

  const matchedSelector = target.closest(selectors);
  const matchedNoDrag = target.closest('.no-drag');
  const matchedBtn = target.closest('.btn');
  const matchedCursorPointer = target.closest('.cursor-pointer');
  const matchedScrollable = isScrollable(target);

  const hit =
    matchedSelector ||
    matchedNoDrag ||
    matchedBtn ||
    matchedCursorPointer ||
    matchedScrollable;

  const matchedElement = hit
    ? ((matchedSelector ??
        matchedNoDrag ??
        matchedBtn ??
        matchedCursorPointer ??
        target) as HTMLElement)
    : null;

  if (hit) {
    requestWake(describeTarget(matchedElement), fromForwarded);
    return;
  }

  setIgnore(true);
}

function evaluatePointerLeft(): void {
  if (dropdownOpen.value) return;

  clearPendingWake();
  headerHovered.value = false;
  document.documentElement.style.cursor = '';
  setIgnore(true);
}

function processMouseMove(event: MouseEvent): void {
  evaluatePointer(event.clientX, event.clientY, false);
}

let pendingMouseEvent: MouseEvent | null = null;
let mouseMoveRaf = 0;

function onMouseMove(event: MouseEvent): void {
  pendingMouseEvent = event;

  if (mouseMoveRaf) return;

  mouseMoveRaf = requestAnimationFrame(() => {
    mouseMoveRaf = 0;
    if (pendingMouseEvent) processMouseMove(pendingMouseEvent);
    pendingMouseEvent = null;
  });
}

function cancelPendingMouseMove(): void {
  if (mouseMoveRaf) {
    cancelAnimationFrame(mouseMoveRaf);
    mouseMoveRaf = 0;
  }
  pendingMouseEvent = null;
}

function onMouseLeave(): void {
  cancelPendingMouseMove();
  evaluatePointerLeft();
}

function onMouseEnter(): void {
  setIgnore(false);
}

const ranks = [
  'rankOwner',
  'rankDeveloper',
  'rankManager',
  'rankAdmin',
  'rankSrmod',
  'rankModerator',
  'rankHelper',
  'rankTrial',
  'rankYoutuber',
  'rankChampion',
  'rankTitan',
  'rankElite',
  'rankVip',
] as const;

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

function hexToRgb(hex: string): [number, number, number] {
  const clean = hex.replace('#', '').slice(0, 6).padEnd(6, '0');

  return [
    parseInt(clean.slice(0, 2), 16),
    parseInt(clean.slice(2, 4), 16),
    parseInt(clean.slice(4, 6), 16),
  ];
}

function rgbToHex(red: number, green: number, blue: number): string {
  return `#${[red, green, blue]
    .map((channel) => clamp(Math.round(channel), 0, 255).toString(16).padStart(2, '0'))
    .join('')}`;
}

function rgbToHsl(red: number, green: number, blue: number): [number, number, number] {
  const r = red / 255;
  const g = green / 255;
  const b = blue / 255;

  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);

  let hue = 0;
  let saturation = 0;

  const lightness = (max + min) / 2;

  if (max !== min) {
    const delta = max - min;

    saturation = lightness > 0.5 ? delta / (2 - max - min) : delta / (max + min);

    switch (max) {
      case r:
        hue = (g - b) / delta + (g < b ? 6 : 0);
        break;

      case g:
        hue = (b - r) / delta + 2;
        break;

      default:
        hue = (r - g) / delta + 4;
        break;
    }

    hue /= 6;
  }

  return [
    Math.round(hue * 360),
    Math.round(saturation * 100),
    Math.round(lightness * 100),
  ];
}

function hexToHsl(hex: string): [number, number, number] {
  const [red, green, blue] = hexToRgb(hex);
  return rgbToHsl(red, green, blue);
}

function hslToHex(hue: number, saturationPct: number, lightnessPct: number): string {
  const saturation = saturationPct / 100;
  const lightness = lightnessPct / 100;

  const chroma = saturation * Math.min(lightness, 1 - lightness);

  const channelAt = (offset: number): number => {
    const k = (offset + hue / 30) % 12;

    return 255 * (lightness - chroma * Math.max(Math.min(k - 3, 9 - k, 1), -1));
  };

  return rgbToHex(channelAt(0), channelAt(8), channelAt(4));
}

function rgba(hex: string, alpha: number): string {
  const [red, green, blue] = hexToRgb(hex);

  return `rgba(${red},${green},${blue},${alpha})`;
}

function mix(hexA: string, hexB: string, amount: number): string {
  const [r1, g1, b1] = hexToRgb(hexA);
  const [r2, g2, b2] = hexToRgb(hexB);

  return rgbToHex(
    r1 + (r2 - r1) * amount,
    g1 + (g2 - g1) * amount,
    b1 + (b2 - b1) * amount,
  );
}

function getBackgroundBase(theme: import('@renderer/store/config').ThemeConfig): string {
  if (theme.bgType === 'gradient' && theme.bgGradientStops.length) {
    const sorted = [...theme.bgGradientStops].sort((a, b) => a.position - b.position);

    return sorted[0].color;
  }

  if (theme.bgType === 'image') {
    return '#0b0f19';
  }

  return theme.bgColor;
}

function deriveDynamicColors(
  theme: import('@renderer/store/config').ThemeConfig,
): ThemeColors {
  const bg = getBackgroundBase(theme);

  const [h, s, l] = hexToHsl(bg);

  const isDark = l < 45;
  const isVeryDark = l < 18;
  const isLight = l > 72;

  const accent = isLight
    ? hslToHex(h, Math.max(s, 58), 42)
    : hslToHex(h, Math.max(s, 62), clamp(l + 18, 52, 68));

  const accentLight = isLight
    ? hslToHex(h, Math.max(s - 6, 48), 56)
    : hslToHex(h, Math.max(s - 4, 54), clamp(l + 30, 66, 82));

  const ink1 = isDark
    ? hslToHex(h, Math.min(s, 12), 96)
    : hslToHex(h, Math.min(s, 18), 10);

  const ink2 = isDark
    ? hslToHex(h, Math.min(s, 10), 72)
    : hslToHex(h, Math.min(s, 14), 34);

  const ink3 = isDark
    ? hslToHex(h, Math.min(s, 8), 52)
    : hslToHex(h, Math.min(s, 10), 52);

  const border = isVeryDark
    ? 'rgba(255,255,255,0.08)'
    : isDark
      ? rgba(mix(bg, '#ffffff', 0.35), 0.18)
      : rgba(mix(bg, '#000000', 0.45), 0.14);

  const good = isDark ? '#4ade80' : '#16a34a';
  const bad = isDark ? '#fb7185' : '#dc2626';

  const nick = isDark
    ? hslToHex((h + 18) % 360, Math.max(s, 72), 76)
    : hslToHex((h + 18) % 360, Math.max(s, 72), 42);

  const rankOverrides = Object.fromEntries(
    ranks.map((key, i) => [key, hslToHex((h + i * 24) % 360, 78, isDark ? 68 : 46)]),
  ) as Pick<ThemeColors, (typeof ranks)[number]>;

  return {
    ...theme.colors,
    accent,
    accentLight,
    border,
    ink1,
    ink2,
    ink3,
    nick,
    good,
    bad,
    ...rankOverrides,
  };
}

function applyPanelBlur(): void {
  const disableBlur = config.lowEndMode || screenshotMode.value || !windowFocused.value;
  document.documentElement.style.setProperty(
    '--panel-blur',
    disableBlur ? 'none' : 'blur(24px)',
  );
}

function applyThemeVars(): void {
  const theme = config.theme;

  const colors: ThemeColors = theme.dynamicColors
    ? deriveDynamicColors(theme)
    : theme.colors;

  const root = document.documentElement.style;

  const bgBase = getBackgroundBase(theme);
  const [br, bg_, bb] = hexToRgb(bgBase);
  root.setProperty('--color-bg', bgBase);
  root.setProperty('--color-bg-rgb', `${br},${bg_},${bb}`);

  const panelAlpha = config.integratedMode
    ? 0
    : config.lowEndMode || screenshotMode.value
      ? Math.max(0.55, theme.opacity * 0.9)
      : Math.max(0.03, theme.opacity * 0.55);
  root.setProperty('--panel-bg', `rgba(${br},${bg_},${bb},${panelAlpha})`);
  applyPanelBlur();

  root.setProperty('--color-accent', colors.accent);
  root.setProperty('--color-accent-light', colors.accentLight);
  root.setProperty('--color-border', colors.border);

  root.setProperty('--color-ink-1', colors.ink1);
  root.setProperty('--color-ink-2', colors.ink2);
  root.setProperty('--color-ink-3', colors.ink3);

  root.setProperty('--color-nick', colors.nick);
  root.setProperty('--color-good', colors.good);
  root.setProperty('--color-bad', colors.bad);

  const [r, g, b] = hexToRgb(colors.accent);

  root.setProperty('--color-accent-rgb', `${r},${g},${b}`);
  root.setProperty('--color-accent-dim', `rgba(${r},${g},${b},0.12)`);
  root.setProperty('--color-accent-glow', `rgba(${r},${g},${b},0.42)`);

  root.setProperty('--shadow-glow', `0 0 18px rgba(${r},${g},${b},0.24)`);

  root.setProperty('--color-border-hover', `rgba(${r},${g},${b},0.28)`);
  root.setProperty('--color-border-active', `rgba(${r},${g},${b},0.42)`);

  root.setProperty('--color-rank-owner', colors.rankOwner);
  root.setProperty('--color-rank-developer', colors.rankDeveloper);
  root.setProperty('--color-rank-manager', colors.rankManager);
  root.setProperty('--color-rank-admin', colors.rankAdmin);
  root.setProperty('--color-rank-srmod', colors.rankSrmod);
  root.setProperty('--color-rank-moderator', colors.rankModerator);
  root.setProperty('--color-rank-helper', colors.rankHelper);
  root.setProperty('--color-rank-trial', colors.rankTrial);
  root.setProperty('--color-rank-youtuber', colors.rankYoutuber);
  root.setProperty('--color-rank-champion', colors.rankChampion);
  root.setProperty('--color-rank-titan', colors.rankTitan);
  root.setProperty('--color-rank-elite', colors.rankElite);
  root.setProperty('--color-rank-vip', colors.rankVip);
}

watch(() => config.theme, applyThemeVars, {
  deep: true,
  immediate: true,
});

watch(
  () => config.fontSize,
  (size) => {
    document.documentElement.style.fontSize = `${size}px`;
  },
  { immediate: true },
);

function clearStalePlayerStorage(): void {
  try {
    localStorage.removeItem('players');
  } catch {}
}

let removeLogListener: (() => void) | null = null;
let removeForwardedMoveListener: (() => void) | null = null;

async function initLogWatcher(): Promise<void> {
  if (!config.logFilePath) {
    await config.setLogFilePathFromPreset(config.logFilePathPreset);
  }

  const valid = await window.api.log.checkPath(config.logFilePath);

  players.logPathValid = valid;

  if (valid) {
    window.api.log.setPath(config.logFilePath);

    if (router.currentRoute.value.name === 'Setup') {
      router.replace('/');
    }
  }
}

function attachLogListener(): void {
  removeLogListener?.();

  removeLogListener = window.api.log.onLine((line) => {
    rpcHeartbeat();

    const event = parseLine(line);

    if (!event) return;

    switch (event.type) {
      case 'join':
        if (config.autoAddPlayers) {
          players.setCount((players.playersCount ?? 0) + 1);
          players.addByName(event.name, 'auto');
        }
        break;

      case 'quit':
        if (config.autoRemoveOnQuit) {
          players.removeByName(nicks.resolve(event.name));
          players.decrementCount();
        }
        break;

      case 'who': {
        const trackedNames = new Set(
          players.players.map((player) => player.realName.toLowerCase()),
        );

        const newNames = event.names.filter(
          (name) => !trackedNames.has(nicks.resolve(name).toLowerCase()),
        );

        if (config.autoRemoveAllOnWho) {
          players.clear();
        }

        players.setCount(event.names.length);

        const toAdd = config.autoRemoveAllOnWho ? event.names : newNames;

        for (const n of toAdd) {
          players.addByName(n, 'auto');
        }

        break;
      }

      case 'finalKill':
        if (config.autoRemoveFinalDeath) {
          players.removeByName(nicks.resolve(event.name));
          players.decrementCount();
        }
        break;
    }
  });
}

let removeShortcutListener: (() => void) | null = null;

let removeProxyListener: (() => void) | null = null;

let removeTelemetryListener: (() => void) | null = null;

function attachTelemetryListener(): void {
  removeTelemetryListener?.();

  removeTelemetryListener = window.api.telemetry.onEvent((event) => {
    handleTelemetryEvent(event);
  });
}

function attachProxyListener(): void {
  removeProxyListener?.();

  removeProxyListener = window.api.proxy.onEvent((raw) => {
    const event = raw as ProxyEventPayload;

    if (
      event.type === 'auth-code' ||
      event.type === 'auth-success' ||
      event.type === 'auth-error'
    ) {
      handleProxyAuthEvent(event);

      return;
    }

    if (event.type === 'client-connect') {
      players.setProxyConnectedNetwork(event.network);

      if (config.autoDetectNetwork && event.network !== config.network) {
        config.network = event.network as typeof config.network;
        window.api.rpc.setNetwork(event.network);
      }

      players.clear();

      return;
    }

    if (event.type === 'client-disconnect') {
      if (players.proxyConnectedNetwork === event.network) {
        players.setProxyConnectedNetwork(null);
      }

      players.clearTeams();

      return;
    }

    if (event.network !== config.network) return;

    switch (event.type) {
      case 'player-join':
        if (config.autoAddPlayers) {
          players.addByName(event.username, 'auto');
        }
        break;

      case 'player-quit':
        if (config.autoRemoveOnQuit) {
          players.removeByName(nicks.resolve(event.username));
        }
        break;

      case 'teams-update':
        players.applyTeams(event.teams);
        break;
    }
  });
}

async function registerShortcuts(): Promise<void> {
  await window.api.shortcuts.register([
    config.shortcutMinimize,
    config.shortcutClearPlayers,
  ]);

  removeShortcutListener?.();

  removeShortcutListener = window.api.shortcuts.onFired((shortcut) => {
    if (shortcut === config.shortcutMinimize) {
      window.api.win.toggleMinimize();
    }

    if (shortcut === config.shortcutClearPlayers) {
      players.clear();
    }
  });
}

let rpcIdleTimer: ReturnType<typeof setTimeout> | null = null;

function rpcSetActive(active: boolean): void {
  window.api.rpc.setActive(active);
}

function rpcHeartbeat(): void {
  rpcSetActive(true);

  if (rpcIdleTimer) {
    clearTimeout(rpcIdleTimer);
  }

  rpcIdleTimer = setTimeout(() => {
    rpcSetActive(false);
    rpcIdleTimer = null;
  }, 15_000);
}

function updateRpcEnabled(): void {
  const enabled = config.discordRpcEnabled && discordChecked.value && discordLinked.value;
  window.api.rpc.setEnabled(enabled);

  if (!enabled && rpcIdleTimer) {
    clearTimeout(rpcIdleTimer);
    rpcIdleTimer = null;
  }
}

watch(
  () => config.discordRpcEnabled,
  () => {
    updateRpcEnabled();
  },
);

watch(
  () => needsDiscordLink.value,
  () => {
    updateRpcEnabled();
  },
);

watch(
  () => config.network,
  (network) => {
    window.api.rpc.setNetwork(network);
    players.clear();
  },
);

watch(
  () => [config.interval, config.mode] as const,
  ([interval, mode]) => {
    players.refreshAllStats(interval, mode);
  },
);

watch(
  () => config.logFilePath,
  async () => {
    window.api.log.setPath(null);
    await initLogWatcher();
  },
);

watch(
  () => [config.shortcutMinimize, config.shortcutClearPlayers] as const,
  () => registerShortcuts(),
);

onMounted(async () => {
  clearStalePlayerStorage();

  players.clear();

  attachLogListener();
  attachProxyListener();
  attachTelemetryListener();

  if (skipDiscordLink) {
    bypassDiscordLink();
  } else {
    await refreshDiscordLinkStatus();
  }

  updateRpcEnabled();

  await initLogWatcher();
  await registerShortcuts();

  window.addEventListener('mousemove', onMouseMove);
  window.addEventListener('mouseleave', onMouseLeave);
  window.addEventListener('mouseenter', onMouseEnter);
  document.addEventListener('visibilitychange', handleVisibilityChange);
  window.addEventListener('focus', handleWindowFocus);
  window.addEventListener('blur', handleWindowBlur);
  updateAnimPlayState();

  setIgnore(true);

  removeForwardedMoveListener = window.api.win.onForwardedMove((x, y) => {
    if (x === null || y === null) {
      evaluatePointerLeft();
      return;
    }

    evaluatePointer(x, y, true);
  });

  updateRpcEnabled();
  window.api.rpc.setNetwork(config.network);

  if (config.autoUpdateEnabled) {
    window.api.updater.check();
  }

  if (skipLoading && !skipAnnouncements) {
    void fetchAnnouncements();
  }

  window.api.proxy.setPort('pikanetwork', config.pikaProxyPort).catch(() => {});
  window.api.proxy.setPort('jartexnetwork', config.jartexProxyPort).catch(() => {});
  window.api.proxy.setBindHost(config.proxyBindHost).catch(() => {});
});

onUnmounted(() => {
  removeLogListener?.();
  removeShortcutListener?.();
  removeForwardedMoveListener?.();
  removeProxyListener?.();
  removeTelemetryListener?.();

  if (ignoreCommitTimer) {
    clearTimeout(ignoreCommitTimer);
    ignoreCommitTimer = null;
  }

  if (rpcIdleTimer) {
    clearTimeout(rpcIdleTimer);
    rpcIdleTimer = null;
  }

  window.removeEventListener('mousemove', onMouseMove);
  window.removeEventListener('mouseleave', onMouseLeave);
  window.removeEventListener('mouseenter', onMouseEnter);
  document.removeEventListener('visibilitychange', handleVisibilityChange);
  window.removeEventListener('focus', handleWindowFocus);
  window.removeEventListener('blur', handleWindowBlur);

  cancelPendingMouseMove();

  window.api.rpc.destroy();
});
</script>

<template>
  <div
    class="overlay-root relative flex h-screen w-screen flex-col overflow-hidden"
    :class="{ 'rounded-lg': config.roundedCorners }"
  >
    <div
      v-if="
        config.theme.bgType === 'image' &&
        config.theme.bgImageUrl &&
        !config.integratedMode
      "
      class="pointer-events-none absolute inset-0"
      :style="{
        backgroundImage: `url('${config.theme.bgImageUrl}')`,
        backgroundSize: 'cover',
        backgroundPosition: 'center',
        opacity: config.theme.bgImageOpacity,
        borderRadius: 'inherit',
      }"
    />

    <div
      v-if="config.theme.bgType !== 'image'"
      class="pointer-events-none absolute inset-0"
      :style="{
        background: config.bgColor,
        opacity: config.integratedMode ? 0 : config.theme.opacity,
        borderRadius: 'inherit',
      }"
    />

    <template v-if="!config.integratedMode && config.theme.bgType === 'gradient'">
      <div class="ambient-layer ambient-tl" />
      <div class="ambient-layer ambient-br" />
      <div class="ambient-layer ambient-center" />
    </template>
    <div
      v-if="!config.integratedMode && config.theme.bgType !== 'image'"
      class="overlay-frost noise-veil"
    />
    <div
      v-if="!config.integratedMode && config.theme.bgType !== 'image'"
      class="overlay-life-pulse"
    />
    <div class="overlay-glass-border" />

    <div
      class="relative flex flex-1 flex-col overflow-hidden"
      :style="{ zIndex: 1 }"
    >
      <div class="titlebar-layer relative shrink-0">
        <TitleBar />
      </div>

      <Transition name="banner-drop">
        <QueueSafetyBanner
          v-if="queueSafetyVisible"
          :verdict="queueSafetyVerdict"
          @dismiss="dismissQueueSafety"
        />
      </Transition>

      <div
        class="relative flex-1 overflow-hidden"
        :style="{
          opacity: appVisible ? 1 : 0,
          transition: 'opacity 0.35s ease-in-out',
        }"
      >
        <router-view
          v-slot="{ Component, route: r }"
          class="flex-1 overflow-hidden"
        >
          <Transition
            name="fade"
            mode="out-in"
          >
            <div
              :key="r.fullPath"
              class="h-full w-full"
            >
              <component :is="Component" />
            </div>
          </Transition>
        </router-view>
      </div>
    </div>

    <Transition
      name="loading-fade"
      @after-leave="onLoadingScreenGone"
    >
      <LoadingScreen
        v-if="!loadingDone"
        @done="onLoadingDone"
      />
    </Transition>

    <template v-if="postLoadingReady">
      <DiscordLinkModal
        v-if="showDiscordLinkModal"
        :linking="discordLinking"
        :error-message="discordLinkError"
        @link="startDiscordLink()"
      />

      <PremiumSignInModal
        v-else-if="showPremiumSignInModal"
        :key="activePremiumAuth!.network"
        :auth="activePremiumAuth!"
        @close="dismissPremiumAuth()"
      />

      <AnnouncementModal
        v-else-if="showAnnouncementModal"
        :key="activeAnnouncement!.payload.id"
        :mode="activeAnnouncement!.mode"
        :changelog="
          activeAnnouncement!.mode === 'changelog'
            ? activeAnnouncement!.payload
            : undefined
        "
        :alert="
          activeAnnouncement!.mode === 'alert' ? activeAnnouncement!.payload : undefined
        "
        @close="dismissActive()"
      />
    </template>

    <PerfOverlay v-if="config.perfLoggingEnabled" />
  </div>
</template>

<style scoped>
.overlay-root {
  position: relative;
  z-index: 11000;
  contain: paint;
}

.overlay-frost {
  pointer-events: none;
  position: absolute;
  inset: 0;
  z-index: 0;
  border-radius: inherit;
  backdrop-filter: blur(7px) saturate(135%);
  -webkit-backdrop-filter: blur(7px) saturate(135%);
}

html.low-end .overlay-frost {
  backdrop-filter: none !important;
  -webkit-backdrop-filter: none !important;
}

.overlay-life-pulse {
  pointer-events: none;
  position: absolute;
  inset: -10% -10% auto -10%;
  z-index: 0;
  height: 55%;
  background: radial-gradient(
    ellipse at 50% 0%,
    rgba(var(--color-accent-rgb), 0.018) 0%,
    transparent 65%
  );
  animation: overlay-life-breathe 7s ease-in-out infinite;
}

@keyframes overlay-life-breathe {
  0%,
  100% {
    opacity: 0.5;
  }
  50% {
    opacity: 0.7;
  }
}

html.low-end .overlay-life-pulse {
  animation: none !important;
  opacity: 0.6;
}

@media (prefers-reduced-motion: reduce) {
  .overlay-life-pulse {
    animation: none !important;
    opacity: 0.6;
  }
}

.overlay-glass-border {
  pointer-events: none;
  position: absolute;
  inset: 0;
  z-index: 20;
  border-radius: inherit;
  padding: 1px;
  background: linear-gradient(
    140deg,
    rgba(255, 255, 255, 0.14) 0%,
    rgba(255, 255, 255, 0) 26%,
    rgba(255, 255, 255, 0) 74%,
    rgba(var(--color-accent-rgb), 0.16) 100%
  );
  -webkit-mask:
    linear-gradient(#000 0 0) content-box,
    linear-gradient(#000 0 0);
  mask:
    linear-gradient(#000 0 0) content-box,
    linear-gradient(#000 0 0);
  -webkit-mask-composite: xor;
  mask-composite: exclude;
}

html.low-end .overlay-glass-border {
  display: none !important;
}

.titlebar-layer {
  position: relative;
  z-index: 11100;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.12s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.loading-fade-leave-active {
  transition: opacity 0.35s ease-in-out;
  pointer-events: none;
}

.loading-fade-leave-to {
  opacity: 0;
}
</style>
