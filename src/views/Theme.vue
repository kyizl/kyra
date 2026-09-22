<script setup lang="ts">
import SideTabNav from '@renderer/components/SideTabNav.vue';
import { Button } from '@renderer/components/ui/button';
import { Slider } from '@renderer/components/ui/slider';
import { Switch } from '@renderer/components/ui/switch';
import { useStaggerReveal } from '@renderer/composables/useStaggerReveal';
import { getAllPalettes } from '@renderer/palettes';
import { useConfigStore } from '@renderer/store/config';
import {
  Blend,
  Image,
  Layers,
  Palette,
  RotateCcw,
  Sparkles,
  Square,
} from 'lucide-vue-next';
import { SliderRange, SliderRoot, SliderThumb, SliderTrack } from 'radix-vue';
import { computed, defineComponent, h, ref } from 'vue';

const config = useConfigStore();
const theme = computed(() => config.theme);
const activeTab = ref<'palette' | 'background' | 'colors'>('palette');

const tabs = [
  { id: 'palette', label: 'Palette', icon: Layers },
  { id: 'background', label: 'Background', icon: Blend },
  { id: 'colors', label: 'Colors', icon: Palette },
];

const bgTypes = [
  { id: 'solid', label: 'Solid', icon: Square },
  { id: 'gradient', label: 'Gradient', icon: Blend },
  { id: 'image', label: 'Image', icon: Image },
];

const directions = [
  { label: '→', value: 'to right' },
  { label: '←', value: 'to left' },
  { label: '↓', value: 'to bottom' },
  { label: '↑', value: 'to top' },
  { label: '↘', value: 'to bottom right' },
  { label: '↙', value: 'to bottom left' },
  { label: '↗', value: 'to top right' },
  { label: '↖', value: 'to top left' },
];

const derivedColors = [
  { key: 'accentLight', label: 'Accent Light' },
  { key: 'border', label: 'Border' },
  { key: 'ink1', label: 'Text Primary' },
  { key: 'ink2', label: 'Text Secondary' },
  { key: 'ink3', label: 'Text Muted' },
  { key: 'nick', label: 'Nick Color' },
  { key: 'good', label: 'Good Stat' },
  { key: 'bad', label: 'Bad Stat' },
];

const rankColors = [
  { key: 'rankOwner', label: 'Owner' },
  { key: 'rankDeveloper', label: 'Developer' },
  { key: 'rankManager', label: 'Manager' },
  { key: 'rankAdmin', label: 'Admin' },
  { key: 'rankSrmod', label: 'Sr. Mod' },
  { key: 'rankModerator', label: 'Moderator' },
  { key: 'rankHelper', label: 'Helper' },
  { key: 'rankTrial', label: 'Trial' },
  { key: 'rankYoutuber', label: 'YouTuber' },
  { key: 'rankChampion', label: 'Champion' },
  { key: 'rankTitan', label: 'Titan' },
  { key: 'rankElite', label: 'Elite' },
  { key: 'rankVip', label: 'VIP' },
];

const palettes = getAllPalettes();

function selectPalette(id: string): void {
  config.applyPalette(id);
}

function toggleDynamic(): void {
  theme.value.dynamicColors = !theme.value.dynamicColors;
}

const solidHex = computed(() => {
  const hex = theme.value.bgColor.replace('#', '').slice(0, 6).padEnd(6, '0');
  return `#${hex}`;
});

