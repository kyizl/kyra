<script setup lang="ts">
import UsageDisclosure from '@renderer/components/UsageDisclosure.vue';
import { useConfigStore } from '@renderer/store/config';
import { ExternalLink, Fingerprint, Image, ShieldCheck, User } from 'lucide-vue-next';
import { onMounted, ref } from 'vue';

defineProps<{
  linking: boolean;
  errorMessage: string;
}>();

const emit = defineEmits<{ link: [] }>();

const config = useConfigStore();
const visible = ref(false);

function handleLink(): void {
  emit('link');
}

onMounted(() => {
  requestAnimationFrame(() => {
    visible.value = true;
  });
});
</script>

<template>
  <Teleport to="body">
    <div
      class="backdrop"
      :class="{ visible }"
      :style="{ borderRadius: config.roundedCorners ? '14px' : '0px' }"
    >
      <div
        class="card"
        :class="{ visible }"
        role="dialog"
        aria-modal="true"
      >
        <div class="card-body">
          <div class="header-icon">
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="currentColor"
              aria-hidden="true"
            >
              <path
                d="M20.317 4.37a19.79 19.79 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.058a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028 14.09 14.09 0 0 0 1.226-1.994.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.3 12.3 0 0 1-1.873.892.076.076 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.84 19.84 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.028ZM8.02 15.331c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.211 0 2.176 1.096 2.157 2.42 0 1.333-.955 2.418-2.157 2.418Zm7.974 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.211 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418Z"
              />
            </svg>
          </div>

          <div class="header">
            <h2 class="title">Connect your Discord account</h2>
            <p class="subtitle">A one-time sign-in to verify your account.</p>
          </div>

          <div class="data-section">
            <span class="section-label">Information requested for verification</span>

            <div class="data-list">
              <div class="data-item">
                <Fingerprint
                  :size="15"
                  class="data-icon"
                />
                <span class="data-name">Discord User ID</span>
              </div>
              <div class="data-item">
                <User
                  :size="15"
                  class="data-icon"
                />
                <span class="data-name">Username</span>
              </div>
              <div class="data-item">
                <Image
                  :size="15"
                  class="data-icon"
                />
                <span class="data-name">Avatar</span>
              </div>
            </div>
          </div>

          <div class="privacy-note">
            <ShieldCheck
              :size="14"
              class="privacy-icon"
            />
            <p>
              Data is used solely to prevent misuse. No other data is accessed or stored
              beyond what is listed above.
            </p>
          </div>

          <UsageDisclosure />

          <button
            class="link-btn"
            :disabled="linking"
            @click="handleLink"
          >
            <span class="link-btn-content">
              <span
                v-if="linking"
                class="spinner"
                aria-hidden="true"
              />
              <ExternalLink
                v-else
                :size="14"
                class="link-btn-icon"
                aria-hidden="true"
              />
              <span class="link-btn-label">{{
                linking ? 'Waiting for Discord…' : 'Continue with Discord'
              }}</span>
            </span>
          </button>

          <p
            v-if="errorMessage"
            class="error-text"
          >
            {{ errorMessage }}
          </p>

          <p class="consent-note">
            By continuing, you authorise Kyra Overlay to receive the information listed
            above from Discord for account verification, and to send the usage statistics
            described above.
          </p>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 12020;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--modal-scrim-clear);
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
  background: var(--modal-scrim);
  backdrop-filter: blur(22px) saturate(115%);
  -webkit-backdrop-filter: blur(22px) saturate(115%);
  pointer-events: all;
}

html.low-end .backdrop.visible {
  background: var(--modal-surface);
  backdrop-filter: none !important;
  -webkit-backdrop-filter: none !important;
}

