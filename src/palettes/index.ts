import type { ThemeColors } from '@renderer/store/config';

export interface Palette {
  id: string;
  name: string;
  bg: string;
  swatches: readonly string[];
  colors: ThemeColors;
}

const rankColors = {
  rankOwner: '#BC4141',
  rankDeveloper: '#FF5555',
  rankManager: '#AA0000',
  rankAdmin: '#FF5555',
  rankSrmod: '#00AAAA',
  rankModerator: '#00AA00',
  rankHelper: '#5555FF',
  rankTrial: '#55FFFF',
  rankYoutuber: '#FF5555',
  rankChampion: '#FF5555',
  rankTitan: '#FFD700',
  rankElite: '#55FFFF',
  rankVip: '#55FF55',
} as const;

export const palettes: readonly Palette[] = [
  {
    id: 'ember',
    name: 'Ember',
    bg: '#130508',
    swatches: ['#130508', '#ff2d55', '#ff5c7c', '#ffe9ef'],
    colors: {
      accent: '#ff2d55',
      accentLight: '#ff5c7c',
      border: '#3a1523',
      ink1: '#ffe9ef',
      ink2: '#ff4d73',
      ink3: '#b87586',
      nick: '#fde68a',
      good: '#34d399',
      bad: '#f87171',
      ...rankColors,
    },
  },
  {
    id: 'nebula',
    name: 'Nebula',
    bg: '#04060f',
    swatches: ['#04060f', '#7c3aed', '#b89aff', '#ede9ff'],
    colors: {
      accent: '#7c3aed',
      accentLight: '#b89aff',
      border: 'rgba(120,80,255,0.16)',
      ink1: '#ede9ff',
      ink2: '#9d93c4',
      ink3: '#4a4270',
      nick: '#fde68a',
      good: '#34d399',
      bad: '#f87171',
      ...rankColors,
    },
  },
  {
    id: 'void',
    name: 'Void',
    bg: '#020408',
    swatches: ['#020408', '#06b6d4', '#67e8f9', '#e0fffe'],
    colors: {
      accent: '#06b6d4',
      accentLight: '#67e8f9',
      border: 'rgba(6,182,212,0.15)',
      ink1: '#e0fffe',
      ink2: '#4e9aaa',
      ink3: '#1e4f58',
      nick: '#a5f3fc',
      good: '#34d399',
      bad: '#f87171',
      ...rankColors,
    },
  },
  {
    id: 'aurora',
    name: 'Aurora',
    bg: '#03080e',
    swatches: ['#03080e', '#10b981', '#6ee7b7', '#ecfdf5'],
    colors: {
      accent: '#10b981',
      accentLight: '#6ee7b7',
      border: 'rgba(16,185,129,0.15)',
      ink1: '#ecfdf5',
      ink2: '#4a9070',
      ink3: '#164e35',
      nick: '#a7f3d0',
      good: '#34d399',
      bad: '#f87171',
      ...rankColors,
    },
  },
  {
    id: 'solar',
    name: 'Solar',
    bg: '#0c0800',
    swatches: ['#0c0800', '#f59e0b', '#fcd34d', '#fffbeb'],
    colors: {
      accent: '#f59e0b',
      accentLight: '#fcd34d',
      border: 'rgba(245,158,11,0.16)',
      ink1: '#fffbeb',
      ink2: '#a07830',
      ink3: '#5a4010',
      nick: '#fde68a',
      good: '#34d399',
      bad: '#f87171',
      ...rankColors,
    },
  },
] as const;

const registry = new Map(palettes.map((palette) => [palette.id, palette]));

export function getPalette(id: string): Palette | undefined {
  return registry.get(id);
}

export function getAllPalettes(): readonly Palette[] {
  return palettes;
}

export const defaultPaletteId = 'ember';
