<script setup lang="ts">
import type { PremiumAuthState } from '@renderer/composables/usePremiumAuth';
import { useConfigStore } from '@renderer/store/config';
import { NETWORKS } from '@renderer/types';
import {
  AlertTriangle,
  Check,
  Copy,
  ExternalLink,
  ShieldCheck,
  X,
} from 'lucide-vue-next';
import { computed, onMounted, onUnmounted, ref } from 'vue';

const props = defineProps<{
  auth: PremiumAuthState;
}>();

const emit = defineEmits<{ close: [] }>();

const config = useConfigStore();
const visible = ref(false);
const copied = ref(false);
const copiedLink = ref(false);
const now = ref(Date.now());

let tickTimer: ReturnType<typeof setInterval> | null = null;
let copiedTimer: ReturnType<typeof setTimeout> | null = null;

const networkLabel = computed(
  () =>
    NETWORKS.find((network) => network.value === props.auth.network)?.label ??
    props.auth.network,
);

const secondsLeft = computed(() =>
  Math.max(0, Math.round((props.auth.expiresAt - now.value) / 1000)),
);

const timeLeftLabel = computed(() => {
  const totalSeconds = secondsLeft.value;
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;

  return `${minutes}:${seconds.toString().padStart(2, '0')}`;
});

function handleOpen(): void {
  window.api.win.openExternal(props.auth.verificationUri);
}

function copyText(text: string, target: 'code' | 'link'): void {
  void window.api.clipboard.setText(text).then(() => {
    if (target === 'code') copied.value = true;
    else copiedLink.value = true;

    if (copiedTimer) {
      clearTimeout(copiedTimer);
    }

    copiedTimer = setTimeout(() => {
      copied.value = false;
      copiedLink.value = false;
    }, 1600);
  });
}

function handleCopy(): void {
  copyText(props.auth.userCode, 'code');
}

function handleCopyLink(): void {
  copyText(props.auth.verificationUri, 'link');
}

function handleClose(): void {
  visible.value = false;

  setTimeout(emit, 300, 'close');
}

onMounted(() => {
  requestAnimationFrame(() => {
    visible.value = true;
  });

  tickTimer = setInterval(() => {
    now.value = Date.now();
  }, 1000);
});

onUnmounted(() => {
  if (tickTimer) {
    clearInterval(tickTimer);
  }

  if (copiedTimer) {
    clearTimeout(copiedTimer);
  }
});
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
        <div class="card-glow" />

        <div class="card-header">
          <div class="header-icon">
            <ShieldCheck :size="14" />
          </div>

          <div class="header-text">
            <span class="header-eyebrow">Premium Sign-In</span>
            <span class="network-pill">{{ networkLabel }}</span>
          </div>

          <button
            class="close-btn"
            @click="handleClose"
          >
            <X :size="12" />
          </button>
        </div>

        <div class="divider" />

        <div class="card-body">
          <p class="explainer">
            This account requires a one-time Microsoft sign-in to use the proxy on
            {{ networkLabel }}. You won't be asked again after this.
          </p>

          <div class="code-block">
            <span class="code-text">{{ auth.userCode || '—' }}</span>

            <button
              class="copy-btn"
              :disabled="!auth.userCode"
              @click="handleCopy"
            >
              <Check
                v-if="copied"
                :size="13"
              />
              <Copy
                v-else
                :size="13"
              />
            </button>
          </div>

          <button
            class="open-btn"
            @click="handleOpen"
          >
            <span class="open-btn-content">
              <ExternalLink
                :size="14"
                class="open-btn-icon"
                aria-hidden="true"
              />
              <span class="open-btn-label">Open Sign-In Page</span>
            </span>
          </button>

          <button
            class="copy-link-btn"
            @click="handleCopyLink"
          >
            <Check
              v-if="copiedLink"
              :size="13"
            />
            <Copy
              v-else
              :size="13"
            />
            <span>{{ copiedLink ? 'Link copied' : 'Copy sign-in link' }}</span>
          </button>

          <div class="status-row">
            <template v-if="auth.status === 'waiting'">
              <span class="spinner" />
              <span class="status-text">
                Waiting for sign-in{{
                  secondsLeft > 0 ? ` · code expires in ${timeLeftLabel}` : ''
                }}
              </span>
            </template>

            <template v-else-if="auth.status === 'success'">
              <span class="status-icon success">
                <Check :size="13" />
              </span>
              <span class="status-text success">Signed in successfully</span>
            </template>

            <template v-else>
              <span class="status-icon error">
                <AlertTriangle :size="13" />
              </span>
              <span class="status-text error">{{ auth.errorMessage }}</span>
            </template>
          </div>
        </div>

        <div class="card-footer">
          <button
            class="dismiss-btn"
            @click="handleClose"
          >
            Close
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
  z-index: 12010;
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
  background: rgba(0, 0, 0, 0.7);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  pointer-events: all;
}