function onSolidColor(event: Event): void {
  theme.value.bgColor = (event.target as HTMLInputElement).value;
}
function onSolidHexInput(rawHex: string): void {
  const normalized = rawHex.startsWith('#') ? rawHex : `#${rawHex}`;
  if (/^#[0-9a-f]{6}$/i.test(normalized)) theme.value.bgColor = normalized;
}

const gradientPreview = computed(() => {
  const stops = theme.value.bgGradientStops
    .slice()
    .sort((stopA, stopB) => stopA.position - stopB.position)
    .map((stop) => `${stop.color} ${stop.position}%`)
    .join(', ');
  return `linear-gradient(${theme.value.bgGradientDir}, ${stops})`;
});

function addStop(): void {
  const stops = theme.value.bgGradientStops;
  const last = stops[stops.length - 1]?.position ?? 50;
  stops.push({ color: '#ffffff', position: Math.min(100, Math.round((last + 100) / 2)) });
}
function removeStop(index: number): void {
  theme.value.bgGradientStops.splice(index, 1);
}

const previewBg = computed(() => {
  const currentTheme = theme.value;
  if (currentTheme.bgType === 'gradient') return gradientPreview.value;
  if (currentTheme.bgType === 'image') return 'rgb(6,9,20)';
  return solidHex.value;
});

const imageFilename = computed(() => {
  const url = theme.value.bgImageUrl;
  if (!url) return '';
  if (url.startsWith('data:')) return 'custom image (embedded)';
  const parts = url.replace(/\\/g, '/').split('/');
  return parts[parts.length - 1] || url;
});

async function pickLocalImage(): Promise<void> {
  const result = await window.api.app.openImageDialog();
  if (!result.canceled && result.filePaths.length > 0) {
    theme.value.bgImageUrl = await window.api.app.readFileBase64(result.filePaths[0]);
  }
}

function resetTheme(): void {
  config.resetTheme();
}

const ColorRow = defineComponent({
  name: 'ColorRow',
  props: {
    label: { type: String, required: true },
    value: { type: String, required: true },
    sourceMode: { type: Boolean, default: false },
  },
  emits: ['update'],
  setup(props, { emit }) {
    const displayColor = () => {
      const colorValue = props.value;
      if (!colorValue || colorValue.startsWith('rgba')) return '#888888';
      return colorValue;
    };
    return () =>
      h('div', { class: 'flex items-center gap-3 px-3 py-2 no-drag' }, [
        h(
          'label',
          {
            style: `position:relative;flex-shrink:0;${props.sourceMode ? 'cursor:default;pointer-events:none' : 'cursor:pointer'}`,
          },
          [
            h('div', {
              style: `width:20px;height:20px;border-radius:50%;background:${displayColor()};border:1.5px solid rgba(255,255,255,0.18);box-shadow:0 0 6px ${displayColor()}44`,
            }),
            props.sourceMode
              ? null
              : h('input', {
                  type: 'color',
                  value: displayColor(),
                  style:
                    'position:absolute;opacity:0;width:0;height:0;pointer-events:none',
                  onInput: (event: Event) =>
                    emit('update', (event.target as HTMLInputElement).value),
                }),
          ],
        ),
        h(
          'span',
          { class: 'text-xs flex-1', style: 'color:var(--color-ink-2)' },
          props.label,
        ),
        props.sourceMode
          ? h(
              'span',
              {
                style:
                  'font-size:0.58rem;font-weight:700;letter-spacing:0.1em;color:var(--color-accent-light);opacity:0.7',
              },
              'SOURCE',
            )
          : h('input', {
              value: props.value,
              class: 'input-field font-mono no-drag',
              style:
                'height:22px;width:80px;font-size:0.7rem;padding:0 6px;text-align:right',
              onChange: (event: Event) =>
                emit('update', (event.target as HTMLInputElement).value),
            }),
      ]);
  },
});

function opacitySliderModel(prop: 'opacity' | 'bgImageOpacity') {
  return computed({
    get: () => Math.round(theme.value[prop] * 100),
    set: (value: number) => {
      theme.value[prop] = value / 100;
    },
  });
}

const opacityModel = opacitySliderModel('opacity');
const bgImageOpacityModel = opacitySliderModel('bgImageOpacity');

const contentRoot = ref<HTMLElement | null>(null);
useStaggerReveal(contentRoot, {
  selector: '.palette-card',
  stagger: 0.025,
  y: 6,
  watchSource: () => activeTab.value,
});
</script>

<template>
  <div
    class="flex h-full overflow-hidden"
    style="pointer-events: all"
  >
    <aside
      class="theme-sidebar kyra-sidebar themed-scroll flex shrink-0 flex-col overflow-y-auto border-r py-2"
      style="width: 190px"
    >
      <div class="flex-1 px-2">
        <SideTabNav
          :tabs="tabs"
          :model-value="activeTab"
          @update:model-value="activeTab = $event as any"
        />
      </div>
      <div class="mt-auto px-2 pt-1">
        <Button
          variant="ghost"
          size="sm"
          class="w-full justify-start gap-2.5 rounded-md px-2.5 py-2 text-[0.78rem]"
          style="color: rgba(248, 113, 113, 0.7); font-weight: 500"
          @click="resetTheme"
        >
          <RotateCcw :size="11" />
          Reset
        </Button>
      </div>
    </aside>

    <div
      ref="contentRoot"
      class="themed-scroll flex flex-1 flex-col gap-3 overflow-y-auto p-3"
    >
      <template v-if="activeTab === 'palette'">
        <div class="palette-grid">
          <button
            v-for="palette in palettes"
            :key="palette.id"
            class="palette-card no-drag"
            :class="{ 'palette-card--active': config.paletteId === palette.id }"
            @click="selectPalette(palette.id)"
          >
            <div class="palette-preview">
              <div
                v-for="swatch in palette.swatches"
                :key="swatch"
                class="palette-swatch"
                :style="{ background: swatch }"
              />
            </div>
            <div class="palette-meta">
              <span class="palette-name">{{ palette.name }}</span>
              <span
                v-if="config.paletteId === palette.id"
                class="palette-active-pip"
              />
            </div>
          </button>
        </div>

        <p
          style="
            font-size: 0.69rem;
            color: var(--color-ink-3);
            text-align: center;
            padding-top: 4px;
          "
        >
          Selecting a palette resets background and all colors.
        </p>
      </template>

      <template v-if="activeTab === 'background'">
        <div class="flex gap-1.5">
          <Button
            v-for="t in bgTypes"
            :key="t.id"
            variant="control"
            size="sm"
            class="flex-1 justify-center gap-1.5"
            :class="{ 'btn-control-v2--active': theme.bgType === t.id }"
            @click="theme.bgType = t.id as any"
          >
            <component
              :is="t.icon"
              :size="11"
            />
            {{ t.label }}
          </Button>
        </div>

        <template v-if="theme.bgType === 'solid'">
          <div class="card divide-subtle">
            <div class="no-drag flex items-center gap-3 px-3 py-2.5">
              <label class="relative shrink-0 cursor-pointer">
                <div
                  class="rounded-md border"
                  style="
                    width: 32px;
                    height: 32px;
                    border-color: rgba(255, 255, 255, 0.12);
                  "
                  :style="{ background: solidHex }"
                />
                <input
                  type="color"
                  :value="solidHex"
                  class="pointer-events-none absolute h-0 w-0 opacity-0"
                  @input="onSolidColor($event)"
                />
              </label>
              <div class="flex flex-1 flex-col gap-1">
                <span class="text-ink-2 text-xs font-medium">Color</span>
                <input
                  :value="solidHex"
                  class="input-field font-mono text-xs"
                  style="height: 24px; padding: 0 8px"
                  @change="
                    (event) => onSolidHexInput((event.target as HTMLInputElement).value)
                  "
                />
              </div>
            </div>
            <div class="no-drag px-3 py-2.5">
              <div class="mb-2 flex justify-between">
                <span class="text-ink-2 text-xs font-medium">Opacity</span>
                <span class="text-accent-light font-mono text-xs tabular-nums">
                  {{ Math.round(theme.opacity * 100) }}%
                </span>
              </div>
              <Slider
                v-model="opacityModel"
                :min="0"
                :max="100"
                :step="1"
              />
            </div>
          </div>
        </template>

        <template v-if="theme.bgType === 'gradient'">
          <div
            class="h-8 rounded-lg"
            style="border: 1px solid rgba(255, 255, 255, 0.07)"
            :style="{ background: gradientPreview }"
          />

          <div class="card px-3 py-2.5">
            <span
              class="text-ink-3 mb-2 block text-xs font-semibold tracking-widest uppercase"
            >
              Direction
            </span>
            <div class="no-drag flex flex-wrap gap-1">
              <Button
                v-for="d in directions"
                :key="d.value"
                variant="control-icon"
                size="icon-sm"
                class="text-sm"
                :class="{ 'btn-control-v2--active': theme.bgGradientDir === d.value }"
                style="width: 32px; height: 28px; border-radius: 6px"
                @click="theme.bgGradientDir = d.value"
              >
                {{ d.label }}
              </Button>
            </div>
          </div>

          <div class="card divide-subtle">
            <div
              v-for="(stop, index) in theme.bgGradientStops"
              :key="index"
              class="no-drag flex items-center gap-2.5 px-3 py-2"
            >
              <label class="relative shrink-0 cursor-pointer">
                <div
                  class="rounded border"
                  style="
                    width: 22px;
                    height: 22px;
                    border-color: rgba(255, 255, 255, 0.12);
                  "
                  :style="{ background: stop.color }"
                />
                <input
                  type="color"
                  :value="stop.color"
                  class="pointer-events-none absolute h-0 w-0 opacity-0"
                  @input="stop.color = ($event.target as HTMLInputElement).value"
                />
              </label>
              <SliderRoot
                :model-value="[stop.position]"
                :min="0"
                :max="100"
                :step="1"
                class="relative flex h-4 flex-1 touch-none items-center select-none"
                @update:model-value="(value) => (stop.position = value?.[0] ?? 0)"
              >
                <SliderTrack class="bg-border relative h-[3px] grow rounded-full">
                  <SliderRange class="bg-accent absolute h-full rounded-full" />
                </SliderTrack>
                <SliderThumb
                  class="bg-accent-light border-[rgba(var(--color-accent-rgb), 0.6)] block h-3 w-3 cursor-pointer rounded-full border-2 focus:outline-none"
                />
              </SliderRoot>
              <span
                class="text-ink-3 min-w-[30px] text-right font-mono text-xs tabular-nums"
              >
                {{ stop.position }}%
              </span>
              <Button
                v-if="theme.bgGradientStops.length > 2"
                variant="control-icon"
                size="icon-sm"
                @click="removeStop(index)"
              >
                ✕
              </Button>
            </div>
            <div class="no-drag px-3 py-2">
              <Button
                variant="control"
                size="sm"
                class="gap-1.5"
                @click="addStop"
              >
                + Add Stop
              </Button>
            </div>
          </div>

          <div class="card no-drag px-3 py-2.5">
            <div class="mb-2 flex justify-between">
              <span class="text-ink-2 text-xs font-medium">Opacity</span>
              <span class="text-accent-light font-mono text-xs tabular-nums">
                {{ Math.round(theme.opacity * 100) }}%
              </span>
            </div>
            <Slider
              v-model="opacityModel"
              :min="0"
              :max="100"
              :step="1"
            />
          </div>
        </template>

        <template v-if="theme.bgType === 'image'">
          <div class="card divide-subtle">
            <div class="no-drag px-3 py-2.5">
              <span class="text-ink-2 mb-2 block text-xs font-medium">
                Background Image
              </span>
              <div class="flex flex-col gap-2">
                <Button
                  variant="control"
                  size="sm"
                  class="justify-center gap-2"
                  @click="pickLocalImage"
                >
                  <Image :size="12" />
                  Choose local image…
                </Button>
                <div
                  v-if="theme.bgImageUrl"
                  class="flex items-center gap-2 rounded px-2 py-1.5"
                  style="
                    background: rgba(255, 255, 255, 0.04);
                    border: 1px solid var(--color-border);
                  "
                >
                  <Image
                    :size="10"
                    class="text-good shrink-0"
                  />
                  <span class="text-ink-3 flex-1 truncate font-mono text-xs">
                    {{ imageFilename }}
                  </span>
                  <Button
                    variant="control-icon"
                    size="icon-sm"
                    @click="theme.bgImageUrl = ''"
                  >
                    ✕
                  </Button>
                </div>
              </div>
            </div>

            <div class="no-drag px-3 py-2.5">
              <div class="mb-2 flex justify-between">
                <span class="text-ink-2 text-xs font-medium">Image opacity</span>
                <span class="text-accent-light font-mono text-xs tabular-nums">
                  {{ Math.round(theme.bgImageOpacity * 100) }}%
                </span>
              </div>
              <Slider
                v-model="bgImageOpacityModel"
                :min="0"
                :max="100"
                :step="1"
              />
            </div>

            <div class="no-drag px-3 py-2.5">
              <div class="mb-2 flex justify-between">
                <span class="text-ink-2 text-xs font-medium">Overlay opacity</span>
                <span class="text-accent-light font-mono text-xs tabular-nums">
                  {{ Math.round(theme.opacity * 100) }}%
                </span>
              </div>
              <Slider
                v-model="opacityModel"
                :min="0"
                :max="100"
                :step="1"
              />
            </div>
          </div>
        </template>

        <div>
          <div class="mb-1.5 flex items-center gap-2">
            <span class="text-ink-3 text-xs font-semibold tracking-widest uppercase">
              Preview
            </span>
            <div class="bg-border h-px flex-1" />
          </div>
          <div
            class="relative overflow-hidden rounded-lg"
            style="height: 64px; border: 1px solid var(--color-border)"
          >
            <div
              v-if="theme.bgType === 'image' && theme.bgImageUrl"
              class="absolute inset-0"
              :style="{
                backgroundImage: `url('${theme.bgImageUrl}')`,
                backgroundSize: 'cover',
                backgroundPosition: 'center',
                opacity: theme.bgImageOpacity,
              }"
            />
            <div
              v-if="theme.bgType !== 'image'"
              class="absolute inset-0"
              :style="{ background: previewBg, opacity: theme.opacity }"
            />
            <div class="absolute inset-0 flex items-center gap-3 px-3">
              <div
                class="h-8 w-8 shrink-0 rounded-md"
                style="
                  background: rgba(var(--color-accent-rgb), 0.25);
                  border: 1px solid rgba(var(--color-accent-rgb), 0.35);
                "
              />
              <div class="flex min-w-0 flex-1 flex-col gap-1">
                <div class="flex items-center gap-3">
                  <span class="text-ink-1 text-xs font-bold">PlayerName</span>
                  <span class="text-good font-mono text-xs">6.50</span>
                  <span class="text-warn font-mono text-xs">2.30</span>
                  <span class="text-accent-light font-mono text-xs">1,204</span>
                </div>
                <div class="bg-border h-px w-full" />
              </div>
            </div>
          </div>
        </div>
      </template>

      <template v-if="activeTab === 'colors'">
        <div
          class="no-drag flex items-center justify-between rounded-lg px-3 py-3 transition-all"
          :style="
            theme.dynamicColors
              ? 'background:rgba(var(--color-accent-rgb), 0.1);border:1px solid rgba(var(--color-accent-rgb), 0.32);box-shadow:0 0 14px rgba(var(--color-accent-rgb), 0.08)'
              : 'background:var(--color-surface-1);border:1px solid var(--color-border)'
          "
        >
          <div class="flex items-center gap-2.5">
            <div
              class="flex h-7 w-7 shrink-0 items-center justify-center rounded-md transition-all"
              :style="
                theme.dynamicColors
                  ? 'background:rgba(var(--color-accent-rgb), 0.28);color:var(--color-accent-light)'
                  : 'background:var(--color-surface-2);color:var(--color-ink-3)'
              "
            >
              <Sparkles :size="13" />
            </div>
            <div class="flex flex-col gap-0.5">
              <span
                class="text-xs font-semibold transition-colors"
                :style="
                  theme.dynamicColors
                    ? 'color:var(--color-accent-light)'
                    : 'color:var(--color-ink-2)'
                "
                >Dynamic Colors</span
              >
              <span style="font-size: 0.67rem; color: var(--color-ink-3)">
                Derive all colors from accent hue
              </span>
            </div>
          </div>
          <Switch
            :model-value="theme.dynamicColors"
            @update:model-value="toggleDynamic"
          />
        </div>

        <div>
          <div class="mb-1.5 flex items-center gap-2">
            <span class="text-ink-3 text-xs font-semibold tracking-widest uppercase">
              Interface
            </span>
            <div class="bg-border h-px flex-1" />
          </div>

          <div class="card mb-1.5">
            <ColorRow
              label="Accent"
              :value="theme.colors.accent"
              :source-mode="theme.dynamicColors"
              @update="theme.colors.accent = $event"
            />
          </div>

          <div
            class="card divide-subtle transition-opacity duration-200"
            :style="theme.dynamicColors ? 'opacity:0.42;pointer-events:none' : ''"
          >
            <ColorRow
              v-for="c in derivedColors"
              :key="c.key"
              :label="c.label"
              :value="(theme.colors as any)[c.key]"
              @update="(theme.colors as any)[c.key] = $event"
            />
          </div>
        </div>

        <div>
          <div class="mb-1.5 flex items-center gap-2">
            <span class="text-ink-3 text-xs font-semibold tracking-widest uppercase">
              Rank Colors
            </span>
            <div class="bg-border h-px flex-1" />
          </div>
          <div
            class="card divide-subtle transition-opacity duration-200"
            :style="theme.dynamicColors ? 'opacity:0.42;pointer-events:none' : ''"
          >
            <ColorRow
              v-for="r in rankColors"
              :key="r.key"
              :label="r.label"
              :value="(theme.colors as any)[r.key]"
              @update="(theme.colors as any)[r.key] = $event"
            />
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.palette-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 8px;
}