.card {
  position: relative;
  z-index: 11510;
  width: 340px;
  max-width: calc(100vw - 32px);
  background: rgba(var(--color-bg-rgb), 0.9);
  backdrop-filter: blur(22px) saturate(150%);
  -webkit-backdrop-filter: blur(22px) saturate(150%);
  border: 1px solid rgba(var(--color-accent-rgb), 0.2);
  border-radius: 14px;
  box-shadow:
    0 0 0 1px var(--modal-line) inset,
    0 24px 60px rgba(0, 0, 0, 0.55),
    0 0 50px rgba(var(--color-accent-rgb), 0.1);
  opacity: 0;
  transform: scale(0.97) translateY(10px);
  transition:
    opacity 0.22s cubic-bezier(0.16, 1, 0.3, 1),
    transform 0.22s cubic-bezier(0.16, 1, 0.3, 1);
}

html.low-end .card {
  backdrop-filter: none !important;
  -webkit-backdrop-filter: none !important;
  background: rgb(var(--color-bg-rgb)) !important;
}

.card.visible {
  opacity: 1;
  transform: scale(1) translateY(0);
}

.card-body {
  padding: 28px 24px 22px;
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
}

.header-icon {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  color: var(--color-accent-light);
  background: rgba(var(--color-accent-rgb), 0.12);
  border: 1px solid rgba(var(--color-accent-rgb), 0.22);
  box-shadow: 0 0 16px rgba(var(--color-accent-rgb), 0.2);
  margin-bottom: 16px;
}

html.low-end .header-icon {
  box-shadow: none !important;
}

.header {
  margin-bottom: 22px;
}

.title {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
  letter-spacing: -0.01em;
  color: var(--color-ink-1);
}

.subtitle {
  margin: 6px 0 0;
  font-size: 12.5px;
  line-height: 1.5;
  color: var(--modal-ink-muted);
}

.data-section {
  width: 100%;
  margin-bottom: 16px;
}

.section-label {
  display: block;
  font-size: 10.5px;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--modal-ink-faint);
  margin-bottom: 8px;
  text-align: left;
}

.data-list {
  display: flex;
  flex-direction: column;
  background: var(--modal-fill);
  border: 1px solid var(--modal-line);
  border-radius: 10px;
  overflow: hidden;
}

.data-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 12px;
  text-align: left;
}

.data-item + .data-item {
  border-top: 1px solid var(--modal-line);
}

.data-icon {
  flex-shrink: 0;
  color: var(--modal-ink-muted);
}

.data-name {
  font-size: 12.5px;
  font-weight: 500;
  color: var(--modal-ink);
}

.privacy-note {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  text-align: left;
  margin-bottom: 20px;
}

.privacy-icon {
  flex-shrink: 0;
  margin-top: 1px;
  color: var(--modal-ink-faint);
}

.privacy-note p {
  margin: 0;
  font-size: 11.5px;
  line-height: 1.5;
  color: var(--modal-ink-muted);
}

.link-btn {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 10px 14px;
  border-radius: 10px;
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

.link-btn-content {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  line-height: 1;
}

.link-btn-icon {
  display: block;
  position: relative;
  top: -1.5px;
  flex-shrink: 0;
}

.link-btn-label {
  display: block;
  position: relative;
  top: 1px;
  font-size: 12.5px;
  font-weight: 600;
  line-height: 1;
  letter-spacing: 0.02em;
  white-space: nowrap;
}

.link-btn:hover:not(:disabled) {
  background: rgba(var(--color-accent-rgb), 0.28);
  border-color: rgba(var(--color-accent-rgb), 0.55);
  color: var(--color-ink-1);
  box-shadow: 0 0 18px rgba(var(--color-accent-rgb), 0.2);
}

.link-btn:disabled {
  opacity: 0.7;
  cursor: default;
}

.spinner {
  display: block;
  width: 12px;
  height: 12px;
  flex-shrink: 0;
  border-radius: 50%;
  border: 2px solid rgba(var(--color-accent-rgb), 0.25);
  border-top-color: var(--color-accent-light);
  animation: spin 0.7s linear infinite;
}

.error-text {
  margin: 10px 0 0;
  font-size: 11.5px;
  color: var(--color-bad);
}

.consent-note {
  margin: 14px 0 0;
  font-size: 10.5px;
  line-height: 1.5;
  color: var(--modal-ink-faint);
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