.card {
  position: relative;
  width: 380px;
  max-width: calc(100vw - 32px);
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
  width: 300px;
  height: 150px;
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

.network-pill {
  font-size: 10.5px;
  font-weight: 600;
  color: var(--color-accent);
  background: rgba(var(--color-accent-rgb), 0.14);
  border: 1px solid rgba(var(--color-accent-rgb), 0.3);
  border-radius: 5px;
  padding: 1px 7px;
  letter-spacing: 0.03em;
  flex-shrink: 0;
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

.card-body {
  position: relative;
  z-index: 1;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.explainer {
  margin: 0;
  font-size: 12.5px;
  line-height: 1.55;
  color: rgba(255, 255, 255, 0.6);
}

.code-block {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 12px 14px;
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.35);
  border: 1px solid rgba(var(--color-accent-rgb), 0.25);
}

.code-text {
  font-family: var(--font-mono);
  font-size: 20px;
  font-weight: 700;
  letter-spacing: 0.12em;
  color: var(--color-accent-light);
}

.copy-btn {
  width: 28px;
  height: 28px;
  border-radius: 7px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-accent-light);
  background: rgba(var(--color-accent-rgb), 0.14);
  border: 1px solid rgba(var(--color-accent-rgb), 0.3);
  cursor: pointer;
  transition:
    background 0.14s,
    border-color 0.14s;
}

.copy-btn:hover:not(:disabled) {
  background: rgba(var(--color-accent-rgb), 0.26);
  border-color: rgba(var(--color-accent-rgb), 0.5);
}

.copy-btn:disabled {
  opacity: 0.4;
  cursor: default;
}

.open-btn {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 9px 14px;
  border-radius: 9px;
  color: var(--color-accent-light);
  background: rgba(var(--color-accent-rgb), 0.16);
  border: 1px solid rgba(var(--color-accent-rgb), 0.35);
  cursor: pointer;
  transition:
    background 0.14s,
    border-color 0.14s,
    color 0.14s,
    box-shadow 0.14s;
}

.open-btn-content {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  line-height: 1;
}

.open-btn-icon {
  display: block;
  position: relative;
  top: -1.5px;
  flex-shrink: 0;
}

.open-btn-label {
  display: block;
  position: relative;
  top: 1px;
  font-size: 12.5px;
  font-weight: 600;
  line-height: 1;
  letter-spacing: 0.02em;
  white-space: nowrap;
}

.open-btn:hover {
  background: rgba(var(--color-accent-rgb), 0.28);
  border-color: rgba(var(--color-accent-rgb), 0.55);
  color: var(--color-ink-1);
  box-shadow: 0 0 18px rgba(var(--color-accent-rgb), 0.2);
}

.copy-link-btn {
  display: inline-flex;
  align-items: center;
  align-self: center;
  gap: 6px;
  padding: 5px 8px;
  border: 0;
  color: rgba(255, 255, 255, 0.58);
  background: transparent;
  font-size: 11px;
  cursor: pointer;
}

.copy-link-btn:hover {
  color: var(--color-accent-light);
}

.status-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 2px 2px 0;
}

.status-text {
  font-size: 11.5px;
  color: rgba(255, 255, 255, 0.45);
  line-height: 1.4;
}

.status-text.success {
  color: #4ade80;
}

.status-text.error {
  color: #f87171;
}

.status-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  flex-shrink: 0;
}

.status-icon.success {
  color: #4ade80;
}

.status-icon.error {
  color: #f87171;
}

.spinner {
  width: 12px;
  height: 12px;
  flex-shrink: 0;
  border-radius: 50%;
  border: 2px solid rgba(var(--color-accent-rgb), 0.25);
  border-top-color: var(--color-accent-light);
  animation: spin 0.7s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.card-footer {
  display: flex;
  justify-content: flex-end;
  padding: 4px 15px 14px;
  flex-shrink: 0;
}

.dismiss-btn {
  font-size: 12px;
  font-weight: 600;
  padding: 7px 20px;
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: rgba(255, 255, 255, 0.03);
  color: rgba(255, 255, 255, 0.55);
  cursor: pointer;
  letter-spacing: 0.02em;
  transition:
    background 0.14s,
    border-color 0.14s,
    color 0.14s;
}

.dismiss-btn:hover {
  background: rgba(255, 255, 255, 0.07);
  border-color: rgba(255, 255, 255, 0.16);
  color: rgba(255, 255, 255, 0.85);
}
</style>
