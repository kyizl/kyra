<script setup lang="ts">
import type {
  AlertPayload,
  ChangelogPayload,
} from '@renderer/composables/useAnnouncements';
import { useConfigStore } from '@renderer/store/config';
import DOMPurify from 'dompurify';
import { AlertTriangle, ArrowUpCircle, Bell, Info, Wrench, X } from 'lucide-vue-next';
import { marked } from 'marked';
import { computed, onMounted, ref } from 'vue';

const props = defineProps<{
  mode: 'changelog' | 'alert';
  changelog?: ChangelogPayload;
  alert?: AlertPayload;
}>();

const emit = defineEmits<{ close: [] }>();

const config = useConfigStore();
const visible = ref(false);

marked.setOptions({
  gfm: true,
  breaks: true,
});

const renderedContent = computed((): string => {
  const raw =
    props.mode === 'changelog' ? props.changelog?.content : props.alert?.content;

  if (!raw) {
    return '';
  }

  const normalized = raw
    .replace(/\\n/g, '\n')
    .replace(/__(.+?)__/g, '<u>$1</u>')
    .replace(/\n([-*+]|\d+\.) /g, '\n\n$1 ');

  const result = marked.parse(normalized);

  const html = typeof result === 'string' ? result : '';

  return DOMPurify.sanitize(html);
});

type AlertType = 'info' | 'warning' | 'update' | 'maintenance';

function getCSSVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

const alertMeta = computed(
  (): Record<
    AlertType,
    {
      icon: typeof Info;
      color: string;
      glow: string;
      label: string;
    }
  > => {
    const accent = getCSSVar('--color-accent');
    const accentRgb = getCSSVar('--color-accent-rgb');
    return {
      info: {
        icon: Info,
        color: '#60a5fa',
        glow: 'rgba(96,165,250,0.28)',
        label: 'Information',
      },
      warning: {
        icon: AlertTriangle,
        color: '#fbbf24',
        glow: 'rgba(251,191,36,0.28)',
        label: 'Warning',
      },
      update: {
        icon: ArrowUpCircle,
        color: accent,
        glow: `rgba(${accentRgb},0.28)`,
        label: 'Update',
      },
      maintenance: {
        icon: Wrench,
        color: '#94a3b8',
        glow: 'rgba(148,163,184,0.28)',
        label: 'Maintenance',
      },
    };
  },
);

const currentAlertMeta = computed(
  () => alertMeta.value[(props.alert?.type as AlertType) ?? 'info'],
);

onMounted(() => {
  requestAnimationFrame(() => {
    visible.value = true;
  });
});

function handleClose(): void {
  visible.value = false;

  setTimeout(emit, 300, 'close');
}
</script>

<template>
  <Teleport to="body">
    <div
      class="backdrop"
      :class="{ visible }"
      :style="{ borderRadius: config.roundedCorners ? '14px' : '0px' }"
      @click.self="handleClose"
    >
      <div
        class="card"
        :class="{ visible }"
        role="dialog"
        aria-modal="true"
      >
        <div
          class="card-glow"
          :style="
            mode === 'alert'
              ? {
                  background: `radial-gradient(ellipse at 50% 0%, ${currentAlertMeta.glow} 0%, transparent 65%)`,
                }
              : {}
          "
        />

        <div class="card-header">
          <div
            class="header-icon"
            :style="
              mode === 'alert'
                ? {
                    color: currentAlertMeta.color,
                    background: currentAlertMeta.glow.replace('0.28', '0.12'),
                    borderColor: `${currentAlertMeta.color}55`,
                    boxShadow: `0 0 14px ${currentAlertMeta.glow}`,
                  }
                : {}
            "
          >
            <component
              :is="mode === 'changelog' ? Bell : currentAlertMeta.icon"
              :size="14"
            />
          </div>

          <div class="header-text">
            <span
              class="header-eyebrow"
              :style="
                mode === 'alert'
                  ? {
                      color: currentAlertMeta.color,
                    }
                  : {}
              "
            >
              {{ mode === 'changelog' ? "What's New" : currentAlertMeta.label }}
            </span>

            <span
              v-if="mode === 'changelog' && changelog"
              class="version-pill"
            >
              v{{ changelog.version }}
            </span>
          </div>

          <button
            class="close-btn"
            @click="handleClose"
          >
            <X :size="12" />
          </button>
        </div>

        <div class="divider" />

        <div
          v-if="mode === 'alert' && alert"
          class="alert-title-row"
        >
          <div
            class="alert-type-bar"
            :style="{
              background: currentAlertMeta.color,
            }"
          />

          <h2
            class="alert-heading"
            :style="{
              color: currentAlertMeta.color,
            }"
          >
            {{ alert.title }}
          </h2>
        </div>

        <div class="card-body">
          <div
            v-if="mode === 'changelog' && changelog?.date"
            class="release-date"
          >
            Released
            {{
              new Date(changelog.date).toLocaleDateString('en-US', {
                year: 'numeric',
                month: 'long',
                day: 'numeric',
              })
            }}
          </div>

          <!-- eslint-disable vue/no-v-html -->
          <div
            class="markdown-content"
            v-html="renderedContent"
          />
          <!-- eslint-enable vue/no-v-html -->
        </div>

        <div class="card-footer">
          <button
            class="dismiss-btn"
            @click="handleClose"
          >
            Got it
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 12000;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0);
  backdrop-filter: blur(0px);
  -webkit-backdrop-filter: blur(0px);

  overflow: hidden;
  contain: paint;
  transition:
    background 0.28s ease,
    backdrop-filter 0.28s ease;
  pointer-events: none;
}