.palette-card {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 7px;
  padding: 9px;
  border-radius: 10px;
  background: linear-gradient(
    160deg,
    rgba(255, 255, 255, 0.05) 0%,
    rgba(255, 255, 255, 0.015) 100%
  );
  border: 1px solid rgba(255, 255, 255, 0.07);
  backdrop-filter: blur(12px) saturate(130%);
  -webkit-backdrop-filter: blur(12px) saturate(130%);
  cursor: pointer;
  transition:
    border-color 150ms ease,
    background 150ms ease,
    box-shadow 150ms ease,
    transform 150ms ease;
  text-align: left;
}

.palette-card:hover {
  border-color: rgba(var(--color-accent-rgb), 0.28);
  background: rgba(var(--color-accent-rgb), 0.04);
  transform: translateY(-1px);
}

.palette-card--active {
  border-color: rgba(var(--color-accent-rgb), 0.5);
  background: rgba(var(--color-accent-rgb), 0.09);
  box-shadow: 0 0 18px rgba(var(--color-accent-rgb), 0.22);
}

html.low-end .palette-card {
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}

html.low-end .palette-card:hover {
  transform: none;
}

.palette-preview {
  display: flex;
  height: 28px;
  border-radius: 6px;
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.palette-swatch {
  flex: 1;
}

.palette-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.palette-name {
  font-size: 0.76rem;
  font-weight: 600;
  color: var(--color-ink-2);
}

.palette-card--active .palette-name {
  color: var(--color-accent-light);
}

.palette-active-pip {
  display: inline-block;
  width: 5px;
  height: 5px;
  border-radius: 9999px;
  background: var(--color-accent);
  box-shadow: 0 0 5px rgba(var(--color-accent-rgb), 0.7);
  flex-shrink: 0;
}
</style>
