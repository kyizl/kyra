import type { BedwarsMode, Interval, LogFilePreset, Network } from '@renderer/types';
import type { QueueSafetyConfig } from '@renderer/types/queue-safety';
import type { PiniaPluginContext } from 'pinia';
import { defaultPaletteId, getPalette } from '@renderer/palettes';
import { Column } from '@renderer/types';
import {
  createExcludedPlayer,
  createRule,
  defaultQueueSafetyConfig,
} from '@renderer/types/queue-safety';
import { defineStore } from 'pinia';
import 'pinia-plugin-persistedstate';

export interface GradientStop {
  color: string;
  position: number;
}
export type BgType = 'solid' | 'gradient' | 'image';
export type GradientDirection =
  | 'to right'
  | 'to left'
  | 'to bottom'
  | 'to top'
  | 'to bottom right'
  | 'to bottom left'
  | 'to top right'
  | 'to top left'
  | string;

export interface ThemeColors {
  accent: string;
  accentLight: string;
  border: string;
  ink1: string;
  ink2: string;
  ink3: string;
  nick: string;
  good: string;
  bad: string;
  rankOwner: string;
  rankDeveloper: string;
  rankManager: string;
  rankAdmin: string;
  rankSrmod: string;
  rankModerator: string;
  rankHelper: string;
  rankTrial: string;
  rankYoutuber: string;
  rankChampion: string;
  rankTitan: string;
  rankElite: string;
  rankVip: string;
}

export interface ThemeConfig {
  bgType: BgType;
  bgColor: string;
  bgGradientStops: GradientStop[];
  bgGradientDir: GradientDirection;
  bgImageUrl: string;
  bgImageOpacity: number;
  opacity: number;
  colors: ThemeColors;
  dynamicColors: boolean;
}

const defaultPalette = getPalette(defaultPaletteId)!;

export const DefaultThemeColors: ThemeColors = { ...defaultPalette.colors };

export const DefaultTheme: ThemeConfig = {
  bgType: 'solid',
  bgColor: defaultPalette.bg,
  bgGradientStops: [
    { color: defaultPalette.colors.accent, position: 0 },
    { color: defaultPalette.bg, position: 100 },
  ],
  bgGradientDir: 'to bottom right',
  bgImageUrl: '',
  bgImageOpacity: 0.3,
  opacity: 0.92,
  colors: { ...defaultPalette.colors },
  dynamicColors: false,
};

export interface ConfigState {
  network: Network;
  logFilePath: string;
  logFilePathPreset: LogFilePreset;
  fontSize: number;
  opacity: number;
  roundedCorners: boolean;
  alwaysOnTop: boolean;
  activeColumns: Column[];
  sortBy: Column;
  sortAscending: boolean;
  interval: Interval;
  mode: BedwarsMode;
  columnLabels: 'FULL' | 'SHORT' | 'HIDDEN';
  textShadow: boolean;
  integratedMode: boolean;
  autoAddPlayers: boolean;
  autoRemoveAllOnServerChange: boolean;
  autoRemoveAllOnWho: boolean;
  autoRemoveFinalDeath: boolean;
  autoRemoveOnQuit: boolean;
  missingPlayersWarning: boolean;
  shortcutMinimize: string;
  shortcutClearPlayers: string;
  discordRpcEnabled: boolean;
  autoUpdateEnabled: boolean;
  theme: ThemeConfig;
  paletteId: string;
  lowEndMode: boolean;
  pikaProxyPort: number;
  jartexProxyPort: number;
  autoDetectNetwork: boolean;
  proxyBindHost: '0.0.0.0' | '127.0.0.1';
  proxyBannerDismissed: boolean;
  perfLoggingEnabled: boolean;
  queueSafety: QueueSafetyConfig;
}

const DefaultColumns: Column[] = [
  Column.NAME,
  Column.LEVEL,
  Column.WINS,
  Column.KILLS,
  Column.FINAL_KILLS,
  Column.FKDR,
  Column.WLR,
  Column.WIN_STREAK,
];

const configVersion = 9;

interface ConfigMigration {
  keys: (keyof ConfigState)[];
}

const migrations: Record<number, ConfigMigration> = {
  2: { keys: ['theme', 'fontSize', 'opacity', 'roundedCorners', 'textShadow'] },
  3: { keys: ['paletteId'] },
  4: { keys: ['lowEndMode'] },
  5: { keys: ['perfLoggingEnabled'] },
  6: { keys: ['queueSafety'] },
  7: { keys: ['queueSafety'] },
  8: { keys: ['queueSafety'] },
  9: { keys: ['alwaysOnTop'] },
};

const ValidColumns = new Set<string>(Object.values(Column));