.backdrop.visible {
  background: rgba(6, 4, 6, 0.94);
  backdrop-filter: blur(22px) saturate(115%);
  -webkit-backdrop-filter: blur(22px) saturate(115%);
  pointer-events: all;
}

html.low-end .backdrop.visible {
  background: rgba(6, 4, 6, 0.98);
  backdrop-filter: none !important;
  -webkit-backdrop-filter: none !important;
}

.card {
  position: relative;
  width: 420px;
  max-width: calc(100vw - 32px);
  max-height: calc(100vh - 80px);
  display: flex;
  flex-direction: column;
  background: rgba(var(--color-bg-rgb), 0.9);
  backdrop-filter: blur(22px) saturate(150%);
  -webkit-backdrop-filter: blur(22px) saturate(150%);
  border: 1px solid rgba(var(--color-accent-rgb), 0.2);
  border-radius: 16px;
  box-shadow:
    0 0 0 1px rgba(255, 255, 255, 0.045) inset,
    0 32px 80px rgba(0, 0, 0, 0.7),
    0 0 60px rgba(var(--color-accent-rgb), 0.1);
  opacity: 0;
  transform: scale(0.95) translateY(16px);
  transition:
    opacity 0.28s cubic-bezier(0.16, 1, 0.3, 1),
    transform 0.28s cubic-bezier(0.16, 1, 0.3, 1);
  overflow: hidden;
}

html.low-end .card {
  backdrop-filter: none !important;
  -webkit-backdrop-filter: none !important;
  background: rgba(var(--color-bg-rgb), 0.97) !important;
}

.card.visible {
  opacity: 1;
  transform: scale(1) translateY(0);
}

.card-glow {
  position: absolute;
  top: -80px;
  left: 50%;
  transform: translateX(-50%);
  width: 340px;
  height: 160px;
  background: radial-gradient(
    ellipse at 50% 0%,
    rgba(var(--color-accent-rgb), 0.32) 0%,
    transparent 65%
  );
  pointer-events: none;
  z-index: 0;
}

.card-header {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 15px 15px 13px;
  flex-shrink: 0;
}

.header-icon {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  color: var(--color-accent-light);
  background: rgba(var(--color-accent-rgb), 0.15);
  border: 1px solid rgba(var(--color-accent-rgb), 0.3);
  box-shadow: 0 0 12px rgba(var(--color-accent-rgb), 0.22);
}

.header-text {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  overflow: hidden;
}

.header-eyebrow {
  font-size: 10.5px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--color-accent-light);
  flex-shrink: 0;
}

.version-pill {
  font-size: 10.5px;
  font-weight: 600;
  font-family: var(--font-mono);
  color: var(--color-accent);
  background: rgba(var(--color-accent-rgb), 0.14);
  border: 1px solid rgba(var(--color-accent-rgb), 0.3);
  border-radius: 5px;
  padding: 1px 7px;
  letter-spacing: 0.03em;
  flex-shrink: 0;
}

.header-title {
  font-size: 13px;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.75);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.close-btn {
  width: 24px;
  height: 24px;
  border-radius: 6px;
  border: 1px solid rgba(255, 255, 255, 0.06);
  background: rgba(255, 255, 255, 0.03);
  color: rgba(255, 255, 255, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  flex-shrink: 0;
  transition:
    background 0.14s,
    color 0.14s,
    border-color 0.14s;
}

.close-btn:hover {
  background: rgba(255, 255, 255, 0.07);
  color: rgba(255, 255, 255, 0.65);
  border-color: rgba(255, 255, 255, 0.12);
}

.divider {
  height: 1px;
  background: linear-gradient(
    90deg,
    transparent 0%,
    rgba(var(--color-accent-rgb), 0.3) 15%,
    rgba(var(--color-accent-rgb), 0.3) 85%,
    transparent 100%
  );
  flex-shrink: 0;
}

.alert-title-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 16px 2px;
  flex-shrink: 0;
}

.alert-type-bar {
  width: 3px;
  height: 22px;
  border-radius: 2px;
  flex-shrink: 0;
  opacity: 0.9;
}

.alert-heading {
  margin: 0;
  font-size: 16px;
  font-weight: 700;
  letter-spacing: -0.01em;
  line-height: 1.3;
}

