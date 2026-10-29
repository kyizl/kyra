<script setup lang="ts">
import UsageDisclosure from '@renderer/components/UsageDisclosure.vue';
import { useConfigStore } from '@renderer/store/config';
import { BarChart3 } from 'lucide-vue-next';
import { onMounted, ref } from 'vue';

defineProps<{
  acknowledging: boolean;
}>();

const emit = defineEmits<{ acknowledge: [] }>();

const config = useConfigStore();
const visible = ref(false);

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
            <BarChart3 :size="20" />
          </div>

          <div class="header">
            <h2 class="title">Usage statistics</h2>
            <p class="subtitle">Kyra Overlay reports basic usage while it is running.</p>
          </div>

          <UsageDisclosure />

          <button
            class="ack-btn"
            :disabled="acknowledging"
            @click="emit('acknowledge')"
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
  margin-bottom: 16px;
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

.ack-btn {
  width: 100%;
  padding: 10px 14px;
  border-radius: 10px;
  font-size: 12.5px;
  font-weight: 600;
  letter-spacing: 0.02em;
  color: var(--color-accent-light);
  background: rgba(var(--color-accent-rgb), 0.16);
  border: 1px solid rgba(var(--color-accent-rgb), 0.35);
  cursor: pointer;
  transition:
    background 0.14s,
    border-color 0.14s,
    color 0.14s;
}

.ack-btn:hover:not(:disabled) {
  background: rgba(var(--color-accent-rgb), 0.28);
  border-color: rgba(var(--color-accent-rgb), 0.55);
  color: var(--color-ink-1);
}

.ack-btn:disabled {
  opacity: 0.7;
  cursor: default;
}
</style>