type PresetPathKey = Exclude<LogFilePreset, 'CUSTOM' | 'LUNAR_CLIENT'>;

function buildPresetPaths(
  platform: string,
  appData: string,
  home: string,
): Record<PresetPathKey, string> {
  if (platform === 'win32') {
    return {
      STANDARD: `${appData}/.minecraft/logs/latest.log`,
      TLAUNCHER: `${appData}/.minecraft/logs/latest.log`,
      BADLION_CLIENT: `${appData}/.minecraft/logs/blclient/minecraft/latest.log`,
      SILENT_CLIENT: `${appData}/.mc/logs/latest.log`,
      FEATHER_CLIENT: `${appData}/.minecraft/feather/logs/latest.log`,
      CM_CLIENT: `${appData}/.cmclient/logs/latest.log`,
      SK_CLIENT: `${appData}/.minecraft/logs/latest.log`,
      SALWYRR: `${appData}/.salwyrr/logs/latest.log`,
      PVPLOUNGE: `${appData}/.pvplounge/logs/latest.log`,
    };
  }

  if (platform === 'darwin') {
    return {
      STANDARD: `${appData}/minecraft/logs/latest.log`,
      TLAUNCHER: `${appData}/minecraft/logs/latest.log`,
      BADLION_CLIENT: `${appData}/minecraft/logs/blclient/minecraft/latest.log`,
      SILENT_CLIENT: `${home}/.mc/logs/latest.log`,
      FEATHER_CLIENT: `${appData}/minecraft/feather/logs/latest.log`,
      CM_CLIENT: `${home}/.cmclient/logs/latest.log`,
      SK_CLIENT: `${appData}/minecraft/logs/latest.log`,
      SALWYRR: `${home}/.salwyrr/logs/latest.log`,
      PVPLOUNGE: `${appData}/pvplounge/logs/latest.log`,
    };
  }

  return {
    STANDARD: `${home}/.minecraft/logs/latest.log`,
    TLAUNCHER: `${home}/.minecraft/logs/latest.log`,
    BADLION_CLIENT: `${home}/.minecraft/logs/blclient/minecraft/latest.log`,
    SILENT_CLIENT: `${home}/.mc/logs/latest.log`,
    FEATHER_CLIENT: `${home}/.minecraft/feather/logs/latest.log`,
    CM_CLIENT: `${home}/.cmclient/logs/latest.log`,
    SK_CLIENT: `${home}/.minecraft/logs/latest.log`,
    SALWYRR: `${home}/.salwyrr/logs/latest.log`,
    PVPLOUNGE: `${home}/.pvplounge/logs/latest.log`,
  };
}

function makeDefaultState(): ConfigState {
  return {
    network: 'pikanetwork',
    logFilePath: '',
    logFilePathPreset: 'STANDARD',
    fontSize: 15,
    opacity: 0.92,
    roundedCorners: true,
    alwaysOnTop: false,
    activeColumns: DefaultColumns,
    sortBy: Column.NAME,
    sortAscending: false,
    interval: 'total',
    mode: 'ALL_MODES',
    columnLabels: 'FULL',
    textShadow: false,
    integratedMode: false,
    autoAddPlayers: true,
    autoRemoveAllOnServerChange: false,
    autoRemoveAllOnWho: true,
    autoRemoveFinalDeath: true,
    autoRemoveOnQuit: true,
    missingPlayersWarning: true,
    shortcutMinimize: '',
    shortcutClearPlayers: '',
    discordRpcEnabled: false,
    autoUpdateEnabled: true,
    theme: { ...DefaultTheme, colors: { ...DefaultThemeColors } },
    paletteId: defaultPaletteId,
    lowEndMode: false,
    pikaProxyPort: 25566,
    jartexProxyPort: 25567,
    autoDetectNetwork: true,
    proxyBindHost: '127.0.0.1',
    proxyBannerDismissed: false,
    perfLoggingEnabled: false,
    queueSafety: defaultQueueSafetyConfig(),
  };
}