.release-date {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.26);
  margin-bottom: 14px;
  letter-spacing: 0.02em;
}

.card-body {
  flex: 1;
  overflow-y: auto;
  padding: 14px 16px 6px;
  scrollbar-width: thin;
  scrollbar-color: rgba(var(--color-accent-rgb), 0.18) transparent;
  min-height: 0;
}

.markdown-content {
  font-size: 13px;
  line-height: 1.65;
  color: rgba(255, 255, 255, 0.78);
}

.markdown-content :deep(h1),
.markdown-content :deep(h2),
.markdown-content :deep(h3),
.markdown-content :deep(h4) {
  color: var(--color-ink-1);
  font-weight: 700;
  margin: 1.1em 0 0.45em;
  line-height: 1.3;
  letter-spacing: -0.01em;
}

.markdown-content :deep(h1) {
  font-size: 17px;
}

.markdown-content :deep(h2) {
  font-size: 14.5px;
}

.markdown-content :deep(h3) {
  font-size: 13px;
  color: rgba(255, 255, 255, 0.88);
}

.markdown-content :deep(h1:first-child),
.markdown-content :deep(h2:first-child) {
  margin-top: 0;
}

.markdown-content :deep(p) {
  margin: 0 0 0.8em;
}

.markdown-content :deep(p:last-child) {
  margin-bottom: 0;
}

.markdown-content :deep(ul),
.markdown-content :deep(ol) {
  margin: 0.4em 0 0.8em;
  padding-left: 1.4em;
}

.markdown-content :deep(ul) {
  list-style-type: disc;
}

.markdown-content :deep(ol) {
  list-style-type: decimal;
}

.markdown-content :deep(li) {
  margin: 0.25em 0;
}

.markdown-content :deep(li::marker) {
  color: rgba(var(--color-accent-rgb), 0.7);
}

.markdown-content :deep(strong) {
  color: var(--color-ink-1);
  font-weight: 600;
}

.markdown-content :deep(em) {
  color: rgba(255, 255, 255, 0.85);
  font-style: italic;
}

.markdown-content :deep(code) {
  font-family: var(--font-mono);
  font-size: 11.5px;
  background: rgba(var(--color-accent-rgb), 0.12);
  border: 1px solid rgba(var(--color-accent-rgb), 0.2);
  border-radius: 4px;
  padding: 1px 5px;
  color: var(--color-accent-light);
}

.markdown-content :deep(pre) {
  background: rgba(0, 0, 0, 0.35);
  border: 1px solid rgba(var(--color-accent-rgb), 0.15);
  border-radius: 8px;
  padding: 12px 14px;
  overflow-x: auto;
  margin: 0.8em 0;
}

.markdown-content :deep(pre code) {
  background: none;
  border: none;
  padding: 0;
  color: var(--color-accent-light);
  font-size: 11.5px;
}

.markdown-content :deep(blockquote) {
  border-left: 3px solid rgba(var(--color-accent-rgb), 0.5);
  margin: 0.8em 0;
  padding: 6px 12px;
  color: rgba(255, 255, 255, 0.58);
  font-style: italic;
  background: rgba(var(--color-accent-rgb), 0.06);
  border-radius: 0 6px 6px 0;
}

.markdown-content :deep(a) {
  color: var(--color-accent-light);
  text-decoration: none;
}

.markdown-content :deep(a:hover) {
  text-decoration: underline;
}

.markdown-content :deep(hr) {
  border: none;
  height: 1px;
  background: rgba(var(--color-accent-rgb), 0.2);
  margin: 1em 0;
}

.markdown-content :deep(table) {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
  margin: 0.8em 0;
}

.markdown-content :deep(th) {
  padding: 6px 10px;
  text-align: left;
  color: rgba(255, 255, 255, 0.9);
  font-weight: 600;
  border-bottom: 1px solid rgba(var(--color-accent-rgb), 0.25);
  background: rgba(var(--color-accent-rgb), 0.08);
}

.markdown-content :deep(td) {
  padding: 5px 10px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  color: rgba(255, 255, 255, 0.72);
}

.card-footer {
  display: flex;
  justify-content: flex-end;
  padding: 10px 15px 14px;
  flex-shrink: 0;
}

.dismiss-btn {
  font-size: 12px;
  font-weight: 600;
  padding: 7px 20px;
  border-radius: 8px;
  border: 1px solid rgba(var(--color-accent-rgb), 0.35);
  background: rgba(var(--color-accent-rgb), 0.16);
  color: var(--color-accent-light);
  cursor: pointer;
  letter-spacing: 0.02em;
  transition:
    background 0.14s,
    border-color 0.14s,
    color 0.14s,
    box-shadow 0.14s;
}

.dismiss-btn:hover {
  background: rgba(var(--color-accent-rgb), 0.28);
  border-color: rgba(var(--color-accent-rgb), 0.55);
  color: var(--color-ink-1);
  box-shadow: 0 0 18px rgba(var(--color-accent-rgb), 0.2);
}
</style>