export const useConfigStore = defineStore('config', {
  state: makeDefaultState,

  getters: {
    bgColor(state): string {
      const theme = state.theme;
      if (theme.bgType === 'gradient') {
        const stops = theme.bgGradientStops
          .slice()
          .sort((stopA, stopB) => stopA.position - stopB.position)
          .map((stop) => `${stop.color} ${stop.position}%`)
          .join(', ');
        return `linear-gradient(${theme.bgGradientDir}, ${stops})`;
      }
      if (theme.bgType === 'image') {
        return 'rgb(6,9,20)';
      }
      const hex = theme.bgColor.replace('#', '').slice(0, 6).padEnd(6, '0');
      const red = Number.parseInt(hex.slice(0, 2), 16);
      const green = Number.parseInt(hex.slice(2, 4), 16);
      const blue = Number.parseInt(hex.slice(4, 6), 16);
      return `rgb(${red},${green},${blue})`;
    },
  },

  actions: {
    applyPalette(id: string): void {
      const palette = getPalette(id);
      if (!palette) return;
      this.paletteId = id;
      this.theme.bgColor = palette.bg;
      this.theme.bgType = 'solid';
      this.theme.colors = { ...palette.colors };
      this.theme.dynamicColors = false;
    },

    resetTheme(): void {
      this.theme = { ...DefaultTheme, colors: { ...DefaultThemeColors } };
      this.paletteId = defaultPaletteId;
    },

    async findLunarLogPath(): Promise<string> {
      return window.api.app.findLunarLog();
    },

    async setLogFilePathFromPreset(preset: LogFilePreset): Promise<void> {
      this.logFilePathPreset = preset;
      if (preset === 'CUSTOM') return;
      if (preset === 'LUNAR_CLIENT') {
        this.logFilePath = await this.findLunarLogPath();
        return;
      }

      const platform = window.api.platform;
      const [appData, home] = await Promise.all([
        window.api.app.getPath('appData'),
        window.api.app.getPath('home'),
      ]);

      const paths = buildPresetPaths(platform, appData, home);
      this.logFilePath = paths[preset];
    },
  },

  persist: {
    key: 'kyra-config',
    storage: localStorage,
    afterHydrate: (context: PiniaPluginContext) => {
      const store = context.store as unknown as Partial<ConfigState>;
      const storedVersion = parseInt(
        localStorage.getItem('kyra-config-version') ?? '0',
        10,
      );

      if (storedVersion < configVersion) {
        const defaults = makeDefaultState();
        for (let version = storedVersion + 1; version <= configVersion; version++) {
          const migration = migrations[version];
          if (migration !== undefined) {
            for (const key of migration.keys) {
              (store as unknown as Record<string, unknown>)[key] = (
                defaults as unknown as Record<string, unknown>
              )[key];
            }
          }
        }
        localStorage.setItem('kyra-config-version', String(configVersion));
      }

      const columns = store.activeColumns;
      const validColumns = Array.isArray(columns)
        ? (columns as string[]).filter((column) => ValidColumns.has(column))
        : [];
      store.activeColumns = (
        validColumns.length > 0 ? validColumns : DefaultColumns
      ) as Column[];

      if (store.theme?.colors === undefined) {
        store.theme = { ...DefaultTheme, colors: { ...DefaultThemeColors } };
      } else {
        store.theme.colors = { ...DefaultThemeColors, ...store.theme.colors };
      }

      if (store.theme.dynamicColors === undefined) {
        store.theme.dynamicColors = false;
      }

      if (store.paletteId === undefined || store.paletteId === '') {
        store.paletteId = defaultPaletteId;
      }

      if (store.network === undefined) {
        store.network = 'pikanetwork';
      }

      if (store.pikaProxyPort === undefined || store.pikaProxyPort === 0) {
        store.pikaProxyPort = 25566;
      }
      if (store.jartexProxyPort === undefined || store.jartexProxyPort === 0) {
        store.jartexProxyPort = 25567;
      }
      if (store.autoDetectNetwork === undefined) store.autoDetectNetwork = true;
      if (store.proxyBindHost === undefined) store.proxyBindHost = '127.0.0.1';
      if (store.proxyBannerDismissed === undefined) {
        store.proxyBannerDismissed = false;
      }

      if (store.lowEndMode === undefined) store.lowEndMode = false;
      if (store.perfLoggingEnabled === undefined) store.perfLoggingEnabled = false;

      if (!store.queueSafety) {
        store.queueSafety = defaultQueueSafetyConfig();
      } else {
        const defaults = defaultQueueSafetyConfig();
        const rules = Array.isArray(store.queueSafety.rules)
          ? store.queueSafety.rules
          : [];
        const excludedPlayers = Array.isArray(store.queueSafety.excludedPlayers)
          ? store.queueSafety.excludedPlayers
          : [];
        store.queueSafety = {
          ...defaults,
          ...store.queueSafety,
          rules: rules.map((rule) => createRule(rule)),
          excludedPlayers: excludedPlayers.map((excludedPlayer) =>
            createExcludedPlayer(excludedPlayer),
          ),
          matchMode:
            store.queueSafety.matchMode === 'any' || store.queueSafety.matchMode === 'all'
              ? store.queueSafety.matchMode
              : defaults.matchMode,
        };
      }
    },
  },
});
